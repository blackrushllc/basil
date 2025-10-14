use std::cell::RefCell;
use std::rc::Rc;

use basil_bytecode::{BasicObject, ObjectDescriptor, PropDesc, MethodDesc, Value, ObjectRef};
use basil_common::{Result, BasilError};

use aws_config::meta::region::RegionProviderChain;
use aws_types::region::Region;

use crate::runtime::RT;

#[derive(Clone, Default)]
pub struct AwsContext {
    profile: Option<String>,
    region: Option<String>,
    max_retries: Option<i32>,
    timeout_ms: Option<i32>,
}

impl AwsContext {
    pub fn with_defaults() -> Self { Self::default() }

    fn load_config(&self) -> Result<aws_config::SdkConfig> {
        let mut loader = aws_config::from_env();
        if let Some(profile) = &self.profile {
            loader = loader.profile_name(profile);
        }
        if let Some(region) = &self.region {
            loader = loader.region(Region::new(region.clone()));
        } else {
            let rp = RegionProviderChain::default_provider().or_default_provider().or_else("us-east-1");
            loader = loader.region(rp);
        }
        // Retry and timeouts: use default SDK policy; expose knobs later.
        let cfg = RT.block_on(async move { loader.load().await });
        Ok(cfg)
    }

    pub(crate) fn make_s3(&self) -> Result<Value> {
        #[cfg(feature = "obj-aws-s3")]
        {
            let cfg = self.load_config()?;
            let client = aws_sdk_s3::Client::new(&cfg);
            let obj = crate::s3::S3Object::from_client(client);
            Ok(Value::Object(Rc::new(RefCell::new(obj))))
        }
        #[cfg(not(feature = "obj-aws-s3"))]
        { Err(BasilError("AWS_S3 not enabled; rebuild with feature obj-aws-s3".into())) }
    }

    pub(crate) fn make_ses(&self) -> Result<Value> {
        #[cfg(feature = "obj-aws-ses")]
        {
            let cfg = self.load_config()?;
            let client = aws_sdk_sesv2::Client::new(&cfg);
            let obj = crate::ses::SesObject::from_client(client);
            Ok(Value::Object(Rc::new(RefCell::new(obj))))
        }
        #[cfg(not(feature = "obj-aws-ses"))]
        { Err(BasilError("AWS_SES not enabled; rebuild with feature obj-aws-ses".into())) }
    }

    pub(crate) fn make_sqs(&self) -> Result<Value> {
        #[cfg(feature = "obj-aws-sqs")]
        {
            let cfg = self.load_config()?;
            let client = aws_sdk_sqs::Client::new(&cfg);
            let obj = crate::sqs::SqsObject::from_client(client);
            Ok(Value::Object(Rc::new(RefCell::new(obj))))
        }
        #[cfg(not(feature = "obj-aws-sqs"))]
        { Err(BasilError("AWS_SQS not enabled; rebuild with feature obj-aws-sqs".into())) }
    }
}

impl BasicObject for AwsContext {
    fn type_name(&self) -> &str { "AWS" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "PROFILE$" => Ok(Value::Str(self.profile.clone().unwrap_or_default())),
            "REGION$" => Ok(Value::Str(self.region.clone().unwrap_or_default())),
            "MAXRETRIES%" => Ok(Value::Int(self.max_retries.unwrap_or_default() as i64)),
            "TIMEOUTMS%" => Ok(Value::Int(self.timeout_ms.unwrap_or_default() as i64)),
            other => Err(BasilError(format!("Unknown AWS property '{}'", other))),
        }
    }

    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "PROFILE$" => { self.profile = match v { Value::Str(s)=> if s.is_empty(){None}else{Some(s)}, _=>None }; Ok(()) }
            ,"REGION$" => { self.region = match v { Value::Str(s)=> if s.is_empty(){None}else{Some(s)}, _=>None }; Ok(()) }
            ,"MAXRETRIES%" => { self.max_retries = match v { Value::Int(i)=>Some(i as i32), Value::Num(n)=>Some(n.trunc() as i32), _=>None }; Ok(()) }
            ,"TIMEOUTMS%" => { self.timeout_ms = match v { Value::Int(i)=>Some(i as i32), Value::Num(n)=>Some(n.trunc() as i32), _=>None }; Ok(()) }
            ,other => Err(BasilError(format!("Unknown AWS property '{}'", other)))
        }
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "MAKES3" => { if !args.is_empty() { return Err(BasilError("AWS.MakeS3 expects 0 args".into())); } self.make_s3() }
            ,"MAKESES" => { if !args.is_empty() { return Err(BasilError("AWS.MakeSES expects 0 args".into())); } self.make_ses() }
            ,"MAKESQS" => { if !args.is_empty() { return Err(BasilError("AWS.MakeSQS expects 0 args".into())); } self.make_sqs() }
            ,"ASSUMEROLE$" => {
                if !(args.len() == 2 || args.len() == 3) { return Err(BasilError("AWS.AssumeRole$ expects role_arn$, session_name$, duration_sec%?".into())); }
                let _role_arn = match &args[0] { Value::Str(s)=>s.clone(), _=>return Err(BasilError("role_arn$ must be string".into())) };
                let _session = match &args[1] { Value::Str(s)=>s.clone(), _=>return Err(BasilError("session_name$ must be string".into())) };
                let _duration = if args.len()==3 { match &args[2] { Value::Int(i)=>Some(*i as i32), Value::Num(n)=>Some(n.trunc() as i32), _=>None } } else { None };
                // Optional: Not implemented in Phase 1 (documented). Return error stub for now.
                Err(BasilError("AWS.AssumeRole$ not implemented in Phase 1".into()))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on AWS", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "AWS".to_string(),
        version: "0.1".to_string(),
        summary: "AWS context: config and client factory".to_string(),
        properties: vec![
            PropDesc { name: "Profile$".into(), type_name: "String".into(), readable: true, writable: true },
            PropDesc { name: "Region$".into(), type_name: "String".into(), readable: true, writable: true },
            PropDesc { name: "MaxRetries%".into(), type_name: "Int".into(), readable: true, writable: true },
            PropDesc { name: "TimeoutMs%".into(), type_name: "Int".into(), readable: true, writable: true },
        ],
        methods: vec![
            MethodDesc { name: "MakeS3".into(), arity: 0, arg_names: vec![], return_type: "AWS_S3".into() },
            MethodDesc { name: "MakeSES".into(), arity: 0, arg_names: vec![], return_type: "AWS_SES".into() },
            MethodDesc { name: "MakeSQS".into(), arity: 0, arg_names: vec![], return_type: "AWS_SQS".into() },
            MethodDesc { name: "AssumeRole$".into(), arity: 3, arg_names: vec!["role_arn$".into(), "session_name$".into(), "duration_sec%?".into()], return_type: "String (JSON)".into() },
        ],
        examples: vec![
            "DIM aws@ AS AWS()".into(),
        ],
    }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let factory = |_args: &[Value]| -> Result<ObjectRef> {
        Ok(Rc::new(RefCell::new(AwsContext::with_defaults())))
    };
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    reg("AWS", crate::TypeInfo { factory, descriptor, constants });
}
