use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use basil_common::{BasilError, Result};
use basil_bytecode::{BasicObject, MethodDesc, ObjectDescriptor, PropDesc, Value};

use crate::runtime::TOKIO_RT;

#[derive(Clone)]
struct HttpObj {
    base_url: String,
    timeout_ms: u64,
    raise_for_status: bool,
    default_headers: reqwest::header::HeaderMap,
    default_query: Vec<(String, String)>,
    auth_bearer: Option<String>,
    auth_basic: Option<(String, String)>,

    client: Option<reqwest::Client>,

    last_status: i32,
    last_url: String,
    last_headers: String,
    last_error: String,
}

fn err(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("HTTP.{}: {}", op, e)) }

impl HttpObj {
    fn new() -> Self {
        Self {
            base_url: String::new(),
            timeout_ms: 30_000,
            raise_for_status: false,
            default_headers: reqwest::header::HeaderMap::new(),
            default_query: Vec::new(),
            auth_bearer: None,
            auth_basic: None,
            client: None,
            last_status: 0,
            last_url: String::new(),
            last_headers: String::new(),
            last_error: String::new(),
        }
    }

    fn rebuild_client(&mut self) -> Result<()> {
        let mut builder = reqwest::Client::builder()
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .redirect(reqwest::redirect::Policy::default())
            .timeout(Duration::from_millis(self.timeout_ms));

        // Default headers
        let mut hdrs = self.default_headers.clone();
        // Apply auth preference (Bearer > Basic) if not already set by SetHeader
        if !hdrs.contains_key(reqwest::header::AUTHORIZATION) {
            if let Some(tok) = &self.auth_bearer {
                if !tok.is_empty() {
                    let v = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", tok)).map_err(|e| err("Auth(Bearer)", e))?;
                    hdrs.insert(reqwest::header::AUTHORIZATION, v);
                }
            } else if let Some((u,p)) = &self.auth_basic {
                let cred = base64::encode(format!("{}:{}", u, p));
                let v = reqwest::header::HeaderValue::from_str(&format!("Basic {}", cred)).map_err(|e| err("Auth(Basic)", e))?;
                hdrs.insert(reqwest::header::AUTHORIZATION, v);
            }
        }
        if !hdrs.is_empty() { builder = builder.default_headers(hdrs); }

        self.client = Some(builder.build().map_err(|e| err("BuildClient", e))?);
        Ok(())
    }

    fn ensure_client(&mut self) -> Result<()> {
        if self.client.is_none() { self.rebuild_client()?; }
        Ok(())
    }

    fn resolve_url(&self, url: &str) -> String {
        let u = url.trim();
        if u.starts_with("http://") || u.starts_with("https://") {
            u.to_string()
        } else if self.base_url.is_empty() {
            u.to_string()
        } else {
            // naive join
            let mut b = self.base_url.clone();
            if b.ends_with('/') && u.starts_with('/') {
                b.pop();
            } else if !b.ends_with('/') && !u.starts_with('/') {
                b.push('/');
            }
            b.push_str(u);
            b
        }
    }

    fn headers_to_json(headers: &reqwest::header::HeaderMap) -> String {
        let mut map: HashMap<String, String> = HashMap::new();
        for (k, v) in headers.iter() {
            if let Ok(s) = v.to_str() { map.insert(k.to_string(), s.to_string()); }
        }
        serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
    }

    fn update_last_from_response(&mut self, resp: &reqwest::Response) {
        self.last_status = resp.status().as_u16() as i32;
        self.last_url = resp.url().to_string();
        self.last_headers = Self::headers_to_json(resp.headers());
        self.last_error.clear();
    }

    fn set_error(&mut self, url: &str, msg: String) {
        self.last_status = 0;
        self.last_url = url.to_string();
        self.last_headers.clear();
        self.last_error = msg;
    }

    fn parse_timeout_override(args: &[Value], base_arity: usize) -> Option<Duration> {
        if args.len() == base_arity + 1 {
            let ms = match &args[base_arity] { Value::Int(i)=>*i, Value::Num(n)=> n.trunc() as i64, _=> -1 };
            if ms > 0 { return Some(Duration::from_millis(ms as u64)); }
        }
        None
    }

