use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use basil_bytecode::{BasicObject, ObjectDescriptor, Value, ElemType, ArrayObj};
use basil_common::{BasilError, Result};

#[cfg(feature = "once_cell")]
use once_cell::sync::Lazy;
#[cfg(feature = "once_cell")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "sha1")]
use sha1::{Digest as Sha1Digest, Sha1};
#[cfg(feature = "sha2")]
use sha2::Sha256;

use serde_json::json;

// --- Test mode flag controlled by VM ---
#[cfg(feature = "once_cell")]
pub static TEST_MODE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[inline]
fn is_test_mode_env() -> bool {
    std::env::var("TEST_MODE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

pub fn set_test_mode(on: bool) {
    #[cfg(feature = "once_cell")]
    {
        TEST_MODE.store(on, Ordering::Relaxed);
    }
}

fn is_test_mode() -> bool {
    let env_on = is_test_mode_env();
    #[cfg(feature = "once_cell")]
    {
        return env_on || TEST_MODE.load(Ordering::Relaxed);
    }
    #[allow(unreachable_code)]
    { env_on }
}

// Register an object type named "AI" so users can also do: DIM a@ AS AI() if they want an instance
pub fn register(reg: &mut crate::Registry) {
    reg.register("AI", crate::TypeInfo {
        factory: |_args| Ok(Rc::new(RefCell::new(AiObject::default()))),
        descriptor: descriptor_static,
        constants: || vec![("AI_VERSION$".to_string(), Value::Str("0.1".to_string()))],
    });
}

pub fn new_ai() -> Rc<RefCell<dyn BasicObject>> {
    Rc::new(RefCell::new(AiObject::default()))
}

#[derive(Default, Clone)]
struct AiObject {
    last_error: String,
}

impl AiObject {
    fn clear_err(&mut self) { self.last_error.clear(); }
    fn set_err<T: Into<String>>(&mut self, msg: T) -> BasilError {
        let m = msg.into();
        self.last_error = m.clone();
        BasilError(m)
    }
}

impl BasicObject for AiObject {
    fn type_name(&self) -> &str { "AI" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "LAST_ERROR$" => Ok(Value::Str(self.last_error.clone())),
            _ => Err(BasilError("Unknown AI property".into())),
        }
    }

    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> {
        Err(BasilError("AI has no settable properties".into()))
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "CHAT$" => {
                self.clear_err();
                if !(args.len() == 1 || args.len() == 2) { return Err(self.set_err("AI.CHAT$ expects 1 or 2 arguments")); }
                let prompt = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                let opts = if args.len() == 2 { match &args[1] { Value::Str(s)=>Some(s.as_str()), _=>None } } else { None };
                // Caching key
                let key = cache_key("chat", &prompt, opts.unwrap_or(""));
                if let Some(hit) = cache_get(&key) { return Ok(Value::Str(hit)); }
                if is_test_mode() {
                    let out = test_chat_text(&prompt);
                    cache_put(&key, &out);
                    return Ok(Value::Str(out));
                }
                // Online path
                match resolve_api_key() {
                    Some(api_key) => {
                        match chat_complete(&prompt, opts, &api_key) {
                            Ok(out) => { cache_put(&key, &out); Ok(Value::Str(out)) }
                            Err(e) => { self.set_err(e); Ok(Value::Str(String::new())) }
                        }
                    }
                    None => { self.set_err("missing API key"); Ok(Value::Str(String::new())) }
                }
            }
            "STREAM" => {
                self.clear_err();
                if !(args.len() == 1 || args.len() == 2) { return Err(self.set_err("AI.STREAM expects 1 or 2 arguments")); }
                let prompt = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                let opts = if args.len() == 2 { match &args[1] { Value::Str(s)=>Some(s.as_str()), _=>None } } else { None };
                let key = cache_key("stream", &prompt, opts.unwrap_or(""));
                if let Some(hit) = cache_get(&key) {
                    // print instantly from cache
                    print!("{}", hit);
                    let _ = std::io::stdout().flush();
                    return Ok(Value::Str(hit));
                }
                if is_test_mode() {
                    let full = test_chat_text(&prompt);
                    // emit in 3 chunks
                    let n = full.len();
                    let c1 = n / 3; let c2 = (2*n) / 3;
                    let parts = [&full[..c1], &full[c1..c2], &full[c2..]];
                    for p in parts { print!("{}", p); let _ = std::io::stdout().flush(); std::thread::sleep(std::time::Duration::from_millis(5)); }
                    cache_put(&key, &full);
                    return Ok(Value::Str(full));
                }
                match resolve_api_key() {
                    Some(api_key) => {
                        match chat_complete(&prompt, opts, &api_key) {
                            Ok(full) => {
                                // simulate streaming by chunking
                                let n = full.len();
                                let c1 = n / 3; let c2 = (2*n) / 3;
                                let parts = [&full[..c1], &full[c1..c2], &full[c2..]];
                                for p in parts { print!("{}", p); let _ = std::io::stdout().flush(); std::thread::sleep(std::time::Duration::from_millis(5)); }
                                cache_put(&key, &full);
                                Ok(Value::Str(full))
                            }
                            Err(e) => { self.set_err(e); Ok(Value::Str(String::new())) }
                        }
                    }
                    None => { self.set_err("missing API key"); Ok(Value::Str(String::new())) }
                }
            }
            "EMBED" => {
                self.clear_err();
                if !(args.len() == 1 || args.len() == 2) { return Err(self.set_err("AI.EMBED expects 1 or 2 arguments")); }
                let text = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                if is_test_mode() {
                    let vec = test_embed16(&text);
                    let dims = vec![vec.len()];
                    let data: Vec<Value> = vec.iter().copied().map(Value::Num).collect();
                    let arr = ArrayObj { elem: ElemType::Num, dims, data: RefCell::new(data) };
                    return Ok(Value::Array(Rc::new(arr)));
                }
                match resolve_api_key() {
                    Some(api_key) => {
                        match embed_request(&text, &api_key) {
                            Ok(vec) => {
                                let dims = vec![vec.len()];
                                let data: Vec<Value> = vec.iter().copied().map(Value::Num).collect();
                                let arr = ArrayObj { elem: ElemType::Num, dims, data: RefCell::new(data) };
                                Ok(Value::Array(Rc::new(arr)))
                            }
                            Err(e) => { self.set_err(e); let dims = vec![0usize]; let arr = ArrayObj { elem: ElemType::Num, dims, data: RefCell::new(Vec::new()) }; Ok(Value::Array(Rc::new(arr))) }
                        }
                    }
                    None => { self.set_err("missing API key"); let dims = vec![0usize]; let arr = ArrayObj { elem: ElemType::Num, dims, data: RefCell::new(Vec::new()) }; Ok(Value::Array(Rc::new(arr))) }
                }
            }
            "MODERATE%" => {
                self.clear_err();
                if !(args.len() == 1 || args.len() == 2) { return Err(self.set_err("AI.MODERATE% expects 1 or 2 arguments")); }
                let text = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                if is_test_mode() {
                    let flagged = text.contains("FLAG_ME");
                    return Ok(Value::Int(if flagged {1} else {0}));
                }
                // Non-test: stub allow
                Ok(Value::Int(0))
            }
            "KNOWLEDGE$" => {
                self.clear_err();
                if !(args.len() == 1 || args.len() == 2) { return Err(self.set_err("AI.KNOWLEDGE$ expects 1 or 2 arguments")); }
                let path = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                match fs::read_to_string(&path) {
                    Ok(s) => Ok(Value::Str(s)),
                    Err(e) => { self.set_err(format!("read failed: {}", e)); Ok(Value::Str(String::new())) }
                }
            }
            other => Err(BasilError(format!("Unknown method '{}' on AI", other))),
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    basil_bytecode::ObjectDescriptor {
        type_name: "AI".to_string(),
        version: "0.1".to_string(),
        summary: "AI helpers (chat/stream/embed/moderate)".to_string(),
        properties: vec![],
        methods: vec![
            basil_bytecode::MethodDesc { name: "CHAT$".to_string(), arity: 2, arg_names: vec!["prompt$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "STREAM".to_string(), arity: 2, arg_names: vec!["prompt$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "EMBED".to_string(), arity: 2, arg_names: vec!["text$".to_string(), "opts$".to_string()], return_type: "Float[]".to_string() },
            basil_bytecode::MethodDesc { name: "MODERATE%".to_string(), arity: 2, arg_names: vec!["text$".to_string(), "opts$".to_string()], return_type: "Int".to_string() },
            basil_bytecode::MethodDesc { name: "KNOWLEDGE$".to_string(), arity: 2, arg_names: vec!["path$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
        ],
        examples: vec![
            "PRINT AI.CHAT$(\"Hello\")".to_string(),
        ],
    }
}

fn test_chat_text(prompt: &str) -> String {
    #[cfg(feature = "sha1")]
    {
        let mut hasher = Sha1::new();
        hasher.update(prompt.as_bytes());
        let digest = hasher.finalize();
        let hex = hex_of(&digest[..]);
        return format!("[[TEST]] {}", &hex[..8]);
    }
    #[allow(unreachable_code)]
    { format!("[[TEST]] {}", prompt.len()) }
}

fn test_embed16(text: &str) -> Vec<f64> {
    // Use SHA1 bytes to derive 16 pseudo floats in [-1,1]
    let mut out = Vec::with_capacity(16);
    #[cfg(feature = "sha1")]
    {
        let mut hasher = Sha1::new();
        hasher.update(text.as_bytes());
        let d = hasher.finalize();
        for i in 0..16 { let b = d[i % d.len()] as i32; let v = (b - 128) as f64 / 128.0; out.push(v); }
        return out;
    }
    #[allow(unreachable_code)]
    {
        for i in 0..16 { out.push(((i as i32 - 8) as f64)/8.0); }
        out
    }
}

fn hex_of(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>() }

fn cache_dir() -> PathBuf {
    let mut dir = PathBuf::from(".basil");
    dir.push("ai-cache");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn cache_key(kind: &str, input: &str, opts: &str) -> String {
    #[cfg(feature = "sha2")]
    {
        let mut h = Sha256::new();
        h.update(kind.as_bytes());
        h.update(0u8.to_le_bytes());
        h.update(input.as_bytes());
        h.update(0u8.to_le_bytes());
        h.update(opts.as_bytes());
        let d = h.finalize();
        return hex_of(&d);
    }
    #[allow(unreachable_code)]
    { format!("{}:{}:{}", kind, input.len(), opts.len()) }
}

fn cache_path_for(key: &str) -> PathBuf { let mut p = cache_dir(); p.push(format!("{}.txt", key)); p }

fn cache_get(key: &str) -> Option<String> {
    let p = cache_path_for(key);
    if let Ok(s) = fs::read_to_string(p) { Some(s) } else { None }
}

fn cache_put(key: &str, val: &str) {
    let p = cache_path_for(key);
    if let Ok(mut f) = fs::File::create(p) { let _ = f.write_all(val.as_bytes()); let _ = f.sync_all(); }
}


// --- Online provider helpers (OpenAI minimal) ---
fn resolve_api_key() -> Option<String> {
    match std::env::var("OPENAI_API_KEY") {
        Ok(v) => { let t = v.trim().to_string(); if t.is_empty() { None } else { Some(t) } },
        Err(_) => None,
    }
}

fn openai_post_json(path: &str, body: serde_json::Value, api_key: &str, timeout_ms: u64) -> std::result::Result<serde_json::Value, String> {
    let url = format!("https://api.openai.com{}", path);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build();
    let resp = agent.post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string());
    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_else(|_| "".to_string());
            let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("invalid json: {}", e))?;
            if status >= 200 && status < 300 {
                Ok(val)
            } else {
                // try to extract message
                let msg = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("");
                if msg.is_empty() { Err(format!("http {}", status)) } else { Err(format!("http {}: {}", status, msg)) }
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            // HTTP status error (e.g., 401/429/5xx). Read body for message if present.
            let text = resp.into_string().unwrap_or_else(|_| "".to_string());
            let val: std::result::Result<serde_json::Value, _> = serde_json::from_str(&text);
            if let Ok(v) = val {
                let msg = v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("");
                if msg.is_empty() { Err(format!("http {}", code)) } else { Err(format!("http {}: {}", code, msg)) }
            } else {
                Err(format!("http {}", code))
            }
        }
        Err(ureq::Error::Transport(t)) => Err(format!("network error: {}", t)),
    }
}

fn chat_complete(prompt: &str, _opts: Option<&str>, api_key: &str) -> std::result::Result<String, String> {
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 400
    });
    let v = openai_post_json("/v1/chat/completions", body, api_key, 60000)?;
    let txt = v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    Ok(txt)
}

fn embed_request(text: &str, api_key: &str) -> std::result::Result<Vec<f64>, String> {
    let body = json!({
        "model": "text-embedding-3-small",
        "input": text
    });
    let v = openai_post_json("/v1/embeddings", body, api_key, 60000)?;
    let arr = v.get("data")
        .and_then(|d| d.get(0))
        .and_then(|o| o.get("embedding"))
        .and_then(|e| e.as_array())
        .ok_or_else(|| "missing embedding".to_string())?;
    let mut out = Vec::with_capacity(arr.len());
    for n in arr {
        if let Some(f) = n.as_f64() { out.push(f); }
        else if let Some(i) = n.as_i64() { out.push(i as f64); }
        else if let Some(u) = n.as_u64() { out.push(u as f64); }
    }
    Ok(out)
}
