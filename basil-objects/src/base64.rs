use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, MethodDesc, BasicObject};

#[cfg(feature = "base64")]
use base64::{engine::general_purpose, Engine as _};

pub fn register(reg: &mut crate::Registry) {
    // Register a lightweight utility object BASE64 with Encode$/Decode$ methods.
    reg.register("BASE64", crate::TypeInfo {
        factory: |_args| {
            Ok(std::rc::Rc::new(std::cell::RefCell::new(Base64Obj {})))
        },
        descriptor: descriptor_static,
        constants: || Vec::new(),
    });
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "BASE64".to_string(),
        version: "1.0".to_string(),
        summary: "Base64 encode/decode utility".to_string(),
        properties: vec![
        ],
        methods: vec![
            MethodDesc { name: "Encode$".to_string(), arity: 1, arg_names: vec!["text$".to_string()], return_type: "String".to_string() },
            MethodDesc { name: "Decode$".to_string(), arity: 1, arg_names: vec!["text$".to_string()], return_type: "String".to_string() },
        ],
        examples: vec![
            "DIM b@ AS BASE64()".to_string(),
            "PRINT b@.Encode$(\"Hello\")".to_string(),
        ],
    }
}

#[derive(Clone)]
struct Base64Obj {}

impl BasicObject for Base64Obj {
    fn type_name(&self) -> &str { "BASE64" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("BASE64 has no properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("BASE64 has no properties".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "ENCODE$" => {
                if args.len()!=1 { return Err(BasilError("Encode$ expects 1 argument".into())); }
                let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                #[cfg(feature = "base64")]
                {
                    let encoded = general_purpose::STANDARD.encode(s.as_bytes());
                    Ok(Value::Str(encoded))
                }
                #[cfg(not(feature = "base64"))]
                { Err(BasilError("BASE64 feature not enabled".into())) }
            }
            "DECODE$" => {
                if args.len()!=1 { return Err(BasilError("Decode$ expects 1 argument".into())); }
                let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                #[cfg(feature = "base64")]
                {
                    match general_purpose::STANDARD.decode(s) {
                        Ok(bytes) => match String::from_utf8(bytes) {
                            Ok(txt) => Ok(Value::Str(txt)),
                            Err(_) => Err(BasilError("BASE64.Decode$: invalid UTF-8 in decoded data".into())),
                        },
                        Err(_) => Err(BasilError("BASE64.Decode$: invalid Base64 string".into())),
                    }
                }
                #[cfg(not(feature = "base64"))]
                { Err(BasilError("BASE64 feature not enabled".into())) }
            }
            other => Err(BasilError(format!("Unknown method '{}' on BASE64", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}
