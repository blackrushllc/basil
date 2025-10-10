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
                // Non-test: require API key (not implemented yet)
                self.set_err("missing API key");
                Ok(Value::Str(String::new()))
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
                self.set_err("missing API key");
                Ok(Value::Str(String::new()))
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
                self.set_err("missing API key");
                // return empty array on error
                let dims = vec![0usize];
                let arr = ArrayObj { elem: ElemType::Num, dims, data: RefCell::new(Vec::new()) };
                Ok(Value::Array(Rc::new(arr)))
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