    fn do_request_body(&mut self, method: reqwest::Method, url: String, body: Option<String>, json_ct: bool, timeout: Option<Duration>) -> Result<String> {
        self.ensure_client()?;
        let client = self.client.as_ref().unwrap().clone();
        let q = self.default_query.clone();
        let url_clone = url.clone();
        let rb_build = move || {
            let mut rb = client.request(method.clone(), &url_clone).query(&q);
            if let Some(d) = timeout { rb = rb.timeout(d); }
            if let Some(b) = &body {
                if json_ct {
                    rb = rb.header(reqwest::header::CONTENT_TYPE, "application/json");
                }
                rb = rb.body(b.clone());
            }
            rb
        };
        let resp_result = TOKIO_RT.block_on(async move { rb_build().send().await });
        match resp_result {
            Ok(resp) => {
                let final_url = resp.url().to_string();
                self.update_last_from_response(&resp);
                let status = resp.status();
                let bytes_res = TOKIO_RT.block_on(async move { resp.bytes().await });
                match bytes_res {
                    Ok(bytes) => {
                        let body_str = String::from_utf8_lossy(&bytes).to_string();
                        if self.raise_for_status && (status.is_client_error() || status.is_server_error()) {
                            let reason = status.canonical_reason().unwrap_or("");
                            let mut snippet = body_str.clone();
                            if snippet.len() > 512 { snippet.truncate(512); }
                            let msg = format!("HTTP {} {} at {} — {}", status.as_u16(), reason, final_url, snippet);
                            self.last_error = msg.clone();
                            return Err(BasilError(msg));
                        }
                        Ok(body_str)
                    }
                    Err(e) => {
                        let msg = format!("ReadBody failed: {}", e);
                        self.last_error = msg.clone();
                        Err(err("ReadBody", e))
                    }
                }
            }
            Err(e) => {
                let msg = format!("RequestFailed: {} at {}", e, url);
                self.set_error(&url, msg.clone());
                Err(BasilError(format!("HTTP {}", msg)))
            }
        }
    }

    fn do_request_head(&mut self, url: String, timeout: Option<Duration>) -> Result<i64> {
        self.ensure_client()?;
        let client = self.client.as_ref().unwrap().clone();
        let q = self.default_query.clone();
        let url_clone = url.clone();
        let resp_result = TOKIO_RT.block_on(async move {
            let mut rb = client.request(reqwest::Method::HEAD, &url_clone).query(&q);
            if let Some(d) = timeout { rb = rb.timeout(d); }
            rb.send().await
        });
        match resp_result {
            Ok(resp) => {
                self.update_last_from_response(&resp);
                let status = resp.status();
                if self.raise_for_status && (status.is_client_error() || status.is_server_error()) {
                    let reason = status.canonical_reason().unwrap_or("");
                    let msg = format!("HTTP {} {} at {}", status.as_u16(), reason, self.last_url);
                    self.last_error = msg.clone();
                    return Err(BasilError(msg));
                }
                Ok(1)
            }
            Err(e) => {
                let msg = format!("RequestFailed: {} at {}", e, url);
                self.set_error(&url, msg.clone());
                Err(BasilError(format!("HTTP {}", msg)))
            }
        }
    }

    fn download_to_file(&mut self, url: String, out_path: String, timeout: Option<Duration>) -> Result<i64> {
        self.ensure_client()?;
        let client = self.client.as_ref().unwrap().clone();
        let q = self.default_query.clone();
        let url_clone = url.clone();
        let resp_result = TOKIO_RT.block_on(async move {
            let mut rb = client.get(&url_clone).query(&q);
            if let Some(d) = timeout { rb = rb.timeout(d); }
            rb.send().await
        });
        match resp_result {
            Ok(resp) => {
                self.update_last_from_response(&resp);
                let status = resp.status();
                let final_url = self.last_url.clone();
                let bytes_res = TOKIO_RT.block_on(async move { resp.bytes().await });
                match bytes_res {
                    Ok(bytes) => {
                        if self.raise_for_status && (status.is_client_error() || status.is_server_error()) {
                            let reason = status.canonical_reason().unwrap_or("");
                            let mut snippet = String::from_utf8_lossy(&bytes).to_string();
                            if snippet.len() > 512 { snippet.truncate(512); }
                            let msg = format!("HTTP {} {} at {} — {}", status.as_u16(), reason, final_url, snippet);
                            self.last_error = msg.clone();
                            return Err(BasilError(msg));
                        }
                        // Ensure directories
                        if let Some(parent) = Path::new(&out_path).parent() { if !parent.as_os_str().is_empty() { let _ = fs::create_dir_all(parent); } }
                        let mut f = fs::File::create(&out_path).map_err(|e| err("CreateFile", e))?;
                        f.write_all(&bytes).map_err(|e| err("WriteFile", e))?;
                        Ok(1)
                    }
                    Err(e) => {
                        let msg = format!("ReadBody failed: {}", e);
                        self.last_error = msg.clone();
                        Err(err("ReadBody", e))
                    }
                }
            }
            Err(e) => {
                let msg = format!("RequestFailed: {} at {}", e, url);
                self.set_error(&url, msg.clone());
                Err(BasilError(format!("HTTP {}", msg)))
            }
        }
    }

