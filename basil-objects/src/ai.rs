use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

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

use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};

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
        factory: |_args| Ok(new_ai()),
        descriptor: descriptor_static,
        constants: || vec![("AI_VERSION$".to_string(), Value::Str("0.2".to_string()))],
    });
}

pub fn new_ai() -> Rc<RefCell<dyn BasicObject>> {
    let ai = Rc::new(RefCell::new(AiObject::default()));
    let mgr = ConvManager { ai: Rc::downgrade(&ai) };
    ai.borrow_mut().conv_mgr = Some(Rc::new(RefCell::new(mgr)));
    ai
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConversationRegistry {
    version: u32,
    conversations: Vec<ConversationEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConversationEntry {
    id: String,
    backend: String,
    created_at: String,
    last_used_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LocalConversation {
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Default, Clone)]
struct AiObject {
    last_error: String,
    last_response_id: String,
    last_conversation_id: String,
    last_model: String,
    conv_mgr: Option<basil_bytecode::ObjectRef>,
}

struct ConvManager {
    ai: std::rc::Weak<RefCell<AiObject>>,
}

impl BasicObject for ConvManager {
    fn type_name(&self) -> &str { "AI.CONVERSATION" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        Err(BasilError(format!("AI.CONVERSATION has no property '{}'", name)))
    }

    fn set_prop(&mut self, name: &str, _v: Value) -> Result<()> {
        Err(BasilError(format!("AI.CONVERSATION has no settable property '{}'", name)))
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        let ai_rc = self.ai.upgrade().ok_or_else(|| BasilError("AI object no longer exists".into()))?;
        let mut ai = ai_rc.borrow_mut();
        match method.to_ascii_uppercase().as_str() {
            "CREATE$" => {
                ai.clear_err();
                if is_test_mode() {
                    return Ok(Value::Str(format!("conv_test_{}", uuid_simple())));
                }
                let mut backend = "openai".to_string();
                if args.len() >= 1 {
                    let opts_str = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                    if let Ok(opts) = serde_json::from_str::<serde_json::Value>(&opts_str) {
                        if let Some(b) = opts.get("backend").and_then(|v| v.as_str()) { backend = b.to_string(); }
                    }
                }
                if backend == "openai" && resolve_api_key().is_none() { backend = "local".to_string(); }
                
                if backend == "openai" {
                    match resolve_api_key() {
                        Some(api_key) => {
                            match openai_request("POST", "/v1/conversations", Some(json!({})), &api_key, 30000) {
                                Ok(v) => {
                                    let id = v.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                    if id.is_empty() { return Err(ai.set_err("OpenAI did not return a conversation ID")); }
                                    let entry = ConversationEntry { id: id.clone(), backend: "openai".into(), created_at: now_rfc3339(), last_used_at: now_rfc3339() };
                                    let _ = with_registry_lock(|reg| { reg.conversations.push(entry); Ok(()) });
                                    Ok(Value::Str(id))
                                }
                                Err(e) => Err(ai.set_err(format!("OpenAI create failed: {}", e))),
                            }
                        }
                        None => Err(ai.set_err("missing API key")),
                    }
                } else {
                    let id = format!("conv_local_{}", uuid_simple());
                    let entry = ConversationEntry { id: id.clone(), backend: "local".into(), created_at: now_rfc3339(), last_used_at: now_rfc3339() };
                    let _ = with_registry_lock(|reg| { reg.conversations.push(entry); Ok(()) });
                    Ok(Value::Str(id))
                }
            }
            "DELETE" => {
                ai.clear_err();
                if is_test_mode() { return Ok(Value::Int(0)); }
                if args.len() < 1 { return Err(ai.set_err("CONVERSATION.DELETE expects 1 argument")); }
                let id = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                let mut entry_to_delete = None;
                let _ = with_registry_lock(|reg| {
                    if let Some(pos) = reg.conversations.iter().position(|c| c.id == id) {
                        entry_to_delete = Some(reg.conversations.remove(pos));
                    }
                    Ok(())
                });
                if let Some(entry) = entry_to_delete {
                    if entry.backend == "openai" {
                        if let Some(api_key) = resolve_api_key() {
                            let mut after = None;
                            loop {
                                let path = format!("/v1/conversations/{}/items?limit=100&order=desc{}", id, after.as_ref().map(|a| format!("&after={}", a)).unwrap_or_default());
                                match openai_request("GET", &path, None, &api_key, 30000) {
                                    Ok(v) => {
                                        if let Some(items) = v.get("data").and_then(|a| a.as_array()) {
                                            for item in items {
                                                if let Some(iid) = item.get("id").and_then(|s| s.as_str()) {
                                                    let _ = openai_request("DELETE", &format!("/v1/conversations/{}/items/{}", id, iid), None, &api_key, 10000);
                                                }
                                            }
                                        }
                                        if let Some(has_more) = v.get("has_more").and_then(|b| b.as_bool()) {
                                            if has_more {
                                                after = v.get("last_id").and_then(|s| s.as_str()).map(|s| s.to_string());
                                                if after.is_none() { break; }
                                                continue;
                                            }
                                        }
                                        break;
                                    }
                                    Err(_) => break,
                                }
                            }
                            let _ = openai_request("DELETE", &format!("/v1/conversations/{}", id), None, &api_key, 30000);
                        }
                    } else {
                        let _ = fs::remove_file(local_conv_path(&id));
                    }
                }
                Ok(Value::Int(0))
            }
            "LIST$" => {
                ai.clear_err();
                let reg = load_registry();
                let ids: Vec<String> = reg.conversations.iter().map(|c| c.id.clone()).collect();
                Ok(Value::Str(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into())))
            }
            other => Err(BasilError(format!("Unknown method '{}' on AI.CONVERSATION", other))),
        }
    }

    fn descriptor(&self) -> ObjectDescriptor {
        basil_bytecode::ObjectDescriptor {
            type_name: "AI.CONVERSATION".into(),
            version: "0.1".into(),
            summary: "AI Conversation management sub-object".into(),
            properties: vec![],
            methods: vec![
                basil_bytecode::MethodDesc { name: "CREATE$".into(), arity: 1, arg_names: vec!["opts$".into()], return_type: "String".into() },
                basil_bytecode::MethodDesc { name: "DELETE".into(), arity: 1, arg_names: vec!["id$".into()], return_type: "Int".into() },
                basil_bytecode::MethodDesc { name: "LIST$".into(), arity: 0, arg_names: vec![], return_type: "String".into() },
            ],
            examples: vec![],
        }
    }
}

impl AiObject {
    fn clear_err(&mut self) { self.last_error.clear(); }
    fn set_err<T: Into<String>>(&mut self, msg: T) -> BasilError {
        let m = msg.into();
        self.last_error = m.clone();
        BasilError(m)
    }
}

// --- Registry Helpers ---
fn now_rfc3339() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.to_rfc3339()
}

