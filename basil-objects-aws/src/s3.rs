use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::rc::Rc;

use basil_bytecode::{ArrayObj, BasicObject, ElemType, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};

use crate::runtime::RT;

fn make_str_array(items: Vec<String>) -> Value {
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

fn err_msg(op: &str, e: impl std::fmt::Display) -> BasilError {
    BasilError(format!("S3.{}: {}", op, e))
}

pub struct S3Object {
    client: aws_sdk_s3::Client,
}

impl S3Object {
    pub fn from_client(client: aws_sdk_s3::Client) -> Self { Self { client } }
}

impl BasicObject for S3Object {
    fn type_name(&self) -> &str { "AWS_S3" }

    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("AWS_S3 has no readable properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("AWS_S3 has no settable properties".into())) }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "PUT$" => {
                if args.len() != 3 { return Err(BasilError("S3.Put$ expects 3 args: bucket$, key$, data$|file$".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let key = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let third = match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                // If third is a path to an existing file, upload from file; else treat as data.
                let etag = if std::path::Path::new(&third).exists() {
                    let mut f = File::open(&third).map_err(|e| err_msg("Put(open file)", e))?;
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf).map_err(|e| err_msg("Put(read file)", e))?;
                    let client = self.client.clone();
                    let bucket_c = bucket.clone(); let key_c = key.clone();
                    RT.block_on(async move {
                        let out = client.put_object().bucket(bucket_c).key(key_c).body(buf.into()).send().await;
                        match out { Ok(resp)=> Ok(resp.e_tag.unwrap_or_default()), Err(e)=> Err(err_msg("Put", e)) }
                    })?
                } else {
                    let bytes = third.into_bytes();
                    let client = self.client.clone();
                    let bucket_c = bucket.clone(); let key_c = key.clone();
                    RT.block_on(async move {
                        let out = client.put_object().bucket(bucket_c).key(key_c).body(bytes.into()).send().await;
                        match out { Ok(resp)=> Ok(resp.e_tag.unwrap_or_default()), Err(e)=> Err(err_msg("Put", e)) }
                    })?
                };
                Ok(Value::Str(etag))
            }
            ,"GET$" => {
                if args.len() != 2 { return Err(BasilError("S3.Get$ expects 2 args: bucket$, key$".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let key = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                let data = RT.block_on(async move {
                    let resp = client.get_object().bucket(bucket).key(key).send().await.map_err(|e| err_msg("Get", e))?;
                    let bytes = resp.body.collect().await.map_err(|e| err_msg("Get(read)", e))?.into_bytes();
                    Ok::<Vec<u8>, BasilError>(bytes.to_vec())
                })?;
                let s = String::from_utf8_lossy(&data).to_string();
                Ok(Value::Str(s))
            }
            ,"GETTOFILE" => {
                if args.len() != 3 { return Err(BasilError("S3.GetToFile expects 3 args: bucket$, key$, file$".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let key = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let file = match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                let data = RT.block_on(async move {
                    let resp = client.get_object().bucket(bucket).key(key).send().await.map_err(|e| err_msg("GetToFile", e))?;
                    let bytes = resp.body.collect().await.map_err(|e| err_msg("GetToFile(read)", e))?.into_bytes();
                    Ok::<Vec<u8>, BasilError>(bytes.to_vec())
                })?;
                let mut f = File::create(&file).map_err(|e| err_msg("GetToFile(create)", e))?;
                f.write_all(&data).map_err(|e| err_msg("GetToFile(write)", e))?;
                Ok(Value::Int(1))
            }
            ,"LIST$" => {
                if !(args.len() == 1 || args.len() == 2 || args.len() == 3) { return Err(BasilError("S3.List$ expects bucket$, prefix$?, max%?".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let prefix = if args.len()>=2 { match &args[1] { Value::Str(s)=> if s.is_empty(){None}else{Some(s.clone())}, _=>None } } else { None };
                let mut max = if args.len()==3 { match &args[2] { Value::Int(i)=> *i as i32, Value::Num(n)=> n.trunc() as i32, _=> 0 } } else { 0 };
                if max <= 0 { max = 100; }
                let client = self.client.clone();
                let keys: Vec<String> = RT.block_on(async move {
                    let mut out: Vec<String> = Vec::new();
                    let mut token: Option<String> = None;
                    loop {
                        let mut req = client.list_objects_v2().bucket(&bucket);
                        if let Some(p) = &prefix { req = req.prefix(p); }
                        if let Some(t) = &token { req = req.continuation_token(t); }
                        let resp = match req.send().await { Ok(r)=>r, Err(e)=>{ return Err(err_msg("List", e)); } };
                        if let Some(contents) = resp.contents {
                            for o in contents {
                                if let Some(k) = o.key { out.push(k); if out.len() as i32 >= max { return Ok(out); } }
                            }
                        }
                        let next = resp.next_continuation_token;
                        if resp.is_truncated.unwrap_or(false) && next.is_some() { token = next; } else { break; }
                    }
                    Ok(out)
                })?;
                Ok(make_str_array(keys))
            }
            ,"DELETE" => {
                if args.len() != 2 { return Err(BasilError("S3.Delete expects 2 args: bucket$, key$".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let key = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                RT.block_on(async move {
                    match client.delete_object().bucket(bucket).key(key).send().await {
                        Ok(_)=> Ok(Value::Int(1)),
                        Err(e)=> Err(err_msg("Delete", e)),
                    }
                })
            }
            ,"SIGNEDURL$" => {
                if args.len() != 3 { return Err(BasilError("S3.SignedUrl$ expects 3 args: bucket$, key$, expires_sec%".into())); }
                let bucket = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let key = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let expires = match &args[2] { Value::Int(i)=> *i as i64, Value::Num(n)=> n.trunc() as i64, _=> 300 } as u64;
                let client = self.client.clone();
                let url = RT.block_on(async move {
                    use aws_sdk_s3::presigning::PresigningConfig;
                    let pres = PresigningConfig::expires_in(std::time::Duration::from_secs(expires))
                        .map_err(|e| err_msg("SignedUrl(config)", e))?;
                    let req = client.get_object().bucket(bucket).key(key);
                    let presigned = req.presigned(pres).await.map_err(|e| err_msg("SignedUrl(presign)", e))?;
                    Ok::<String, BasilError>(presigned.uri().to_string())
                })?;
                Ok(Value::Str(url))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on AWS_S3", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "AWS_S3".into(),
        version: "0.1".into(),
        summary: "Amazon S3 client for Basil".into(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "Put$".into(), arity: 3, arg_names: vec!["bucket$".into(), "key$".into(), "data$|file$".into()], return_type: "String (ETag$)".into() },
            MethodDesc { name: "Get$".into(), arity: 2, arg_names: vec!["bucket$".into(), "key$".into()], return_type: "String (bytes$)".into() },
            MethodDesc { name: "GetToFile".into(), arity: 3, arg_names: vec!["bucket$".into(), "key$".into(), "file$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "List$".into(), arity: 3, arg_names: vec!["bucket$".into(), "prefix$?".into(), "max%?".into()], return_type: "String[]".into() },
            MethodDesc { name: "Delete".into(), arity: 2, arg_names: vec!["bucket$".into(), "key$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "SignedUrl$".into(), arity: 3, arg_names: vec!["bucket$".into(), "key$".into(), "expires_sec%".into()], return_type: "String (url$)".into() },
        ],
        examples: vec![
            "DIM aws@ AS AWS() : DIM s3@ = aws@.MakeS3()".into(),
        ],
    }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let factory = |_args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        // Independent constructor that discovers config
        let cfg = crate::context::AwsContext::default();
        let val = cfg.make_s3()?;
        match val { Value::Object(o)=> Ok(o), _=> Err(BasilError("unexpected".into())) }
    };
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    reg("AWS_S3", crate::TypeInfo { factory, descriptor, constants });
}