    fn upload_file(&mut self, url: String, file_path: String, field_name: String, content_type: String, timeout: Option<Duration>) -> Result<String> {
        self.ensure_client()?;
        let client = self.client.as_ref().unwrap().clone();
        let q = self.default_query.clone();
        let field = if field_name.is_empty() { "file".to_string() } else { field_name };

        // Build form (read file bytes synchronously to avoid async Part::file)
        let bytes = fs::read(&file_path).map_err(|e| err("OpenFile", e))?;
        let fname = Path::new(&file_path).file_name().and_then(|s| s.to_str()).unwrap_or("file").to_string();
        let mut part = reqwest::multipart::Part::bytes(bytes).file_name(fname);
        if !content_type.is_empty() {
            part = part.mime_str(&content_type).map_err(|e| err("ContentType", e))?;
        } else if let Some(ct) = guess_mime(&file_path) {
            part = part.mime_str(ct).map_err(|e| err("ContentType", e))?;
        }
        let form = reqwest::multipart::Form::new().part(field, part);

        let url_clone = url.clone();
        let resp_result = TOKIO_RT.block_on(async move {
            let mut rb = client.post(&url_clone).query(&q).multipart(form);
            if let Some(d) = timeout { rb = rb.timeout(d); }
            rb.send().await
        });
        match resp_result {
            Ok(resp) => {
                self.update_last_from_response(&resp);
                let status = resp.status();
                let final_url = self.last_url.clone();
                let bytes_res = TOKIO_RT.block_on(async move { resp.bytes().await });
                match bytes_res {
                    Ok(bytes) => {
                        let body_str = String::from_utf8_lossy(&bytes).to_string();
                        if self.raise_for_status && (status.is_client_error() || status.is_server_error()) {
                            let reason = status.canonical_reason().unwrap_or("");
                            let mut snippet = body_str.clone();
                            if snippet.len() > 512 { snippet.truncate(512); }
                            let msg = format!("HTTP {} {} at {} — {}", status.as_u16(), reason, final_url, snippet);
                            self.last_error = msg.clone();
                            return Err(BasilError(msg));
                        }
                        Ok(body_str)
                    }
                    Err(e) => {
                        let msg = format!("ReadBody failed: {}", e);
                        self.last_error = msg.clone();
                        Err(err("ReadBody", e))
                    }
                }
            }
            Err(e) => {
                let msg = format!("RequestFailed: {} at {}", e, url);
                self.set_error(&url, msg.clone());
                Err(BasilError(format!("HTTP {}", msg)))
            }
        }
    }
}