fn ai_config_dir() -> PathBuf {
    let sub = if is_test_mode() { "test-ai" } else { "ai" };
    #[cfg(feature = "dirs")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let mut p = config_dir;
            p.push("basil");
            p.push(sub);
            let _ = fs::create_dir_all(&p);
            return p;
        }
    }
    // Fallback: use .basil/ai in current dir
    let mut p = PathBuf::from(".basil");
    p.push(sub);
    let _ = fs::create_dir_all(&p);
    p
}

fn registry_path() -> PathBuf {
    let mut p = ai_config_dir();
    p.push("conversations.json");
    p
}

fn local_conv_dir() -> PathBuf {
    let mut p = ai_config_dir();
    p.push("conversations");
    let _ = fs::create_dir_all(&p);
    p
}

fn local_conv_path(id: &str) -> PathBuf {
    let mut p = local_conv_dir();
    p.push(format!("{}.json", id));
    p
}

fn load_registry() -> ConversationRegistry {
    let path = registry_path();
    if let Ok(s) = fs::read_to_string(&path) {
        if let Ok(reg) = serde_json::from_str::<ConversationRegistry>(&s) {
            return reg;
        }
    }
    ConversationRegistry { version: 1, conversations: Vec::new() }
}

fn save_registry(reg: &ConversationRegistry) -> std::result::Result<(), String> {
    let path = registry_path();
    let tmp = path.with_extension("json.tmp");
    let s = serde_json::to_string_pretty(reg).map_err(|e| e.to_string())?;
    fs::write(&tmp, s).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

fn with_registry_lock<F, R>(f: F) -> std::result::Result<R, String>
where F: FnOnce(&mut ConversationRegistry) -> std::result::Result<R, String>
{
    let mut lock_path = registry_path();
    lock_path.set_extension("lock");
    
    let mut retry = 0;
    let _lock_file = loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(&lock_path) {
            Ok(f) => f,
            Err(_) => {
                if retry > 20 { return Err("could not acquire registry lock".into()); }
                std::thread::sleep(std::time::Duration::from_millis(50));
                retry += 1;
                continue;
            }
        };
        break;
    };
    
    let mut reg = load_registry();
    let result = f(&mut reg);
    if result.is_ok() {
        let _ = save_registry(&reg);
    }
    
    let _ = fs::remove_file(&lock_path);
    result
}