fn guess_mime(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg"|"jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "webp" => Some("image/webp"),
        "json" => Some("application/json"),
        "txt" => Some("text/plain; charset=utf-8"),
        "csv" => Some("text/csv; charset=utf-8"),
        "xml" => Some("application/xml"),
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

impl BasicObject for HttpObj {
    fn type_name(&self) -> &str { "HTTP" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "BASEURL$" => Ok(Value::Str(self.base_url.clone())),
            "TIMEOUTMS%" => Ok(Value::Int(self.timeout_ms as i64)),
            "RAISEFORSTATUS%" => Ok(Value::Int(if self.raise_for_status {1} else {0})),
            "LASTSTATUS%" => Ok(Value::Int(self.last_status as i64)),
            "LASTURL$" => Ok(Value::Str(self.last_url.clone())),
            "LASTHEADERS$" => Ok(Value::Str(self.last_headers.clone())),
            "LASTERROR$" => Ok(Value::Str(self.last_error.clone())),
            other => Err(BasilError(format!("Unknown property '{}' on HTTP", other))),
        }
    }

    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "BASEURL$" => { self.base_url = match v { Value::Str(s)=>s, other=>format!("{}", other) }; Ok(()) }
            ,"TIMEOUTMS%" => {
                self.timeout_ms = match v { Value::Int(i)=> if i<0 {0} else {i as u64}, Value::Num(n)=> if n<0.0 {0} else {n.trunc() as u64}, _=> self.timeout_ms };
                self.rebuild_client()?;
                Ok(())
            }
            ,"RAISEFORSTATUS%" => { self.raise_for_status = match v { Value::Int(i)=> i!=0, Value::Num(n)=> n!=0.0, Value::Bool(b)=>b, _=> false }; Ok(()) }
            ,other => Err(BasilError(format!("Unknown or read-only property '{}' on HTTP", other)))
        }
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            // Header & auth helpers
            "SETHEADER" => {
                if args.len()!=2 { return Err(BasilError("HTTP.SetHeader expects name$, value$".into())); }
                let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let value = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let hname = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|e| err("HeaderName", e))?;
                let hval = reqwest::header::HeaderValue::from_str(&value).map_err(|e| err("HeaderValue", e))?;
                self.default_headers.insert(hname, hval);
                self.rebuild_client()?;
                Ok(Value::Int(1))
            }
            ,"CLEARHEADERS" => { self.default_headers.clear(); self.rebuild_client()?; Ok(Value::Int(1)) }
            ,"SETBEARER$" => {
                if args.len()!=1 { return Err(BasilError("HTTP.SetBearer$ expects token$".into())); }
                let token = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                self.auth_bearer = Some(token);
                self.auth_basic = None; // prefer bearer
                self.rebuild_client()?;
                Ok(Value::Int(1))
            }
            ,"SETBASICAUTH$" => {
                if args.len()!=2 { return Err(BasilError("HTTP.SetBasicAuth$ expects user$, pass$".into())); }
                let user = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let pass = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                self.auth_basic = Some((user, pass));
                self.auth_bearer = None;
                self.rebuild_client()?;
                Ok(Value::Int(1))
            }
            ,"SETQUERYPARAM" => {
                if args.len()!=2 { return Err(BasilError("HTTP.SetQueryParam expects name$, value$".into())); }
                let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let value = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                self.default_query.push((name, value));
                Ok(Value::Int(1))
            }
            ,"CLEARQUERYPARAMS" => { self.default_query.clear(); Ok(Value::Int(1)) }

            // Core request methods
            ,"GET$" => {
                if !(args.len()==1 || args.len()==2) { return Err(BasilError("HTTP.Get$ expects url$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let to = Self::parse_timeout_override(args, 1);
                let body = self.do_request_body(reqwest::Method::GET, url, None, false, to)?;
                Ok(Value::Str(body))
            }
            ,"DELETE$" => {
                if !(args.len()==1 || args.len()==2) { return Err(BasilError("HTTP.Delete$ expects url$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let to = Self::parse_timeout_override(args, 1);
                let body = self.do_request_body(reqwest::Method::DELETE, url, None, false, to)?;
                Ok(Value::Str(body))
            }
            ,"HEAD$"|"HEAD" => {
                if !(args.len()==1 || args.len()==2) { return Err(BasilError("HTTP.Head$ expects url$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let to = Self::parse_timeout_override(args, 1);
                let ok = self.do_request_head(url, to)?;
                Ok(Value::Int(ok))
            }
            ,"POST$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.Post$ expects url$, body$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::POST, url, Some(body), false, to)?;
                Ok(Value::Str(body))
            }
            ,"PUT$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.Put$ expects url$, body$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::PUT, url, Some(body), false, to)?;
                Ok(Value::Str(body))
            }
            ,"PATCH$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.Patch$ expects url$, body$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::PATCH, url, Some(body), false, to)?;
                Ok(Value::Str(body))
            }
            // JSON convenience
            ,"POSTJSON$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.PostJson$ expects url$, json$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::POST, url, Some(body), true, to)?;
                Ok(Value::Str(body))
            }
            ,"PUTJSON$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.PutJson$ expects url$, json$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::PUT, url, Some(body), true, to)?;
                Ok(Value::Str(body))
            }
            ,"PATCHJSON$" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.PatchJson$ expects url$, json$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let body = self.do_request_body(reqwest::Method::PATCH, url, Some(body), true, to)?;
                Ok(Value::Str(body))
            }

            // File I/O
            ,"DOWNLOADTOFILE" => {
                if !(args.len()==2 || args.len()==3) { return Err(BasilError("HTTP.DownloadToFile expects url$, out_path$, timeout_ms%?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let out_path = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = Self::parse_timeout_override(args, 2);
                let ok = self.download_to_file(url, out_path, to)?;
                Ok(Value::Int(ok))
            }
            ,"UPLOADFILE$" => {
                if !(args.len()==2 || args.len()==3 || args.len()==4) { return Err(BasilError("HTTP.UploadFile$ expects url$, file_path$, field_name$?, content_type$?".into())); }
                let input_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let url = self.resolve_url(&input_url);
                let file_path = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let field = if args.len()>=3 { match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) } } else { "file".to_string() };
                let ctype = if args.len()>=4 { match &args[3] { Value::Str(s)=>s.clone(), other=>format!("{}", other) } } else { String::new() };
                let body = self.upload_file(url, file_path, field, ctype, None)?;
                Ok(Value::Str(body))
            }

            ,other => Err(BasilError(format!("Unknown method '{}' on HTTP", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    let factory = |_args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        Ok(Rc::new(RefCell::new(HttpObj::new())))
    };
    reg("HTTP", crate::TypeInfo { factory, descriptor, constants });
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "HTTP".into(),
        version: "0.1".into(),
        summary: "General HTTP/REST client with defaults, headers, auth, file I/O".into(),
        properties: vec![
            PropDesc { name: "BaseUrl$".into(), type_name: "String".into(), readable: true, writable: true },
            PropDesc { name: "TimeoutMs%".into(), type_name: "Integer".into(), readable: true, writable: true },
            PropDesc { name: "RaiseForStatus%".into(), type_name: "Integer".into(), readable: true, writable: true },
            PropDesc { name: "LastStatus%".into(), type_name: "Integer".into(), readable: true, writable: false },
            PropDesc { name: "LastUrl$".into(), type_name: "String".into(), readable: true, writable: false },
            PropDesc { name: "LastHeaders$".into(), type_name: "String".into(), readable: true, writable: false },
            PropDesc { name: "LastError$".into(), type_name: "String".into(), readable: true, writable: false },
        ],
        methods: vec![
            MethodDesc { name: "SetHeader".into(), arity: 2, arg_names: vec!["name$".into(), "value$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "ClearHeaders".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },
            MethodDesc { name: "SetBearer$".into(), arity: 1, arg_names: vec!["token$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "SetBasicAuth$".into(), arity: 2, arg_names: vec!["user$".into(), "pass$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "SetQueryParam".into(), arity: 2, arg_names: vec!["name$".into(), "value$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "ClearQueryParams".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },

            MethodDesc { name: "Get$".into(), arity: 2, arg_names: vec!["url$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "Delete$".into(), arity: 2, arg_names: vec!["url$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "Head$".into(), arity: 2, arg_names: vec!["url$".into(), "timeout_ms%?".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Post$".into(), arity: 3, arg_names: vec!["url$".into(), "body$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "Put$".into(), arity: 3, arg_names: vec!["url$".into(), "body$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "Patch$".into(), arity: 3, arg_names: vec!["url$".into(), "body$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },

            MethodDesc { name: "PostJson$".into(), arity: 3, arg_names: vec!["url$".into(), "json$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "PutJson$".into(), arity: 3, arg_names: vec!["url$".into(), "json$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },
            MethodDesc { name: "PatchJson$".into(), arity: 3, arg_names: vec!["url$".into(), "json$".into(), "timeout_ms%?".into()], return_type: "String (body$)".into() },

            MethodDesc { name: "DownloadToFile".into(), arity: 3, arg_names: vec!["url$".into(), "out_path$".into(), "timeout_ms%?".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "UploadFile$".into(), arity: 4, arg_names: vec!["url$".into(), "file_path$".into(), "field_name$?".into(), "content_type$?".into()], return_type: "String (body$)".into() },
        ],
        examples: vec![
            "DIM http@ AS HTTP()".into(),
        ],
    }
}