impl BasicObject for AiObject {
    fn type_name(&self) -> &str { "AI" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "LAST_ERROR$" => Ok(Value::Str(self.last_error.clone())),
            "CONVERSATION" => {
                if let Some(mgr) = &self.conv_mgr {
                    Ok(Value::Object(mgr.clone()))
                } else {
                    Err(BasilError("CONVERSATION manager not initialized".into()))
                }
            }
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
                if !(args.len() >= 1 && args.len() <= 3) { return Err(self.set_err("AI.CHAT$ expects 1 to 3 arguments")); }
                let prompt = match &args[0] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                
                let mut conversation_id = None;
                let mut options_json = None;
                
                if args.len() == 2 {
                    let arg1 = match &args[1] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                    if !arg1.is_empty() {
                        if arg1.starts_with("conv_") { conversation_id = Some(arg1); }
                        else { options_json = Some(arg1); }
                    }
                } else if args.len() == 3 {
                    let arg1 = match &args[1] { Value::Str(s)=>s.clone(), other => format!("{}", other) };
                    if !arg1.is_empty() { conversation_id = Some(arg1); }
                    options_json = Some(match &args[2] { Value::Str(s)=>s.clone(), other => format!("{}", other) });
                }
                
                // Allow conversation_id inside options_json
                if let Some(opts_str) = &options_json {
                    if let Ok(opts) = serde_json::from_str::<serde_json::Value>(opts_str) {
                        if let Some(cid) = opts.get("conversation_id").and_then(|v| v.as_str()) {
                            if conversation_id.is_none() { conversation_id = Some(cid.to_string()); }
                            else if conversation_id.as_deref() != Some(cid) {
                                return Err(self.set_err("Mismatched conversation_id in arguments and options"));
                            }
                        }
                        if conversation_id.is_some() && opts.get("previous_response_id").is_some() {
                            return Err(self.set_err("conversation_id and previous_response_id are mutually exclusive"));
                        }
                    }
                }

                let mut prev_resp_id = None;
                if let Some(opts_str) = &options_json {
                    if let Ok(opts) = serde_json::from_str::<serde_json::Value>(opts_str) {
                        if let Some(prid) = opts.get("previous_response_id").and_then(|v| v.as_str()) {
                            prev_resp_id = Some(prid.to_string());
                        }
                        if let Some(m) = opts.get("model").and_then(|v| v.as_str()) {
                            self.last_model = m.to_string();
                        }
                    }
                }

                if is_test_mode() {
                    let out = test_chat_text(&prompt);
                    self.last_response_id = format!("resp_test_{}", out.len());
                    self.last_conversation_id = conversation_id.unwrap_or_default();
                    return Ok(Value::Str(out));
                }

                // Backend resolution
                let mut backend = "openai".to_string();
                if let Some(cid) = &conversation_id {
                    let reg = load_registry();
                    if let Some(entry) = reg.conversations.iter().find(|c| c.id == *cid) {
                        backend = entry.backend.clone();
                    } else if cid.starts_with("conv_local_") {
                        backend = "local".to_string();
                    }
                }

                if backend == "local" {
                    let cid = conversation_id.unwrap_or_else(|| "conv_local_default".into());
                    let path = local_conv_path(&cid);
                    let mut local_conv = if path.exists() {
                        let s = fs::read_to_string(&path).unwrap_or_default();
                        serde_json::from_str::<LocalConversation>(&s).unwrap_or(LocalConversation { messages: Vec::new() })
                    } else {
                        LocalConversation { messages: Vec::new() }
                    };
                    local_conv.messages.push(ChatMessage { role: "user".into(), content: prompt.clone() });

                    match resolve_api_key() {
                        Some(api_key) => {
                            let mut body = json!({
                                "model": "gpt-4o",
                                "input": local_conv.messages.iter().map(|m| json!({"role": &m.role, "content": &m.content})).collect::<Vec<_>>()
                            });
                            if let Some(opts_str) = &options_json {
                                if let Ok(opts) = serde_json::from_str::<serde_json::Value>(opts_str) {
                                    if let Some(m) = opts.get("model") { body["model"] = m.clone(); }
                                    if let Some(i) = opts.get("instructions") { body["instructions"] = i.clone(); }
                                }
                            }
                            match openai_request("POST", "/v1/responses", Some(body), &api_key, 60000) {
                                Ok(v) => {
                                    let out = v.get("output_text").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                    local_conv.messages.push(ChatMessage { role: "assistant".into(), content: out.clone() });
                                    let _ = fs::write(&path, serde_json::to_string_pretty(&local_conv).unwrap_or_default());
                                    self.last_response_id = v.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                    self.last_conversation_id = cid;
                                    let _ = with_registry_lock(|reg| {
                                        if let Some(entry) = reg.conversations.iter_mut().find(|c| c.id == self.last_conversation_id) {
                                            entry.last_used_at = now_rfc3339();
                                        }
                                        Ok(())
                                    });
                                    Ok(Value::Str(out))
                                }
                                Err(e) => { self.set_err(e); Ok(Value::Str(String::new())) }
                            }
                        }
                        None => { self.set_err("missing API key"); Ok(Value::Str(String::new())) }
                    }
                } else {
                    match resolve_api_key() {
                        Some(api_key) => {
                            match responses_request(&prompt, conversation_id.as_deref(), prev_resp_id.as_deref(), options_json.as_deref(), &api_key) {
                                Ok((out, rid, cid)) => {
                                    self.last_response_id = rid;
                                    self.last_conversation_id = cid;
                                    if !self.last_conversation_id.is_empty() {
                                        let _ = with_registry_lock(|reg| {
                                            if let Some(entry) = reg.conversations.iter_mut().find(|c| c.id == self.last_conversation_id) {
                                                entry.last_used_at = now_rfc3339();
                                            }
                                            Ok(())
                                        });
                                    }
                                    Ok(Value::Str(out))
                                }
                                Err(e) => { self.set_err(e); Ok(Value::Str(String::new())) }
                            }
                        }
                        None => { self.set_err("missing API key"); Ok(Value::Str(String::new())) }
                    }
                }
            }
            "LAST_RESPONSE_ID$" => Ok(Value::Str(self.last_response_id.clone())),
            "LAST_CONVERSATION_ID$" => Ok(Value::Str(self.last_conversation_id.clone())),
            "LAST_META$" => {
                let meta = json!({ "response_id": self.last_response_id, "conversation_id": self.last_conversation_id, "model": self.last_model });
                Ok(Value::Str(meta.to_string()))
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
        version: "0.2".to_string(),
        summary: "AI helpers (chat/stream/embed/moderate/conversation)".to_string(),
        properties: vec![
            basil_bytecode::PropDesc { name: "CONVERSATION".into(), type_name: "AI.CONVERSATION".into(), readable: true, writable: false },
        ],
        methods: vec![
            basil_bytecode::MethodDesc { name: "CHAT$".to_string(), arity: 3, arg_names: vec!["prompt$".to_string(), "conversation_id$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "STREAM".to_string(), arity: 2, arg_names: vec!["prompt$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "EMBED".to_string(), arity: 2, arg_names: vec!["text$".to_string(), "opts$".to_string()], return_type: "Float[]".to_string() },
            basil_bytecode::MethodDesc { name: "MODERATE%".to_string(), arity: 2, arg_names: vec!["text$".to_string(), "opts$".to_string()], return_type: "Int".to_string() },
            basil_bytecode::MethodDesc { name: "KNOWLEDGE$".to_string(), arity: 2, arg_names: vec!["path$".to_string(), "opts$".to_string()], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "LAST_RESPONSE_ID$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "LAST_CONVERSATION_ID$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
            basil_bytecode::MethodDesc { name: "LAST_META$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
        ],
        examples: vec![
            "PRINT AI.CHAT$(\"Hello\")".to_string(),
            "id$ = AI.CONVERSATION.CREATE$()".to_string(),
            "PRINT AI.CHAT$(\"Remember my name is Basil\", id$)".to_string(),
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

fn openai_request(method: &str, path: &str, body: Option<serde_json::Value>, api_key: &str, timeout_ms: u64) -> std::result::Result<serde_json::Value, String> {
    let url = format!("https://api.openai.com{}", path);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build();
    let req = match method {
        "GET" => agent.get(&url),
        "POST" => agent.post(&url),
        "DELETE" => agent.delete(&url),
        _ => return Err(format!("Unsupported method {}", method)),
    };
    let req = req.set("Authorization", &format!("Bearer {}", api_key))
                 .set("Content-Type", "application/json");
    
    let resp = if let Some(b) = body {
        req.send_string(&b.to_string())
    } else {
        req.call()
    };

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_default();
            if text.is_empty() && status >= 200 && status < 300 {
                return Ok(json!({}));
            }
            let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("invalid json: {}", e))?;
            if status >= 200 && status < 300 {
                Ok(val)
            } else {
                let msg = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("");
                if msg.is_empty() { Err(format!("http {}", status)) } else { Err(format!("http {}: {}", status, msg)) }
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
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
    let v = openai_request("POST", "/v1/chat/completions", Some(body), api_key, 60000)?;
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
    let v = openai_request("POST", "/v1/embeddings", Some(body), api_key, 60000)?;
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

fn responses_request(
    prompt: &str,
    conversation_id: Option<&str>,
    previous_response_id: Option<&str>,
    opts_json: Option<&str>,
    api_key: &str,
) -> std::result::Result<(String, String, String), String> {
    let mut body = json!({
        "model": "gpt-4o",
    });
    
    if let Some(opts_str) = opts_json {
        if let Ok(opts) = serde_json::from_str::<serde_json::Value>(opts_str) {
            if let Some(m) = opts.get("model") { body["model"] = m.clone(); }
            if let Some(i) = opts.get("instructions") { body["instructions"] = i.clone(); }
            if let Some(t) = opts.get("temperature") { body["temperature"] = t.clone(); }
            if let Some(m) = opts.get("max_output_tokens") { body["max_output_tokens"] = m.clone(); }
            if let Some(s) = opts.get("store") { body["store"] = s.clone(); }
            if let Some(meta) = opts.get("metadata") { body["metadata"] = meta.clone(); }
        }
    }

    if let Some(cid) = conversation_id {
        body["conversation"] = json!(cid);
        body["input"] = json!(prompt);
    } else if let Some(prid) = previous_response_id {
        body["previous_response_id"] = json!(prid);
        body["input"] = json!(prompt);
    } else {
        body["input"] = json!([
            {"role": "user", "content": prompt}
        ]);
    }

    let v = openai_request("POST", "/v1/responses", Some(body), api_key, 60000)?;
    
    let resp_id = v.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let conv_id = v.get("conversation_id").and_then(|s| s.as_str()).unwrap_or("").to_string();
    
    let mut txt = v.get("output_text").and_then(|s| s.as_str()).map(|s| s.to_string());
    if txt.is_none() {
        if let Some(output) = v.get("output").and_then(|a| a.as_array()) {
            let mut full = String::new();
            for item in output {
                if let Some(content) = item.get("content").and_then(|a| a.as_array()) {
                    for part in content {
                        if let Some(t) = part.get("text").and_then(|s| s.as_str()) {
                            full.push_str(t);
                        }
                    }
                }
            }
            if !full.is_empty() {
                txt = Some(full);
            }
        }
    }
    
    Ok((txt.unwrap_or_default(), resp_id, conv_id))
}

fn uuid_simple() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    #[cfg(feature = "sha2")]
    {
        let mut h = Sha256::new();
        h.update(now.to_le_bytes());
        let d = h.finalize();
        return hex_of(&d[..8]);
    }
    #[allow(unreachable_code)]
    { format!("{:x}", now as u64) }
}
