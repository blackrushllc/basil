/*

 ▄▄▄▄    ██▓    ▄▄▄       ▄████▄   ██ ▄█▀ ██▀███   █    ██   ██████  ██░ ██
▓█████▄ ▓██▒   ▒████▄    ▒██▀ ▀█   ██▄█▒ ▓██ ▒ ██▒ ██  ▓██▒▒██    ▒ ▓██░ ██▒
▒██▒ ▄██▒██░   ▒██  ▀█▄  ▒▓█    ▄ ▓███▄░ ▓██ ░▄█ ▒▓██  ▒██░░ ▓██▄   ▒██▀▀██░
▒██░█▀  ▒██░   ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄ ▒██▀▀█▄  ▓▓█  ░██░  ▒   ██▒░▓█ ░██
░▓█  ▀█▓░██████▒▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄░██▓ ▒██▒▒▒█████▓ ▒██████▒▒░▓█▒░██▓
░▒▓███▀▒░ ▒░▓  ░▒▒   ▓▒█░░ ░▒ ▒  ░▒ ▒▒ ▓▒░ ▒▓ ░▒▓░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░ ▒ ░░▒░▒
▒░▒   ░ ░ ░ ▒  ░ ▒   ▒▒ ░  ░  ▒   ░ ░▒ ▒░  ░▒ ░ ▒░░░▒░ ░ ░ ░ ░▒  ░ ░ ▒ ░▒░ ░
 ░    ░   ░ ░    ░   ▒   ░        ░ ░░ ░   ░░   ░  ░░░ ░ ░ ░  ░  ░   ░  ░░ ░
 ░          ░  ░     ░  ░░ ░      ░  ░      ░        ░           ░   ░  ░  ░
      ░                  ░
Copyright (C) 2026, Blackrush LLC, All Rights Reserved
Created by Erik Olson, Tarpon Springs, Florida
For more information, visit BlackrushDrive.com

MIT License

Copyright (c) 2026 Erik Lee Olson for Blackrush, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

*/

//! Frame-based VM with calls, locals, jumps, comparisons
use std::rc::Rc;
use std::sync::Arc;
use std::io::{self, Write, Read, Seek, SeekFrom};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use std::env;
use std::time::Duration;
use crossterm::event::{read, Event, KeyEvent, KeyCode};
use crossterm::event::poll;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

pub mod debug;

use basil_common::{Result, BasilError};
use basil_bytecode::{Program as BCProgram, Chunk, Value, Op, ElemType, ArrayObj, ObjectDescriptor, PropDesc, MethodDesc};
use basil_objects::{Registry, register_objects};
use basil_parser::parse as parse_basil;
use basil_compiler::compile as compile_basil;
use basil_bytecode::{deserialize_program};
#[cfg(feature = "obj-base64")]
use base64::{engine::general_purpose, Engine as _};
#[cfg(feature = "obj-zip")]
use basil_objects::zip as zip_utils;
#[cfg(feature = "obj-curl")]
use basil_objects::curl as curl_utils;
#[cfg(any(feature = "obj-json", feature = "obj-csv"))]
use serde_json::{Value as JValue};
#[cfg(feature = "obj-csv")]
use csv::{ReaderBuilder, WriterBuilder};
#[cfg(feature = "obj-sqlite")]
use basil_objects::sqlite as sqlite_utils;
#[cfg(feature = "obj-audio")]
use basil_objects::audio as audio_utils;
#[cfg(feature = "obj-midi")]
use basil_objects::midi as midi_utils;
#[cfg(feature = "obj-daw")]
use basil_objects::daw as daw_utils;

#[cfg(feature = "obj-json")]
fn value_to_jvalue(v: &Value) -> Result<JValue> {
    use serde_json::Map;
    match v {
        Value::Null => Ok(JValue::Null),
        Value::Bool(b) => Ok(JValue::Bool(*b)),
        Value::Int(i) => Ok(JValue::Number((*i).into())),
        Value::Num(n) => serde_json::Number::from_f64(*n)
            .map(JValue::Number)
            .ok_or_else(|| BasilError("JSON_STRINGIFY$: NaN/Inf not representable".into())),
        Value::Str(s) => Ok(JValue::String(s.clone())),
        Value::Array(arr_rc) => {
            let arr = arr_rc.as_ref();
            let data = arr.data.borrow();
            let mut out = Vec::with_capacity(data.len());
            for elem in data.iter() {
                out.push(value_to_jvalue(elem)?);
            }
            Ok(JValue::Array(out))
        }
        Value::Object(obj_rc) => {
            let obj_ref = obj_rc.borrow();
            let desc = obj_ref.descriptor();
            let mut map = Map::new();
            for prop in desc.properties.iter().filter(|p| p.readable) {
                // get_prop returns Result<Value>
                if let Ok(pv) = obj_ref.get_prop(&prop.name) {
                    let jv = value_to_jvalue(&pv)?;
                    map.insert(prop.name.clone(), jv);
                }
            }
            Ok(JValue::Object(map))
        }
        Value::List(items) => {
            let v = items.borrow();
            let mut out = Vec::with_capacity(v.len());
            for it in v.iter() {
                out.push(value_to_jvalue(it)?);
            }
            Ok(JValue::Array(out))
        }
        Value::Dict(map) => {
            let m = map.borrow();
            let mut obj = Map::new();
            for (k, v) in m.iter() {
                obj.insert(k.clone(), value_to_jvalue(v)?);
            }
            Ok(JValue::Object(obj))
        }
        Value::StrArray2D { rows, cols, data } => {
            let mut out: Vec<JValue> = Vec::with_capacity(*rows);
            for r in 0..*rows {
                let mut row: Vec<JValue> = Vec::with_capacity(*cols);
                for c in 0..*cols {
                    let idx = r * *cols + c;
                    row.push(JValue::String(data[idx].clone()));
                }
                out.push(JValue::Array(row));
            }
            Ok(JValue::Array(out))
        }
        Value::Func(_) => Err(BasilError("JSON_STRINGIFY$: cannot stringify a function".into())),
    }
}

// --- Input provider abstraction for test mode ---
pub trait InputProvider {
    fn read_line(&mut self) -> String;       // for INPUT/INPUT$
    fn read_char(&mut self) -> Option<char>; // for INPUTC$/INKEY$/INKEY%
}

// Deterministic mock input provider with simple PRNG-based cycling sequence
pub struct MockInputProvider {
    seq: Vec<u8>, // values 0..=5 selecting among choices
    idx: usize,
}
impl MockInputProvider {
    pub fn new(seed: u64) -> Self {
        let mut s = if seed == 0 { 0x9E3779B97F4A7C15u64 } else { seed };
        let mut seq = Vec::with_capacity(1024);
        for _ in 0..1024 {
            // xorshift64*
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            let r = s.wrapping_mul(0x2545F4914F6CDD1Du64);
            seq.push((r % 6) as u8);
        }
        Self { seq, idx: 0 }
    }
    fn next_index(&mut self) -> u8 {
        let v = self.seq[self.idx];
        self.idx = (self.idx + 1) % self.seq.len();
        v
    }
}
impl InputProvider for MockInputProvider {
    fn read_line(&mut self) -> String {
        match self.next_index() {
            0 => "Y".to_string(),
            1 => "N".to_string(),
            2 => "0".to_string(),
            3 => "1".to_string(),
            4 => "9".to_string(),
            _ => String::new(), // blank
        }
    }
    fn read_char(&mut self) -> Option<char> {
        match self.next_index() {
            0 => Some('Y'),
            1 => Some('N'),
            2 => Some('0'),
            3 => Some('1'),
            4 => Some('9'),
            _ => Some('\r'), // Enter
        }
    }
}

struct Frame {
    chunk: Rc<Chunk>,
    ip: usize,
    base: usize,
}

struct ArrEnum {
    arr: Rc<ArrayObj>,
    cur: isize,    // -1 before first element
    total: usize,  // total elements
}

struct FileHandleEntry {
    file: std::fs::File,
    text: bool,
    readable: bool,
    writable: bool,
    owner_depth: usize,
}

struct HandlerEntry { handler_ip: usize }

pub struct VM {
    frames: Vec<Frame>,
    stack: Vec<Value>,
    globals: Vec<Value>,
    // Keep a copy of global names for reflection/snapshots (REPL, :vars, etc.)
    global_names: Vec<String>,
    registry: Registry,
    enums: Vec<ArrEnum>,
    current_line: u32,
    // Test mode fields
    test_mode: bool,
    trace: bool,
    script_path: Option<String>,
    comments_map: Option<HashMap<u32, Vec<String>>>,
    mocked_inputs: usize,
    max_mocked_inputs: Option<usize>,
    mock: Option<MockInputProvider>,
    // Caches for CGI params
    get_params_cache: Option<Vec<String>>,    // name=value pairs from QUERY_STRING
    post_params_cache: Option<Vec<String>>,   // name=value pairs from stdin (x-www-form-urlencoded)
    // File I/O
    file_table: HashMap<i64, FileHandleEntry>,
    next_fh: i64,
    // Control whether to auto-close handles on function return (used for class methods)
    close_handles_on_ret: bool,
    // GOSUB stack and safety cap
    gosub_stack: Vec<usize>,
    gosub_max_depth: usize,
    // Optional debugger
    pub debugger: Option<Arc<debug::Debugger>>,
    // Exceptions
    _handlers: Vec<HandlerEntry>,
    current_exception: Option<String>,
}

// --- Lightweight Class Instance object ---
struct ClassInstance {
    globals_names: Vec<String>,
    values: Vec<Value>,
    name_to_index: HashMap<String, usize>,
    // Persist open file handles across method calls for this instance
    file_table: HashMap<i64, FileHandleEntry>,
    next_fh: i64,
}

impl ClassInstance {
    fn new(globals_names: Vec<String>, values: Vec<Value>) -> Self {
        let mut name_to_index = HashMap::new();
        for (i, n) in globals_names.iter().enumerate() {
            name_to_index.insert(n.to_ascii_uppercase(), i);
        }
        Self { globals_names, values, name_to_index, file_table: HashMap::new(), next_fh: 1 }
    }

    fn get_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(&name.to_ascii_uppercase()).copied()
    }
}

impl basil_bytecode::BasicObject for ClassInstance {
    fn type_name(&self) -> &str { "CLASS" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        if let Some(i) = self.get_index(name) {
            let v = self.values[i].clone();
            match v {
                Value::Func(_) => Err(BasilError("Unknown property or function in class.".into())),
                other => Ok(other),
            }
        } else {
            Err(BasilError("Unknown property or function in class.".into()))
        }
    }

    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        if let Some(i) = self.get_index(name) {
            if matches!(self.values[i], Value::Func(_)) {
                return Err(BasilError("Unknown property or function in class.".into()));
            }
            self.values[i] = v;
            Ok(())
        } else {
            Err(BasilError("Unknown property or function in class.".into()))
        }
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        let i = self.get_index(method).ok_or_else(|| BasilError("Unknown property or function in class.".into()))?;
        let f = match &self.values[i] { Value::Func(f) => f.clone(), _ => return Err(BasilError("Unknown property or function in class.".into())) };
        // Run function in inner VM with this instance's globals
        // Build a tiny program with empty top chunk (HALT) and same globals names
        let mut top = Chunk::default();
        top.push_op(Op::Halt);
        let prog = BCProgram { chunk: top, globals: self.globals_names.clone() };
        let mut vm = VM::new(prog);
        // Move persistent file handles into inner VM and disable auto-close-on-ret for methods
        vm.file_table = std::mem::take(&mut self.file_table);
        vm.next_fh = self.next_fh;
        vm.close_handles_on_ret = false;
        // Seed globals with our instance values
        vm.globals = self.values.clone();
        // Prepare stack: place arguments starting at base 0
        for a in args { vm.stack.push(a.clone()); }
        // Push frame directly
        let frame = Frame { chunk: f.chunk.clone(), ip: 0, base: 0 };
        vm.frames.push(frame);
        vm.run()?;
        // Capture back persistent file handles into this instance
        self.next_fh = vm.next_fh;
        self.file_table = std::mem::take(&mut vm.file_table);
        // Collect return value
        let ret = vm.stack.pop().unwrap_or(Value::Null);
        // Update our globals from inner VM (so mutations persist)
        self.values = vm.globals.clone();
        Ok(ret)
    }

    fn descriptor(&self) -> ObjectDescriptor {
        let mut props: Vec<PropDesc> = Vec::new();
        let mut methods: Vec<MethodDesc> = Vec::new();
        for (i, n) in self.globals_names.iter().enumerate() {
            match &self.values[i] {
                Value::Func(f) => {
                    methods.push(MethodDesc { name: n.clone(), arity: f.arity, arg_names: Vec::new(), return_type: "ANY".to_string() });
                }
                Value::Array(_) => props.push(PropDesc { name: n.clone(), type_name: "ARRAY".to_string(), readable: true, writable: true }),
                Value::Num(_) => props.push(PropDesc { name: n.clone(), type_name: "FLOAT".to_string(), readable: true, writable: true }),
                Value::Int(_) => props.push(PropDesc { name: n.clone(), type_name: "INTEGER".to_string(), readable: true, writable: true }),
                Value::Str(_) => props.push(PropDesc { name: n.clone(), type_name: "STRING".to_string(), readable: true, writable: true }),
                Value::Bool(_) => props.push(PropDesc { name: n.clone(), type_name: "BOOL".to_string(), readable: true, writable: true }),
                Value::Object(_) => props.push(PropDesc { name: n.clone(), type_name: "OBJECT".to_string(), readable: true, writable: true }),
                Value::Null => props.push(PropDesc { name: n.clone(), type_name: "NULL".to_string(), readable: true, writable: true }),
                Value::List(_) => props.push(PropDesc { name: n.clone(), type_name: "LIST".to_string(), readable: true, writable: true }),
                Value::Dict(_) => props.push(PropDesc { name: n.clone(), type_name: "DICT".to_string(), readable: true, writable: true }),
                Value::StrArray2D { .. } => props.push(PropDesc { name: n.clone(), type_name: "STRARRAY2D".to_string(), readable: true, writable: true }),
            }
        }
        ObjectDescriptor { type_name: "CLASS".to_string(), version: "1.0".to_string(), summary: "Basil file-based class instance".to_string(), properties: props, methods, examples: Vec::new() }
    }
}

impl VM {
    pub fn new(p: BCProgram) -> Self {
        let globals = vec![Value::Null; p.globals.len()];
        let top_chunk = Rc::new(p.chunk);
        let frame = Frame { chunk: top_chunk, ip: 0, base: 0 };
        let mut registry = Registry::new();
        register_objects(&mut registry);
        #[allow(unused_mut)]
        let mut s = Self {
            frames: vec![frame],
            stack: Vec::new(),
            globals,
            // snapshot the names so we can reflect later
            global_names: p.globals.clone(),
            registry,
            enums: Vec::new(),
            current_line: 0,
            test_mode: false,
            trace: false,
            script_path: None,
            comments_map: None,
            mocked_inputs: 0,
            max_mocked_inputs: None,
            mock: None,
            get_params_cache: None,
            post_params_cache: None,
            file_table: HashMap::new(),
            next_fh: 1,
            close_handles_on_ret: true,
            gosub_stack: Vec::new(),
            gosub_max_depth: 4096,
            debugger: None,
            _handlers: Vec::new(),
            current_exception: None,
        };
        #[cfg(feature = "obj-ai")]
        {
            // Reset test flag unless in explicit test VM
            basil_objects::ai::set_test_mode(false);
            // If program references global AI, seed it with an instance
            if s.global_names.iter().any(|n| n.eq_ignore_ascii_case("AI")) {
                let obj = basil_objects::ai::new_ai();
                s.set_global_by_name("AI", Value::Object(obj));
            }
        }
        s
    }

    pub fn new_with_test(p: BCProgram, mock: MockInputProvider, trace: bool, script_path: Option<String>, comments_map: Option<HashMap<u32, Vec<String>>>, max_mocked_inputs: Option<usize>) -> Self {
        let mut vm = VM::new(p);
        vm.test_mode = true;
        vm.trace = trace;
        vm.script_path = script_path;
        vm.comments_map = comments_map;
        vm.max_mocked_inputs = max_mocked_inputs;
        vm.mock = Some(mock);
        #[cfg(feature = "obj-ai")]
        { basil_objects::ai::set_test_mode(true); }
        vm
    }

    pub fn current_line(&self) -> u32 { self.current_line }

    // Provide script path so CLASS() can resolve relative file names
    pub fn set_script_path(&mut self, p: String) { self.script_path = Some(p); }

    // Snapshot (clone) the current global names and values. Useful for REPL sessions.
    pub fn globals_snapshot(&self) -> (Vec<String>, Vec<Value>) {
        (self.global_names.clone(), self.globals.clone())
    }

    // Seed a global by name (case-insensitive). Returns true if found.
    pub fn set_global_by_name(&mut self, name: &str, v: Value) -> bool {
        if let Some(idx) = self.global_names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
            self.globals[idx] = v;
            true
        } else { false }
    }

    fn cur(&mut self) -> &mut Frame { self.frames.last_mut().expect("no frame") }

    // --- CGI param helpers ---
    fn url_decode_form(&self, s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = String::with_capacity(bytes.len());
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => { out.push(' '); i += 1; }
                b'%' if i + 2 < bytes.len() => {
                    let h1 = bytes[i+1] as char; let h2 = bytes[i+2] as char;
                    let hex = [h1, h2];
                    let hv = u8::from_str_radix(&hex.iter().collect::<String>(), 16).ok();
                    if let Some(b) = hv { out.push(b as char); i += 3; } else { out.push('%'); i += 1; }
                }
                b => { out.push(b as char); i += 1; }
            }
        }
        out
    }
    fn url_encode_form(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for &b in s.as_bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                b' ' => out.push('+'),
                _ => {
                    out.push('%');
                    out.push_str(&format!("{:02X}", b));
                }
            }
        }
        out
    }
    fn parse_pairs(&self, s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for part in s.split('&') {
            if part.is_empty() { continue; }
            let mut it = part.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            let kd = self.url_decode_form(k);
            let vd = self.url_decode_form(v);
            out.push(format!("{}={}", kd, vd));
        }
        out
    }
    fn ensure_get_params(&mut self) {
        if self.get_params_cache.is_none() {
            let q = env::var("QUERY_STRING").unwrap_or_default();
            let v = self.parse_pairs(&q);
            self.get_params_cache = Some(v);
        }
    }
    fn ensure_post_params(&mut self) {
        if self.post_params_cache.is_some() { return; }
        let clen: usize = env::var("CONTENT_LENGTH").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        if clen == 0 { self.post_params_cache = Some(Vec::new()); return; }
        let ctype = env::var("CONTENT_TYPE").unwrap_or_default();
        if !ctype.to_ascii_lowercase().starts_with("application/x-www-form-urlencoded") {
            // unsupported type for now
            self.post_params_cache = Some(Vec::new());
            return;
        }
        let mut body = Vec::with_capacity(clen);
        let _ = io::stdin().take(clen as u64).read_to_end(&mut body);
        let s = String::from_utf8_lossy(&body).to_string();
        let v = self.parse_pairs(&s);
        self.post_params_cache = Some(v);
    }
    fn make_string_array(vals: Vec<String>) -> Value {
        use std::cell::RefCell;
        let dims = vec![vals.len()];
        let data: Vec<Value> = vals.into_iter().map(Value::Str).collect();
        let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
        Value::Array(arr)
    }

    fn fh_get_mut(&mut self, h: i64) -> Result<&mut FileHandleEntry> {
        if !self.file_table.contains_key(&h) {
            let mut keys: Vec<i64> = self.file_table.keys().copied().collect();
            keys.sort();
            return Err(BasilError(format!("InvalidHandle (wanted {}, have {:?})", h, keys)));
        }
        Ok(self.file_table.get_mut(&h).unwrap())
    }

    fn fh_close(&mut self, h: i64) -> Result<()> {
        if let Some(mut e) = self.file_table.remove(&h) {
            e.file.flush().map_err(|er| BasilError(format!("FCLOSE flush error: {}", er)))?;
        }
        Ok(())
    }

    fn fh_close_owner_depth(&mut self, depth: usize) {
        let keys: Vec<i64> = self.file_table.iter().filter_map(|(k,e)| if e.owner_depth == depth { Some(*k) } else { None }).collect();
        for k in keys { let _ = self.fh_close(k); }
    }

    fn to_i64(&self, v: &Value) -> Result<i64> {
        match v {
            Value::Int(i) => Ok(*i),
            Value::Num(n) => Ok(n.trunc() as i64),
            other => Err(BasilError(format!("expected numeric value, got {}", self.type_of(other)))),
        }
    }

    fn glob_match_simple(&self, pat: &str, name: &str) -> bool {
        #[allow(clippy::collapsible_else_if)]
        fn inner(p: &[u8], s: &[u8], case_insens: bool) -> bool {
            if p.is_empty() { return s.is_empty(); }
            match p[0] {
                b'*' => {
                    let mut i = 0usize;
                    while i <= s.len() {
                        if inner(&p[1..], &s[i..], case_insens) { return true; }
                        i += 1;
                    }
                    false
                }
                b'?' => {
                    if s.is_empty() { false } else { inner(&p[1..], &s[1..], case_insens) }
                }
                c => {
                    if s.is_empty() { return false; }
                    let mut pc = c;
                    let mut sc = s[0];
                    if case_insens { pc = (pc as char).to_ascii_lowercase() as u8; sc = (sc as char).to_ascii_lowercase() as u8; }
                    if pc != sc { return false; }
                    inner(&p[1..], &s[1..], case_insens)
                }
            }
        }
        let case_insens = cfg!(windows);
        inner(pat.as_bytes(), name.as_bytes(), case_insens)
    }

    pub fn run(&mut self) -> Result<()> {
        if let Some(dbg) = &self.debugger { dbg.emit(debug::DebugEvent::Started); }
        loop {
            let op = self.read_op()?;
            match op {
                Op::Const => {
                    let i = self.read_u16()? as usize;
                    let v = self.cur().chunk.consts[i].clone();
                    self.stack.push(v);
                }
                Op::LoadGlobal => {
                    let i = self.read_u8()? as usize;
                    let v = self.globals[i].clone();
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let i = self.read_u8()? as usize;
                    let v = self.pop()?;
                    self.globals[i] = v;
                }

                Op::LoadLocal => {
                    let i = self.read_u8()? as usize;
                    let base = self.cur().base;
                    let v = self.stack[base + i].clone();
                    self.stack.push(v);
                }
                Op::StoreLocal => {
                    let i = self.read_u8()? as usize;
                    let v = self.pop()?;
                    let base = self.cur().base;
                    while self.stack.len() <= base + i { self.stack.push(Value::Null); }
                    self.stack[base + i] = v;
                }

                Op::Add => {
                    let rb = self.pop()?;
                    let lb = self.pop()?;
                    match (&lb, &rb) {
                        (Value::Str(_), _) | (_, Value::Str(_)) => {
                            let ls = format!("{}", lb);
                            let rs = format!("{}", rb);
                            self.stack.push(Value::Str(format!("{}{}", ls, rs)));
                        }
                        _ => {
                            // numeric addition (use existing numeric coercion)
                            let a = self.as_num(lb)?;
                            let b = self.as_num(rb)?;
                            self.stack.push(Value::Num(a + b));
                        }
                    }
                },
                Op::Sub => self.bin_num(|a,b| a-b)?,
                Op::Mul => self.bin_num(|a,b| a*b)?,
                Op::Div => self.bin_num(|a,b| a/b)?,
                Op::Mod => self.bin_num(|a,b| a % b)?,
                Op::Neg => {
                    let v = self.pop()?;
                    let n = self.as_num(v)?;
                    self.stack.push(Value::Num(-n));
                }

                Op::Eq => self.bin_eq()?,
                Op::Ne => self.bin_ne()?,
                Op::Lt => self.bin_num_cmp(|a,b| a<b)?,
                Op::Le => self.bin_num_cmp(|a,b| a<=b)?,
                Op::Gt => self.bin_num_cmp(|a,b| a>b)?,
                Op::Ge => self.bin_num_cmp(|a,b| a>=b)?,

                Op::Jump => {
                    let off = self.read_u16()? as usize;
                    self.cur().ip += off;
                }
                Op::JumpIfFalse => {
                    let off = self.read_u16()? as usize;
                    let cond = self.pop()?;
                    if !is_truthy(&cond) { self.cur().ip += off; }
                }
                Op::JumpBack => {
                    let off = self.read_u16()? as usize;
                    self.cur().ip -= off;
                }
                Op::Gosub => {
                    let off = self.read_u16()? as usize;
                    let ip_after = self.cur().ip;
                    if self.gosub_stack.len() >= self.gosub_max_depth { return Err(BasilError(format!("GOSUB stack overflow (depth limit {})", self.gosub_max_depth))); }
                    self.gosub_stack.push(ip_after);
                    self.cur().ip += off;
                }
                Op::GosubBack => {
                    let off = self.read_u16()? as usize;
                    let ip_after = self.cur().ip;
                    if self.gosub_stack.len() >= self.gosub_max_depth { return Err(BasilError(format!("GOSUB stack overflow (depth limit {})", self.gosub_max_depth))); }
                    self.gosub_stack.push(ip_after);
                    self.cur().ip -= off;
                }
                Op::GosubRet => {
                    let ret_ip = match self.gosub_stack.pop() { Some(ip) => ip, None => return Err(BasilError("RETURN without GOSUB".into())) };
                    self.cur().ip = ret_ip;
                }
                Op::GosubPop => {
                    if self.gosub_stack.pop().is_none() { return Err(BasilError("RETURN without GOSUB".into())); }
                    // continue execution; typically followed by a Jump to a label
                }
                Op::TryPush => {
                    // Read handler and finally offsets (we ignore finally; compiler handles FINALLY paths)
                    let handler_off = self.read_u16()? as usize;
                    let _finally_off = self.read_u16()? as usize;
                    let target_ip = self.cur().ip + handler_off;
                    self._handlers.push(HandlerEntry { handler_ip: target_ip });
                }
                Op::TryPop => {
                    let _ = self._handlers.pop();
                    // Clear any handled exception when leaving TRY region normally
                    self.current_exception = None;
                }
                Op::Raise => {
                    // Pop message, convert to string, then transfer to nearest handler or abort
                    let msg_v = self.pop()?;
                    let msg = format!("{}", msg_v);
                    if let Some(h) = self._handlers.last() {
                        // record message and jump to handler; also make it available on stack
                        self.current_exception = Some(msg.clone());
                        self.stack.push(Value::Str(msg));
                        let target = h.handler_ip;
                        self.cur().ip = target;
                    } else {
                        return Err(BasilError(msg));
                    }
                }
                Op::Reraise => {
                    // rethrow current exception to next outer handler
                    let msg = match self.current_exception.clone() { Some(m) => m, None => return Err(BasilError("Reraise without active exception".into())) };
                    // Pop current handler if any
                    let _ = self._handlers.pop();
                    if let Some(h) = self._handlers.last() {
                        self.stack.push(Value::Str(msg));
                        let target = h.handler_ip;
                        self.cur().ip = target;
                    } else {
                        return Err(BasilError(msg));
                    }
                }

                Op::Call => {
                    let argc = self.read_u8()? as usize;
                    let callee_idx = self.stack.len() - 1 - argc;
                    let callee = self.stack.remove(callee_idx);
                    let base = callee_idx;
                    match callee {
                        Value::Func(f) => {
                            if f.arity as usize != argc {
                                return Err(BasilError(format!("arity mismatch: expected {}, got {}", f.arity, argc)));
                            }
                            let frame = Frame { chunk: f.chunk.clone(), ip: 0, base };
                            self.frames.push(frame);
                        }
                        _ => return Err(BasilError("CALL target is not a function".into())),
                    }
                }

                Op::SetLine => {
                    let line = self.read_u16()? as u32;
                    self.current_line = line;
                    if self.test_mode {
                        if let Some(map) = &self.comments_map {
                            if let Some(list) = map.get(&line) {
                                for text in list { println!("COMMENT: {}", text); }
                            }
                        }
                    }
                    if let Some(dbg) = &self.debugger {
                        let file = self.script_path.clone().unwrap_or_else(|| "<unknown>".into());
                        let cur_depth = self.frames.len();
                        if dbg.check_pause_point(&file, line as usize, cur_depth) {
                            // Wait until resumed
                            loop {
                                if let Ok(st) = dbg.state.lock() { if !st.paused { break; } }
                                std::thread::sleep(Duration::from_millis(5));
                            }
                        }
                    }
                }

                Op::Ret => {
                    let retv = self.pop().unwrap_or(Value::Null);
                    let depth = self.frames.len();
                    let frame = self.frames.pop().ok_or_else(|| BasilError("RET with no frame".into()))?;
                    self.stack.truncate(frame.base);
                    self.stack.push(retv);
                    // auto-close any file handles opened in this frame (unless suppressed for class methods)
                    if self.close_handles_on_ret {
                        self.fh_close_owner_depth(depth);
                    }
                    if self.frames.is_empty() { break; }
                }

                Op::Print => { let v = self.pop()?; if let Some(dbg) = &self.debugger { dbg.emit(debug::DebugEvent::Output(format!("{}", v))); } print!("{}", v); let _ = io::stdout().flush(); }
                Op::Pop   => { let _ = self.pop()?; }
                Op::ToInt => {
                    let v = self.pop()?;
                    match v {
                        Value::Int(i) => self.stack.push(Value::Int(i)),
                        Value::Num(n) => self.stack.push(Value::Int(n.trunc() as i64)),
                        _ => return Err(BasilError("ToInt expects a numeric value".into())),
                    }
                }

                Op::ArrMake => {
                    let rank = self.read_u8()? as usize;
                    let et_code = self.read_u8()? as u8;
                    let type_cidx = self.read_u16()?; // may be 0xFFFF if not applicable
                    let elem = match et_code {
                        0 => ElemType::Num,
                        1 => ElemType::Int,
                        2 => ElemType::Str,
                        3 => {
                            if type_cidx == 0xFFFF {
                                ElemType::Obj(None)
                            } else {
                                let tn_v = self.cur().chunk.consts[type_cidx as usize].clone();
                                let tn = match tn_v { Value::Str(s) => s, _ => return Err(BasilError("ArrMake type expects string const".into())) };
                                ElemType::Obj(Some(tn))
                            }
                        }
                        _ => return Err(BasilError("bad elem type".into())),
                    };
                    if rank == 0 || rank > 4 { return Err(BasilError("array rank must be 1..4".into())); }
                    let mut uppers: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array dimension must be numeric".into())) };
                        uppers.push(n);
                    }
                    uppers.reverse();
                    let mut dims: Vec<usize> = Vec::with_capacity(rank);
                    let mut total: usize = 1;
                    for u in uppers {
                        if u < 0 { return Err(BasilError("array dimension upper bound must be >= 0".into())); }
                        let len = (u as usize) + 1;
                        dims.push(len);
                        total = total.saturating_mul(len);
                    }
                    let defv = match &elem {
                        ElemType::Num => Value::Num(0.0),
                        ElemType::Int => Value::Int(0),
                        ElemType::Str => Value::Str(String::new()),
                        ElemType::Obj(_) => Value::Null,
                    };
                    let mut data = Vec::with_capacity(total);
                    data.resize(total, defv);
                    let arr = Rc::new(ArrayObj { elem, dims, data: std::cell::RefCell::new(data) });
                    self.stack.push(Value::Array(arr));
                }

                Op::ArrGet => {
                    let rank = self.read_u8()? as usize;
                    let mut idxs: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array index must be numeric".into())) };
                        idxs.push(n);
                    }
                    idxs.reverse();
                    let arr_v = self.pop()?;
                    let arr_rc = match arr_v { Value::Array(rc) => rc, _ => return Err(BasilError("array access on non-array or not DIMed".into())) };
                    let arr = arr_rc.as_ref();
                    if idxs.len() != arr.dims.len() { return Err(BasilError("array rank mismatch".into())); }
                    for (dim_len, idx) in arr.dims.iter().zip(&idxs) {
                        if *idx < 0 || (*idx as usize) >= *dim_len { return Err(BasilError("array index out of bounds".into())); }
                    }
                    // compute linear index (row-major)
                    let mut lin: usize = 0;
                    let mut stride: usize = 1;
                    for d in 0..arr.dims.len() {
                        let len = arr.dims[arr.dims.len() - 1 - d];
                        let idx = idxs[arr.dims.len() - 1 - d] as usize;
                        if d == 0 { lin = idx; stride = len; } else { lin += idx * stride; stride *= len; }
                    }
                    let val = arr.data.borrow()[lin].clone();
                    self.stack.push(val);
                }

                Op::ArrSet => {
                    let rank = self.read_u8()? as usize;
                    let val = self.pop()?;
                    let mut idxs: Vec<i64> = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let v = self.pop()?;
                        let n = match v { Value::Int(i) => i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("array index must be numeric".into())) };
                        idxs.push(n);
                    }
                    idxs.reverse();
                    let arr_v = self.pop()?;
                    let arr_rc = match arr_v { Value::Array(rc) => rc, _ => return Err(BasilError("array write on non-array or not DIMed".into())) };
                    let arr = arr_rc.as_ref();
                    if idxs.len() != arr.dims.len() { return Err(BasilError("array rank mismatch".into())); }
                    for (dim_len, idx) in arr.dims.iter().zip(&idxs) {
                        if *idx < 0 || (*idx as usize) >= *dim_len { return Err(BasilError("array index out of bounds".into())); }
                    }
                    let mut lin: usize = 0;
                    let mut stride: usize = 1;
                    for d in 0..arr.dims.len() {
                        let len = arr.dims[arr.dims.len() - 1 - d];
                        let idx = idxs[arr.dims.len() - 1 - d] as usize;
                        if d == 0 { lin = idx; stride = len; } else { lin += idx * stride; stride *= len; }
                    }
                    let coerced = match &arr.elem {
                        ElemType::Num => match val { Value::Num(n)=>Value::Num(n), Value::Int(i)=>Value::Num(i as f64), other=>return Err(BasilError(format!("cannot store non-numeric {:?} into numeric array", other))) },
                        ElemType::Int => match val { Value::Int(i)=>Value::Int(i), Value::Num(n)=>Value::Int(n.trunc() as i64), other=>return Err(BasilError(format!("cannot store non-numeric {:?} into integer array", other))) },
                        ElemType::Str => match val { Value::Str(s)=>Value::Str(s), other=>Value::Str(format!("{}", other)) },
                        ElemType::Obj(Some(tname)) => match val {
                            Value::Object(rc) => {
                                let got = rc.borrow().type_name().to_string();
                                if got.eq_ignore_ascii_case(tname) { Value::Object(rc) }
                                else { return Err(BasilError(format!("Expected {} in typed object array, got {}.", tname, got))); }
                            }
                            Value::Null => Value::Null,
                            other => return Err(BasilError(format!("cannot store non-object {:?} into typed OBJECT[] array", other))),
                        },
                        ElemType::Obj(None) => match val {
                            Value::Object(_) | Value::Null => val,
                            other => return Err(BasilError(format!("cannot store non-object {:?} into OBJECT[] array", other))),
                        },
                    };
                    arr.data.borrow_mut()[lin] = coerced;
                }

                // enumeration over arrays
                Op::EnumNew => {
                    let it = self.pop()?;
                    match it {
                        Value::Array(rc) => {
                            let total = rc.dims.iter().copied().fold(1usize, |acc, d| acc.saturating_mul(d));
                            let handle = self.enums.len();
                            self.enums.push(ArrEnum { arr: rc, cur: -1, total });
                            self.stack.push(Value::Int(handle as i64));
                        }
                        Value::List(items_rc) => {
                            // Enumerate list elements by materializing a temporary 1-D array view
                            let items = items_rc.borrow();
                            let mut data: Vec<Value> = Vec::with_capacity(items.len());
                            for v in items.iter() { data.push(v.clone()); }
                            let arr = Rc::new(ArrayObj { elem: ElemType::Obj(None), dims: vec![data.len()], data: std::cell::RefCell::new(data) });
                            let total = arr.dims.iter().copied().fold(1usize, |acc, d| acc.saturating_mul(d));
                            let handle = self.enums.len();
                            self.enums.push(ArrEnum { arr, cur: -1, total });
                            self.stack.push(Value::Int(handle as i64));
                        }
                        Value::Dict(map_rc) => {
                            // Enumerate dictionary keys (strings) by materializing a temporary 1-D array of keys
                            let map = map_rc.borrow();
                            let mut data: Vec<Value> = Vec::with_capacity(map.len());
                            for k in map.keys() { data.push(Value::Str(k.clone())); }
                            let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims: vec![data.len()], data: std::cell::RefCell::new(data) });
                            let total = arr.dims.iter().copied().fold(1usize, |acc, d| acc.saturating_mul(d));
                            let handle = self.enums.len();
                            self.enums.push(ArrEnum { arr, cur: -1, total });
                            self.stack.push(Value::Int(handle as i64));
                        }
                        Value::Object(_) => {
                            let ty = self.type_of(&it);
                            return Err(BasilError(format!("FOR EACH expects an array or iterable object after IN (got TYPE={}).", ty)));
                        }
                        other => {
                            let ty = self.type_of(&other);
                            return Err(BasilError(format!("FOR EACH expects an array or iterable object after IN (got TYPE={}).", ty)));
                        }
                    }
                }
                Op::EnumMoveNext => {
                    let handle = match self.stack.last() {
                        Some(Value::Int(i)) => *i as usize,
                        _ => return Err(BasilError("ENUM_MOVENEXT requires enumerator handle on stack".into())),
                    };
                    let e = self.enums.get_mut(handle).ok_or_else(|| BasilError("bad enumerator handle".into()))?;
                    if (e.cur + 1) < e.total as isize { e.cur += 1; self.stack.push(Value::Bool(true)); }
                    else { self.stack.push(Value::Bool(false)); }
                }
                Op::EnumCurrent => {
                    let handle = match self.stack.last() {
                        Some(Value::Int(i)) => *i as usize,
                        _ => return Err(BasilError("ENUM_CURRENT requires enumerator handle on stack".into())),
                    };
                    let e = self.enums.get(handle).ok_or_else(|| BasilError("bad enumerator handle".into()))?;
                    if e.cur < 0 { return Err(BasilError("ENUM_CURRENT before first element".into())); }
                    let lin = e.cur as usize;
                    let val = e.arr.data.borrow()[lin].clone();
                    self.stack.push(val);
                }
                Op::EnumDispose => {
                    let h = self.pop()?;
                    match h {
                        Value::Int(_i) => { /* no-op; freed with VM */ }
                        _ => return Err(BasilError("ENUM_DISPOSE expects enumerator handle".into())),
                    }
                }

                // --- Objects ---
                Op::NewObj => {
                    let type_cidx = self.read_u16()? as usize;
                    let argc = self.read_u8()? as usize;
                    let tname_v = self.cur().chunk.consts[type_cidx].clone();
                    let type_name = match tname_v { Value::Str(s) => s, _ => return Err(BasilError("NEW_OBJ expects type name string const".into())) };
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let obj = self.registry.make(&type_name, &args)?;
                    self.stack.push(Value::Object(obj));
                }
                Op::GetProp => {
                    let prop_cidx = self.read_u16()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("GETPROP expects property name string const".into())) };
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow().get_prop(&prop)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("GETPROP on non-object".into())),
                    }
                }
                Op::SetProp => {
                    let prop_cidx = self.read_u16()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("SETPROP expects property name string const".into())) };
                    let val = self.pop()?;
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            rc.borrow_mut().set_prop(&prop, val)?;
                        }
                        _ => return Err(BasilError("SETPROP on non-object".into())),
                    }
                }
                Op::CallMethod => {
                    let meth_cidx = self.read_u16()? as usize;
                    let argc = self.read_u8()? as usize;
                    let mname_v = self.cur().chunk.consts[meth_cidx].clone();
                    let method = match mname_v { Value::Str(s)=>s, _=>return Err(BasilError("CALLMETHOD expects method name string const".into())) };
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow_mut().call(&method, &args)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("CALLMETHOD on non-object".into())),
                    }
                }
                Op::DescribeObj => {
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let desc = rc.borrow().descriptor();
                            // simple formatting
                            let mut s = String::new();
                            s.push_str(&format!("{} — v{}\n{}\n", desc.type_name, desc.version, desc.summary));
                            if !desc.properties.is_empty() {
                                s.push_str("Properties:\n");
                                for p in desc.properties { s.push_str(&format!("  {} : {} {}{}\n", p.name, p.type_name, if p.readable {"R"} else {""}, if p.writable {"W"} else {""})); }
                            }
                            if !desc.methods.is_empty() {
                                s.push_str("Methods:\n");
                                for m in desc.methods { s.push_str(&format!("  {}({}) -> {}\n", m.name, m.arg_names.join(", "), m.return_type)); }
                            }
                            self.stack.push(Value::Str(s));
                        }
                        Value::Array(arr_rc) => {
                            let arr = arr_rc.as_ref();
                            let elem = match &arr.elem {
                                ElemType::Num => "FLOAT".to_string(),
                                ElemType::Int => "INTEGER".to_string(),
                                ElemType::Str => "STRING".to_string(),
                                ElemType::Obj(Some(t)) => t.clone(),
                                ElemType::Obj(None) => "OBJECT".to_string(),
                            };
                            let mut total: usize = 1;
                            for d in &arr.dims { total = total.saturating_mul(*d); }
                            let dims = if arr.dims.is_empty() { "0".to_string() } else { arr.dims.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("x") };
                            let s = format!("Array — elem={}, dims={}, size={} (row-major)", elem, dims, total);
                            self.stack.push(Value::Str(s));
                        }
                        other => return Err(BasilError(format!("DESCRIBE on unsupported value: {}", self.type_of(&other)))),
                    }
                }

                Op::NewClass => {
                    // Pop filename and instantiate class instance
                    let fname_v = self.pop()?;
                    let fname = match fname_v { Value::Str(s)=>s, other=> return Err(BasilError(format!("CLASS(filename) expects a string, got {}", self.type_of(&other)))) };
                    let (prog, resolved_path) = self.load_class_program(&fname)?;
                    // Run top-level of class program in an inner VM to initialize globals
                    let mut inner = VM::new(prog.clone());
                    inner.set_script_path(resolved_path.clone());
                    inner.run()?;
                    let class_vals = inner.globals.clone();
                    let inst = ClassInstance::new(prog.globals.clone(), class_vals);
                    let rc: basil_bytecode::ObjectRef = Rc::new(std::cell::RefCell::new(inst));
                    self.stack.push(Value::Object(rc));
                }
                Op::GetMember => {
                    let prop_cidx = self.read_u16()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("GETMEMBER expects property name string const".into())) };
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow().get_prop(&prop)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("GETMEMBER on non-object".into())),
                    }
                }
                Op::SetMember => {
                    let prop_cidx = self.read_u16()? as usize;
                    let pname_v = self.cur().chunk.consts[prop_cidx].clone();
                    let prop = match pname_v { Value::Str(s)=>s, _=>return Err(BasilError("SETMEMBER expects property name string const".into())) };
                    let val = self.pop()?;
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            rc.borrow_mut().set_prop(&prop, val)?;
                        }
                        _ => return Err(BasilError("SETMEMBER on non-object".into())),
                    }
                }
                Op::CallMember => {
                    let meth_cidx = self.read_u16()? as usize;
                    let argc = self.read_u8()? as usize;
                    let mname_v = self.cur().chunk.consts[meth_cidx].clone();
                    let method = match mname_v { Value::Str(s)=>s, _=>return Err(BasilError("CALLMEMBER expects method name string const".into())) };
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let target = self.pop()?;
                    match target {
                        Value::Object(rc) => {
                            let v = rc.borrow_mut().call(&method, &args)?;
                            self.stack.push(v);
                        }
                        _ => return Err(BasilError("CALLMEMBER on non-object".into())),
                    }
                }
                Op::DestroyInstance => {
                    // Hint to GC; currently a no-op
                }

                Op::ExecString => {
                    let code_v = self.pop()?;
                    let code = match code_v { Value::Str(s)=>s, other=> return Err(BasilError(format!("EXEC expects a STRING, got {}", self.type_of(&other)))) };
                    let ast = parse_basil(&code)?;
                    let prog = compile_basil(&ast)?;
                    let mut child = VM::new(prog.clone());
                    if let Some(sp) = &self.script_path { child.set_script_path(sp.clone()); }
                    child.run()?;
                    // no value pushed
                }
                Op::EvalString => {
                    let expr_v = self.pop()?;
                    let expr = match expr_v { Value::Str(s)=>s, other=> return Err(BasilError(format!("EVAL expects a STRING, got {}", self.type_of(&other)))) };
                    let src = format!("LET __EVAL_RES = ({});", expr);
                    let ast = parse_basil(&src)?;
                    let prog = compile_basil(&ast)?;
                    let mut child = VM::new(prog.clone());
                    if let Some(sp) = &self.script_path { child.set_script_path(sp.clone()); }
                    child.run()?;
                    // locate result global
                    let mut idx_opt: Option<usize> = None;
                    for (i, name) in prog.globals.iter().enumerate() {
                        if name == "__EVAL_RES" { idx_opt = Some(i); break; }
                    }
                    let idx = idx_opt.ok_or_else(|| BasilError("EVAL internal error: result not found".into()))?;
                    let val = child.globals.get(idx).cloned().unwrap_or(Value::Null);
                    self.stack.push(val);
                }

                Op::Builtin => {
                    let bid = self.read_u8()? as u8;
                    let argc = self.read_u8()? as usize;
                    // pop args in reverse then reverse to preserve call order
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();

                    match bid {
                        1 => { // LEN(arg)
                            if argc != 1 { return Err(BasilError("LEN expects 1 argument".into())); }
                            match &args[0] {
                                Value::Str(s) => {
                                    let n = s.chars().count() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                                Value::Array(arr_rc) => {
                                    let arr = arr_rc.as_ref();
                                    let mut total: usize = 1;
                                    for d in &arr.dims { total = total.saturating_mul(*d); }
                                    self.stack.push(Value::Int(total as i64));
                                }
                                Value::List(rc) => {
                                    let n = rc.borrow().len() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                                Value::Dict(rc) => {
                                    let n = rc.borrow().len() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                                other => {
                                    // Fallback: coerce to string via Display and count chars
                                    let s = format!("{}", other);
                                    let n = s.chars().count() as i64;
                                    self.stack.push(Value::Int(n));
                                }
                            }
                        }
                        2 => { // MID$(s, start [,len]) -- start is 1-based
                            if !(argc == 2 || argc == 3) { return Err(BasilError("MID$ expects 2 or 3 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("MID$ arg 1 must be string".into())) };
                            // convert numeric args to i64 via truncation
                            let start_i = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("MID$ start must be numeric".into())),
                            };
                            let start_idx0 = if start_i <= 1 { 0usize } else { (start_i as usize) - 1 };
                            let mut iter = s.chars();
                            // drop start_idx0 chars
                            for _ in 0..start_idx0 { if iter.next().is_none() { break; } }
                            let res = if argc == 2 {
                                iter.collect::<String>()
                            } else {
                                let len_i = match &args[2] {
                                    Value::Int(i) => *i,
                                    Value::Num(n) => n.trunc() as i64,
                                    _ => return Err(BasilError("MID$ length must be numeric".into())),
                                };
                                if len_i <= 0 { String::new() } else { iter.take(len_i as usize).collect::<String>() }
                            };
                            self.stack.push(Value::Str(res));
                        }
                        3 => { // LEFT$(s, n)
                            if argc != 2 { return Err(BasilError("LEFT$ expects 2 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("LEFT$ arg 1 must be string".into())) };
                            let n = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("LEFT$ count must be numeric".into())),
                            };
                            if n <= 0 { self.stack.push(Value::Str(String::new())); }
                            else {
                                let res: String = s.chars().take(n as usize).collect();
                                self.stack.push(Value::Str(res));
                            }
                        }
                        4 => { // RIGHT$(s, n)
                            if argc != 2 { return Err(BasilError("RIGHT$ expects 2 arguments".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("RIGHT$ arg 1 must be string".into())) };
                            let n = match &args[1] {
                                Value::Int(i) => *i,
                                Value::Num(n) => n.trunc() as i64,
                                _ => return Err(BasilError("RIGHT$ count must be numeric".into())),
                            };
                            if n <= 0 { self.stack.push(Value::Str(String::new())); }
                            else {
                                let total = s.chars().count() as i64;
                                let take_n = if n > total { total } else { n } as usize;
                                let skip_n = (total - take_n as i64) as usize;
                                let res: String = s.chars().skip(skip_n).take(take_n).collect();
                                self.stack.push(Value::Str(res));
                            }
                        }
                        5 => { // INSTR(hay, needle [,start]) -- returns 0-based index or 0 if not found
                            if !(argc == 2 || argc == 3) { return Err(BasilError("INSTR expects 2 or 3 arguments".into())); }
                            let hay = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("INSTR arg 1 must be string".into())) };
                            let needle = match &args[1] { Value::Str(s) => s.clone(), _ => return Err(BasilError("INSTR arg 2 must be string".into())) };
                            let start = if argc == 3 {
                                match &args[2] { Value::Int(i) => *i, Value::Num(n) => n.trunc() as i64, _ => return Err(BasilError("INSTR start must be numeric".into())) }
                            } else { 0 };
                            if needle.is_empty() {
                                // empty needle: return start (already 0-based)
                                let start_nonneg = if start < 0 { 0 } else { start as usize };
                                let total_chars = hay.chars().count();
                                let idx = if start_nonneg > total_chars { total_chars } else { start_nonneg };
                                self.stack.push(Value::Int(idx as i64));
                            } else {
                                // map start (char) to byte index
                                let start_nonneg = if start < 0 { 0 } else { start as usize };
                                let mut byte_idx = 0usize;
                                let mut seen = 0usize;
                                for (bidx, _) in hay.char_indices() {
                                    if seen == start_nonneg { byte_idx = bidx; break; }
                                    seen += 1;
                                    byte_idx = hay.len();
                                }
                                if start_nonneg == 0 { byte_idx = 0; }
                                if start_nonneg > hay.chars().count() { self.stack.push(Value::Int(0)); }
                                else {
                                    let slice = &hay[byte_idx..];
                                    if let Some(rel) = slice.find(&needle) {
                                        let abs_byte = byte_idx + rel;
                                        // convert abs_byte to char index
                                        let idx = hay[..abs_byte].chars().count();
                                        self.stack.push(Value::Int(idx as i64));
                                    } else {
                                        self.stack.push(Value::Int(0));
                                    }
                                }
                            }
                        }
                        6 => { // INPUT$([prompt])
                            if !(argc == 0 || argc == 1) { return Err(BasilError("INPUT$ expects 0 or 1 argument".into())); }
                            if argc == 1 {
                                let prompt = match &args[0] { Value::Str(s) => s.clone(), other => format!("{}", other) };
                                print!("{}", prompt);
                                let _ = io::stdout().flush();
                            }
                            if self.test_mode {
                                // enforce max inputs
                                self.mocked_inputs += 1;
                                if let Some(maxn) = self.max_mocked_inputs { if self.mocked_inputs > maxn { let loc = if let Some(p) = &self.script_path { if self.current_line>0 { format!(" at {}:{}", std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p), self.current_line) } else { String::new() } } else { String::new() }; return Err(BasilError(format!("Hit --max-inputs={}{}", maxn, loc))); } }
                                let val = if let Some(mock) = &mut self.mock { mock.read_line() } else { String::new() };
                                let shown = if val.is_empty() { "<BLANK+ENTER>".to_string() } else { val.clone() };
                                let mut msg = format!("Mock input to INPUT given as {}", shown);
                                if self.trace {
                                    if let Some(p) = &self.script_path { if self.current_line > 0 { let fname = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p); msg.push_str(&format!(" (at {}:{})", fname, self.current_line)); } }
                                }
                                println!("{}", msg);
                                self.stack.push(Value::Str(val));
                            } else {
                                let mut input = String::new();
                                io::stdin().read_line(&mut input).map_err(|e| BasilError(format!("INPUT$ read error: {}", e)))?;
                                while input.ends_with('\n') || input.ends_with('\r') { input.pop(); }
                                self.stack.push(Value::Str(input));
                            }
                        }
                        7 => { // INKEY$() — non-blocking, returns "" if no key available
                            if argc != 0 { return Err(BasilError("INKEY$ expects 0 arguments".into())); }
                            if self.test_mode {
                                self.mocked_inputs += 1;
                                if let Some(maxn) = self.max_mocked_inputs { if self.mocked_inputs > maxn { let loc = if let Some(p) = &self.script_path { if self.current_line>0 { format!(" at {}:{}", std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p), self.current_line) } else { String::new() } } else { String::new() }; return Err(BasilError(format!("Hit --max-inputs={}{}", maxn, loc))); } }
                                let ch = if let Some(mock) = &mut self.mock { mock.read_char() } else { None };
                                let s = match ch { Some('\r') => "\r".to_string(), Some(c) => c.to_string(), None => String::new() };
                                let shown = match ch { Some('\r') => "<ENTER>".to_string(), Some(c) => c.to_string(), None => String::new() };
                                let mut msg = format!("Mock input to INKEY$ given as {}", shown);
                                if self.trace { if let Some(p) = &self.script_path { if self.current_line>0 { let fname = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p); msg.push_str(&format!(" (at {}:{})", fname, self.current_line)); } } }
                                println!("{}", msg);
                                self.stack.push(Value::Str(s));
                            } else {
                                enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                                let s = if poll(Duration::from_millis(0)).map_err(|e| BasilError(format!("poll: {}", e)))? {
                                    match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                        Event::Key(KeyEvent { code, .. }) => match code {
                                            KeyCode::Char(c) => c.to_string(),
                                            KeyCode::Enter => "\r".to_string(),
                                            KeyCode::Backspace => "\u{0008}".to_string(),
                                            KeyCode::Tab => "\t".to_string(),
                                            KeyCode::Esc => "\u{001B}".to_string(),
                                            _ => String::new(),
                                        },
                                        _ => String::new(),
                                    }
                                } else { String::new() };
                                let _ = disable_raw_mode();
                                self.stack.push(Value::Str(s));
                            }
                        }
                        8 => { // INKEY%() — non-blocking, returns 0 if no key available
                            if argc != 0 { return Err(BasilError("INKEY% expects 0 arguments".into())); }
                            if self.test_mode {
                                self.mocked_inputs += 1;
                                if let Some(maxn) = self.max_mocked_inputs { if self.mocked_inputs > maxn { let loc = if let Some(p) = &self.script_path { if self.current_line>0 { format!(" at {}:{}", std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p), self.current_line) } else { String::new() } } else { String::new() }; return Err(BasilError(format!("Hit --max-inputs={}{}", maxn, loc))); } }
                                let ch = if let Some(mock) = &mut self.mock { mock.read_char() } else { None };
                                let code_i: i64 = match ch { Some('\r') => 13, Some(c) => c as i64, None => 0 };
                                let shown = match ch { Some('\r') => "<ENTER>".to_string(), Some(c) => c.to_string(), None => String::new() };
                                let mut msg = format!("Mock input to INKEY% given as {}", shown);
                                if self.trace { if let Some(p) = &self.script_path { if self.current_line>0 { let fname = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p); msg.push_str(&format!(" (at {}:{})", fname, self.current_line)); } } }
                                println!("{}", msg);
                                self.stack.push(Value::Int(code_i));
                            } else {
                                enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                                let code_i: i64 = if poll(Duration::from_millis(0)).map_err(|e| BasilError(format!("poll: {}", e)))? {
                                    match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                        Event::Key(KeyEvent { code, .. }) => {
                                            match code {
                                                KeyCode::Char(c) => c as i64,
                                                KeyCode::Enter => 13,
                                                KeyCode::Backspace => 8,
                                                KeyCode::Tab => 9,
                                                KeyCode::Esc => 27,
                                                KeyCode::Up => 1000,
                                                KeyCode::Down => 1001,
                                                KeyCode::Left => 1002,
                                                KeyCode::Right => 1003,
                                                KeyCode::Home => 1004,
                                                KeyCode::End => 1005,
                                                KeyCode::PageUp => 1006,
                                                KeyCode::PageDown => 1007,
                                                KeyCode::Insert => 1008,
                                                KeyCode::Delete => 1009,
                                                KeyCode::F(n) => 1100 + n as i64,
                                                _ => 0,
                                            }
                                        }
                                        _ => 0,
                                    }
                                } else { 0 };
                                let _ = disable_raw_mode();
                                self.stack.push(Value::Int(code_i));
                            }
                        }
                        9 => { // TYPE$(value)
                            if argc != 1 { return Err(BasilError("TYPE$ expects 1 argument".into())); }
                            let s = self.type_of(&args[0]);
                            self.stack.push(Value::Str(s));
                        }
                        10 => { // HTML/HTML$(x)
                            if argc != 1 { return Err(BasilError("HTML expects 1 argument".into())); }
                            let s = format!("{}", args[0]);
                            let mut out = String::with_capacity(s.len());
                            for ch in s.chars() {
                                match ch {
                                    '&' => out.push_str("&amp;"),
                                    '<' => out.push_str("&lt;"),
                                    '>' => out.push_str("&gt;"),
                                    '"' => out.push_str("&quot;"),
                                    '\'' => out.push_str("&#39;"),
                                    _ => out.push(ch),
                                }
                            }
                            self.stack.push(Value::Str(out));
                        },
                        11 => { // GET$()
                            if argc != 0 { return Err(BasilError("GET$ expects 0 arguments".into())); }
                            self.ensure_get_params();
                            let vals = self.get_params_cache.clone().unwrap_or_default();
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        12 => { // POST$()
                            if argc != 0 { return Err(BasilError("POST$ expects 0 arguments".into())); }
                            self.ensure_post_params();
                            let vals = self.post_params_cache.clone().unwrap_or_default();
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        13 => { // REQUEST$()
                            if argc != 0 { return Err(BasilError("REQUEST$ expects 0 arguments".into())); }
                            self.ensure_get_params();
                            self.ensure_post_params();
                            let mut vals = self.get_params_cache.clone().unwrap_or_default();
                            if let Some(mut p) = self.post_params_cache.clone() { vals.append(&mut p); }
                            let arr = VM::make_string_array(vals);
                            self.stack.push(arr);
                        }
                        14 => { // UCASE$(s)
                            if argc != 1 { return Err(BasilError("UCASE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("UCASE$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.to_uppercase()));
                        }
                        15 => { // LCASE$(s)
                            if argc != 1 { return Err(BasilError("LCASE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("LCASE$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.to_lowercase()));
                        }
                        16 => { // TRIM$(s)
                            if argc != 1 { return Err(BasilError("TRIM$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("TRIM$ arg must be string".into())) };
                            self.stack.push(Value::Str(s.trim().to_string()));
                        }
                        17 => { // CHR$(n)
                            if argc != 1 { return Err(BasilError("CHR$ expects 1 argument".into())); }
                            let n = match &args[0] {
                                Value::Int(i) => *i,
                                Value::Num(f) => f.trunc() as i64,
                                _ => return Err(BasilError("CHR$ arg must be numeric".into())),
                            };
                            let out = if n < 0 || n > 0x10FFFF { String::new() } else { std::char::from_u32(n as u32).map(|c| c.to_string()).unwrap_or_default() };
                            self.stack.push(Value::Str(out));
                        }
                        18 => { // ASC%(s)
                            if argc != 1 { return Err(BasilError("ASC% expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s, _ => return Err(BasilError("ASC% arg must be string".into())) };
                            let code: i64 = s.chars().next().map(|c| c as u32 as i64).unwrap_or(0);
                            self.stack.push(Value::Int(code));
                        }
                        19 => { // INPUTC$([prompt])
                            if !(argc == 0 || argc == 1) { return Err(BasilError("INPUTC$ expects 0 or 1 argument".into())); }
                            if argc == 1 {
                                let prompt = match &args[0] { Value::Str(s) => s.clone(), other => format!("{}", other) };
                                print!("{}", prompt);
                                let _ = io::stdout().flush();
                            }
                            if self.test_mode {
                                self.mocked_inputs += 1;
                                if let Some(maxn) = self.max_mocked_inputs { if self.mocked_inputs > maxn { let loc = if let Some(p) = &self.script_path { if self.current_line>0 { format!(" at {}:{}", std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p), self.current_line) } else { String::new() } } else { String::new() }; return Err(BasilError(format!("Hit --max-inputs={}{}", maxn, loc))); } }
                                let ch = if let Some(mock) = &mut self.mock { mock.read_char() } else { None };
                                let s = match ch { Some('\r') => String::new(), Some(c) => c.to_string(), None => String::new() };
                                if let Some(c) = ch { if c != '\r' { print!("{}", c); let _ = io::stdout().flush(); } }
                                let shown = match ch { Some('\r') => "<ENTER>".to_string(), Some(c) => c.to_string(), None => String::new() };
                                let mut msg = format!("Mock input to INPUTC$ given as {}", shown);
                                if self.trace { if let Some(p) = &self.script_path { if self.current_line>0 { let fname = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p); msg.push_str(&format!(" (at {}:{})", fname, self.current_line)); } } }
                                println!("{}", msg);
                                self.stack.push(Value::Str(s));
                            } else {
                                // Enable raw mode and ensure we only capture a single key (no echo from console)
                                enable_raw_mode().map_err(|e| BasilError(format!("enable_raw_mode: {}", e)))?;
                                // Drain any pending events (typeahead) to avoid consuming earlier keys
                                loop {
                                    match poll(Duration::from_millis(0)) {
                                        Ok(true) => { let _ = read(); }
                                        Ok(false) => break,
                                        Err(e) => { let _ = disable_raw_mode(); return Err(BasilError(format!("poll: {}", e))); }
                                    }
                                }
                                // Wait for the next key event (any kind). Capture only ASCII chars; others => "".
                                let s = loop {
                                    match read().map_err(|e| BasilError(format!("read key: {}", e)))? {
                                        Event::Key(KeyEvent { code, .. }) => {
                                            let out = match code {
                                                KeyCode::Char(c) if c.is_ascii() => c.to_string(),
                                                _ => String::new(),
                                            };
                                            break out;
                                        }
                                        _ => { /* ignore non-key events */ }
                                    }
                                };
                                // Echo the captured ASCII character exactly once
                                if !s.is_empty() { print!("{}", s); let _ = io::stdout().flush(); }
                                let _ = disable_raw_mode();
                                self.stack.push(Value::Str(s));
                            }
                        }
                        20 => { // ESCAPE$(s) - SQL string literal escape (single quotes doubled)
                            if argc != 1 { return Err(BasilError("ESCAPE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.clone(), _ => return Err(BasilError("ESCAPE$ arg must be string".into())) };
                            let out = s.replace("'", "''");
                            self.stack.push(Value::Str(out));
                        }
                        21 => { // UNESCAPE$(s) - reverse SQL string literal escaping ('' -> ')
                            if argc != 1 { return Err(BasilError("UNESCAPE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.as_str(), _ => return Err(BasilError("UNESCAPE$ arg must be string".into())) };
                            let mut out = String::with_capacity(s.len());
                            let mut it = s.chars().peekable();
                            while let Some(c) = it.next() {
                                if c == '\'' {
                                    if let Some('\'') = it.peek().copied() { it.next(); out.push('\''); } else { out.push('\''); }
                                } else { out.push(c); }
                            }
                            self.stack.push(Value::Str(out));
                        }
                        22 => { // URLENCODE$(s) - application/x-www-form-urlencoded encode (spaces -> '+')
                            if argc != 1 { return Err(BasilError("URLENCODE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.as_str(), _ => return Err(BasilError("URLENCODE$ arg must be string".into())) };
                            let out = self.url_encode_form(s);
                            self.stack.push(Value::Str(out));
                        }
                        23 => { // URLDECODE$(s) - application/x-www-form-urlencoded decode ('+' -> space)
                            if argc != 1 { return Err(BasilError("URLDECODE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s) => s.as_str(), _ => return Err(BasilError("URLDECODE$ arg must be string".into())) };
                            let out = self.url_decode_form(s);
                            self.stack.push(Value::Str(out));
                        }
                        24 => { // SLEEP(ms)
                            if argc != 1 { return Err(BasilError("SLEEP expects 1 argument".into())); }
                            let ms = self.to_i64(&args[0])?;
                            let msu = if ms < 0 { 0 } else { ms as u64 };
                            std::thread::sleep(std::time::Duration::from_millis(msu));
                            self.stack.push(Value::Int(0));
                        }
                        26 => { // STRING$(n, ch$ or code%)
                            if argc != 2 { return Err(BasilError("STRING$ expects 2 arguments".into())); }
                            let n = self.to_i64(&args[0])?;
                            let n = if n <= 0 { 0usize } else { (n as usize).min(1_000_000) };
                            let unit = match &args[1] {
                                Value::Str(s) => s.clone(),
                                other => {
                                    let code = self.to_i64(other)?;
                                    let ch = std::char::from_u32((code as u32) & 0xFF).unwrap_or('\u{0000}');
                                    ch.to_string()
                                }
                            };
                            let out = if unit.is_empty() || n == 0 { String::new() } else { unit.repeat(n) };
                            self.stack.push(Value::Str(out));
                        }
                        40 => { // FOPEN(path$, mode$) -> fh%
                            if argc != 2 { return Err(BasilError("FOPEN expects 2 arguments".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let mode = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            if path.contains('\u{0000}') { return Err(BasilError("FOPEN: invalid NUL in path".into())); }
                            let m = mode.to_ascii_lowercase();
                            let text = !m.contains('b');
                            let plus = m.contains('+');
                            let mut opts = OpenOptions::new();
                            let (mut readable, mut writable) = (false, false);
                            if m.starts_with('r') { readable = true; opts.read(true); if plus { writable = true; opts.write(true); } }
                            else if m.starts_with('w') { writable = true; opts.write(true).create(true).truncate(true); if plus { readable = true; opts.read(true); } }
                            else if m.starts_with('a') { writable = true; opts.append(true).create(true); if plus { readable = true; opts.read(true); opts.write(true); } }
                            else { return Err(BasilError(format!("FOPEN: invalid mode '{}'; expected r/w/a variants", mode))); }
                            match opts.open(&path) {
                                Ok(file) => {
                                    let fh = self.next_fh; self.next_fh += 1;
                                    let entry = FileHandleEntry { file, text, readable, writable, owner_depth: self.frames.len() };
                                    self.file_table.insert(fh, entry);
                                    self.stack.push(Value::Int(fh));
                                }
                                Err(_e) => {
                                    // Non-throwing failure: return -1 to signal open error
                                    self.stack.push(Value::Int(-1));
                                }
                            }
                        }
                        41 => { // FCLOSE fh%
                            if argc != 1 { return Err(BasilError("FCLOSE expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?; let _ = self.fh_close(h); self.stack.push(Value::Bool(true));
                        }
                        42 => { // FFLUSH fh%
                            if argc != 1 { return Err(BasilError("FFLUSH expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; e.file.flush().map_err(|er| BasilError(format!("FFLUSH error: {}", er)))?; self.stack.push(Value::Bool(true));
                        }
                        43 => { // FEOF(fh%) -> BOOL
                            if argc != 1 { return Err(BasilError("FEOF expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; let cur = e.file.stream_position().map_err(|er| BasilError(format!("FEOF tell: {}", er)))?; let mut b=[0u8;1]; let n = e.file.read(&mut b).map_err(|er| BasilError(format!("FEOF read: {}", er)))?; if n>0 { let _ = e.file.seek(SeekFrom::Start(cur)); }
                            self.stack.push(Value::Bool(n==0));
                        }
                        44 => { // FTELL&(fh%) -> LONG
                            if argc != 1 { return Err(BasilError("FTELL& expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; let pos = e.file.stream_position().map_err(|er| BasilError(format!("FTELL: {}", er)))?; self.stack.push(Value::Int(pos as i64));
                        }
                        45 => { // FSEEK fh%, offset&, whence%
                            if argc != 3 { return Err(BasilError("FSEEK expects 3 arguments".into())); }
                            let h = self.to_i64(&args[0])?; let off = self.to_i64(&args[1])?; let wh = self.to_i64(&args[2])?; let e = self.fh_get_mut(h)?;
                            let whence = match wh { 0 => SeekFrom::Start(off as u64), 1 => SeekFrom::Current(off), 2 => SeekFrom::End(off), _ => return Err(BasilError("FSEEK: whence must be 0,1,2".into())) };
                            let _ = e.file.seek(whence).map_err(|er| BasilError(format!("FSEEK: {}", er)))?; self.stack.push(Value::Bool(true));
                        }
                        46 => { // FREAD$(fh%, n&) -> STRING
                            if argc != 2 { return Err(BasilError("FREAD$ expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?; let n = self.to_i64(&args[1])?; if n <= 0 { self.stack.push(Value::Str(String::new())); }
                            else { let e = self.fh_get_mut(h)?; if !e.readable { return Err(BasilError("FREAD$: handle not opened for reading".into())); } let mut buf = vec![0u8; n as usize]; let got = e.file.read(&mut buf).map_err(|er| BasilError(format!("FREAD$: {}", er)))?; buf.truncate(got); let s = if e.text { String::from_utf8_lossy(&buf).to_string() } else { String::from_utf8_lossy(&buf).to_string() }; self.stack.push(Value::Str(s)); }
                        }
                        47 => { // FREADLINE$(fh%) -> STRING
                            if argc != 1 { return Err(BasilError("FREADLINE$ expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; if !e.readable { return Err(BasilError("FREADLINE$: handle not opened for reading".into())); }
                            let mut out: Vec<u8> = Vec::new();
                            let mut buf = [0u8;1];
                            loop {
                                let n = e.file.read(&mut buf).map_err(|er| BasilError(format!("FREADLINE$: {}", er)))?;
                                if n == 0 { break; }
                                if buf[0] == b'\n' { break; }
                                out.push(buf[0]);
                            }
                            if out.ends_with(&[b'\r']) { out.pop(); }
                            let s = if e.text { String::from_utf8_lossy(&out).to_string() } else { String::from_utf8_lossy(&out).to_string() };
                            self.stack.push(Value::Str(s));
                        }
                        48 => { // FWRITE fh%, s$
                            if argc != 2 { return Err(BasilError("FWRITE expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; if !e.writable { return Err(BasilError("FWRITE: handle not opened for writing".into())); }
                            let s = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            e.file.write_all(s.as_bytes()).map_err(|er| BasilError(format!("FWRITE: {}", er)))?; self.stack.push(Value::Bool(true));
                        }
                        49 => { // FWRITELN fh%, s$
                            if argc != 2 { return Err(BasilError("FWRITELN expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?; let e = self.fh_get_mut(h)?; if !e.writable { return Err(BasilError("FWRITELN: handle not opened for writing".into())); }
                            let s = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            e.file.write_all(s.as_bytes()).map_err(|er| BasilError(format!("FWRITELN: {}", er)))?; e.file.write_all(b"\n").map_err(|er| BasilError(format!("FWRITELN: {}", er)))?; self.stack.push(Value::Bool(true));
                        }
                        50 => { // READFILE$(path$)
                            if argc != 1 { return Err(BasilError("READFILE$ expects 1 argument".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let data = fs::read(&path).map_err(|e| BasilError(format!("READFILE$ {}: {}", path, e)))?; let s = String::from_utf8_lossy(&data).to_string(); self.stack.push(Value::Str(s));
                        }
                        51 => { // WRITEFILE path$, data$
                            if argc != 2 { return Err(BasilError("WRITEFILE expects 2 arguments".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let s = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let mut f = OpenOptions::new().write(true).create(true).truncate(true).open(&path).map_err(|e| BasilError(format!("WRITEFILE {}: {}", path, e)))?; f.write_all(s.as_bytes()).map_err(|e| BasilError(format!("WRITEFILE {}: {}", path, e)))?; f.flush().ok(); self.stack.push(Value::Null);
                        }
                        52 => { // APPENDFILE path$, data$
                            if argc != 2 { return Err(BasilError("APPENDFILE expects 2 arguments".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let s = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let mut f = OpenOptions::new().write(true).create(true).append(true).open(&path).map_err(|e| BasilError(format!("APPENDFILE {}: {}", path, e)))?; f.write_all(s.as_bytes()).map_err(|e| BasilError(format!("APPENDFILE {}: {}", path, e)))?; f.flush().ok(); self.stack.push(Value::Null);
                        }
                        53 => { // COPY src$, dst$
                            if argc != 2 { return Err(BasilError("COPY expects 2 arguments".into())); }
                            let src = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let dst = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let _ = fs::copy(&src, &dst).map_err(|e| BasilError(format!("COPY {} -> {}: {}", src, dst, e)))?; self.stack.push(Value::Null);
                        }
                        54 => { // MOVE src$, dst$
                            if argc != 2 { return Err(BasilError("MOVE expects 2 arguments".into())); }
                            let src = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let dst = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            fs::rename(&src, &dst).map_err(|e| BasilError(format!("MOVE {} -> {}: {}", src, dst, e)))?; self.stack.push(Value::Null);
                        }
                        55 => { // RENAME path$, newname$
                            if argc != 2 { return Err(BasilError("RENAME expects 2 arguments".into())); }
                            let src = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let newname = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let p = Path::new(&src); let dir = p.parent().unwrap_or(Path::new(".")); let dst = dir.join(newname);
                            fs::rename(&src, &dst).map_err(|e| BasilError(format!("RENAME {} -> {}: {}", src, dst.display(), e)))?; self.stack.push(Value::Null);
                        }
                        56 => { // DELETE path$
                            if argc != 1 { return Err(BasilError("DELETE expects 1 argument".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            fs::remove_file(&path).map_err(|e| BasilError(format!("DELETE {}: {}", path, e)))?; self.stack.push(Value::Null);
                        }
                        57 => { // DIR$(pattern$) -> STRING[]
                            if argc != 1 { return Err(BasilError("DIR$ expects 1 argument".into())); }
                            let patt = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let p = Path::new(&patt);
                            let (dir, patstr): (PathBuf, String) = if p.components().count() > 1 {
                                (p.parent().unwrap_or(Path::new(".")).to_path_buf(), p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string())
                            } else { (PathBuf::from("."), patt.clone()) };
                            let mut names: Vec<String> = Vec::new();
                            for ent in fs::read_dir(&dir).map_err(|e| BasilError(format!("DIR$: {}: {}", dir.display(), e)))? {
                                let ent = ent.map_err(|e| BasilError(format!("DIR$: {}", e)))?;
                                let md = ent.metadata().map_err(|e| BasilError(format!("DIR$: {}", e)))?;
                                if !md.is_file() { continue; }
                                let name = ent.file_name().to_string_lossy().to_string();
                                if self.glob_match_simple(&patstr, &name) { names.push(name); }
                            }
                            names.sort();
                            self.stack.push(VM::make_string_array(names));
                        }
                        58 => { // ENV$(name$)
                            if argc != 1 { return Err(BasilError("ENV$ expects 1 argument".into())); }
                            let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let val = env::var(&name).unwrap_or_default();
                            self.stack.push(Value::Str(val));
                        }
                        59 => { // SETENV/EXPORTENV name$, value, exportFlag
                            if argc != 3 { return Err(BasilError("SETENV expects 3 arguments (name$, value, exportFlag)".into())); }
                            let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let value_str = match &args[1] {
                                Value::Str(s) => s.clone(),
                                Value::Int(i) => i.to_string(),
                                Value::Num(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                other => format!("{}", other),
                            };
                            let export = match &args[2] {
                                Value::Bool(b) => *b,
                                Value::Int(i) => *i != 0,
                                Value::Num(n) => *n != 0.0,
                                _ => false,
                            };
                            env::set_var(&name, &value_str);
                            let mut ok = true;
                            if export {
                                #[cfg(windows)]
                                {
                                    use std::process::Command;
                                    let status = Command::new("cmd").args(["/C", "setx", &name, &value_str]).status();
                                    ok = status.map(|s| s.success()).unwrap_or(false);
                                }
                                #[cfg(not(windows))]
                                {
                                    // Cannot export to parent shell from child; consider process-local set sufficient
                                    ok = true;
                                }
                            }
                            self.stack.push(Value::Bool(ok));
                        }
                        60 => { // SHELL(cmd$) -> exit code
                            if argc != 1 { return Err(BasilError("SHELL expects 1 argument".into())); }
                            let cmd = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            #[cfg(windows)]
                            let status = std::process::Command::new("cmd").args(["/C", &cmd]).status();
                            #[cfg(not(windows))]
                            let status = std::process::Command::new("sh").args(["-c", &cmd]).status();
                            let code_i: i64 = match status {
                                Ok(st) => st.code().unwrap_or(-1) as i64,
                                Err(_) => -1,
                            };
                            self.stack.push(Value::Int(code_i));
                        }
                        61 => { // EXIT(code)
                            if argc != 1 { return Err(BasilError("EXIT expects 1 argument".into())); }
                            let code = self.to_i64(&args[0])? as i32;
                            std::process::exit(code);
                        }
                        62 => { // MKDIRS%(path$) -> Int (1=ok,0=fail)
                            if argc != 1 { return Err(BasilError("MKDIRS% expects 1 argument".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            match fs::create_dir_all(&path) {
                                Ok(_) => self.stack.push(Value::Int(1)),
                                Err(_e) => self.stack.push(Value::Int(0)),
                            }
                        }
                        63 => { // LOADENV%(filename$?) -> Int (1=ok,0=fail)
                            if !(argc == 0 || argc == 1) { return Err(BasilError("LOADENV% expects 0 or 1 argument".into())); }
                            let default_name = ".env".to_string();
                            let file = if argc == 0 {
                                default_name
                            } else {
                                let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                                let t = s.trim();
                                if t.is_empty() { ".env".to_string() } else { t.to_string() }
                            };
                            match fs::read_to_string(&file) {
                                Ok(contents) => {
                                    for (i, line) in contents.lines().enumerate() {
                                        let trimmed = line.trim();
                                        if trimmed.is_empty() { continue; }
                                        if trimmed.starts_with('#') || trimmed.starts_with(';') { continue; }
                                        match trimmed.find('=') {
                                            Some(eq) => {
                                                let key = trimmed[..eq].trim();
                                                let val_raw = trimmed[eq+1..].trim();
                                                if key.is_empty() {
                                                    eprintln!("warning: LOADENV% {}:{}: missing key before '='", file, i + 1);
                                                    continue;
                                                }
                                                let unquoted = if (val_raw.starts_with('"') && val_raw.ends_with('"') && val_raw.len() >= 2) ||
                                                                (val_raw.starts_with('\'') && val_raw.ends_with('\'') && val_raw.len() >= 2) {
                                                    val_raw[1..val_raw.len()-1].to_string()
                                                } else { val_raw.to_string() };
                                                env::set_var(key, unquoted);
                                            }
                                            None => {
                                                eprintln!("warning: LOADENV% {}:{}: invalid line (expected name=value or comment)", file, i + 1);
                                            }
                                        }
                                    }
                                    self.stack.push(Value::Int(1));
                                }
                                Err(e) => {
                                    eprintln!("warning: LOADENV% could not read {}: {}", file, e);
                                    self.stack.push(Value::Int(0));
                                }
                            }
                        }
                        #[cfg(feature = "obj-base64")]
                        90 => { // BASE64_ENCODE$(text$)
                            if argc != 1 { return Err(BasilError("BASE64_ENCODE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let encoded = general_purpose::STANDARD.encode(s.as_bytes());
                            self.stack.push(Value::Str(encoded));
                        }
                        #[cfg(feature = "obj-base64")]
                        91 => { // BASE64_DECODE$(text$)
                            if argc != 1 { return Err(BasilError("BASE64_DECODE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            match general_purpose::STANDARD.decode(s) {
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(txt) => self.stack.push(Value::Str(txt)),
                                    Err(_) => return Err(BasilError("BASE64_DECODE$: invalid UTF-8 in decoded data".into())),
                                },
                                Err(_) => return Err(BasilError("BASE64_DECODE$: invalid Base64 string".into())),
                            }
                        }
                        #[cfg(feature = "obj-zip")]
                        120 => { // ZIP_EXTRACT_ALL(zip_path$, dest_dir$)
                            if argc != 2 { return Err(BasilError("ZIP_EXTRACT_ALL expects 2 arguments".into())); }
                            let zip_path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let dest_dir = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            zip_utils::zip_extract_all(&zip_path, &dest_dir)?;
                            self.stack.push(Value::Str(String::new()));
                        }
                        #[cfg(feature = "obj-zip")]
                        121 => { // ZIP_COMPRESS_FILE(src_path$, zip_path$, entry_name$)
                            if !(argc == 2 || argc == 3) { return Err(BasilError("ZIP_COMPRESS_FILE expects 2 or 3 arguments".into())); }
                            let src_path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let zip_path = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let entry_opt: Option<String> = if argc == 3 { Some(match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) }) } else { None };
                            zip_utils::zip_compress_file(&src_path, &zip_path, entry_opt.as_deref())?;
                            self.stack.push(Value::Str(String::new()));
                        }
                        #[cfg(feature = "obj-zip")]
                        122 => { // ZIP_COMPRESS_DIR(src_dir$, zip_path$)
                            if argc != 2 { return Err(BasilError("ZIP_COMPRESS_DIR expects 2 arguments".into())); }
                            let src_dir = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let zip_path = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            zip_utils::zip_compress_dir(&src_dir, &zip_path)?;
                            self.stack.push(Value::Str(String::new()));
                        }
                        #[cfg(feature = "obj-zip")]
                        123 => { // ZIP_LIST$(zip_path$)
                            if argc != 1 { return Err(BasilError("ZIP_LIST$ expects 1 argument".into())); }
                            let zip_path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let listing = zip_utils::zip_list(&zip_path)?;
                            self.stack.push(Value::Str(listing));
                        }
                        #[cfg(feature = "obj-curl")]
                        124 => { // HTTP_GET$(url$)
                            if argc != 1 { return Err(BasilError("HTTP_GET$ expects 1 argument".into())); }
                            let url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let body = curl_utils::http_get(&url)?;
                            self.stack.push(Value::Str(body));
                        }
                        #[cfg(feature = "obj-curl")]
                        125 => { // HTTP_POST$(url$, body$[, content_type$])
                            if !(argc == 2 || argc == 3) { return Err(BasilError("HTTP_POST$ expects 2 or 3 arguments".into())); }
                            let url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let ct_opt: Option<String> = if argc == 3 { Some(match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) }) } else { None };
                            let resp = curl_utils::http_post(&url, &body, ct_opt.as_deref())?;
                            self.stack.push(Value::Str(resp));
                        }
                        #[cfg(feature = "obj-json")]
                        126 => { // JSON_PARSE$(text$)
                            if argc != 1 { return Err(BasilError("JSON_PARSE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let v: JValue = serde_json::from_str(&s)
                                .map_err(|e| BasilError(format!("JSON_PARSE$: invalid JSON: {}", e)))?;
                            let out = serde_json::to_string(&v)
                                .map_err(|e| BasilError(format!("JSON_PARSE$: serialize failed: {}", e)))?;
                            self.stack.push(Value::Str(out));
                        }
                        #[cfg(feature = "obj-json")]
                        127 => { // JSON_STRINGIFY$(value)
                            if argc != 1 { return Err(BasilError("JSON_STRINGIFY$ expects 1 argument".into())); }
                            match &args[0] {
                                Value::Str(s) => {
                                    if let Ok(v) = serde_json::from_str::<JValue>(&s) {
                                        let out = serde_json::to_string(&v)
                                            .map_err(|e| BasilError(format!("JSON_STRINGIFY$: serialize failed: {}", e)))?;
                                        self.stack.push(Value::Str(out));
                                    } else {
                                        let out = serde_json::to_string(&s)
                                            .map_err(|e| BasilError(format!("JSON_STRINGIFY$: wrap failed: {}", e)))?;
                                        self.stack.push(Value::Str(out));
                                    }
                                }
                                other => {
                                    let v = value_to_jvalue(other)?;
                                    let out = serde_json::to_string(&v)
                                        .map_err(|e| BasilError(format!("JSON_STRINGIFY$: serialize failed: {}", e)))?;
                                    self.stack.push(Value::Str(out));
                                }
                            }
                        }
                        #[cfg(feature = "obj-csv")]
                        128 => { // CSV_PARSE$(csv_text$)
                            if argc != 1 { return Err(BasilError("CSV_PARSE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let mut rdr = ReaderBuilder::new()
                                .has_headers(true)
                                .from_reader(s.as_bytes());
                            let headers = rdr.headers()
                                .map_err(|e| BasilError(format!("CSV_PARSE$: read headers failed: {}", e)))?
                                .clone();
                            let mut rows: Vec<JValue> = Vec::new();
                            for rec in rdr.records() {
                                let rec = rec.map_err(|e| BasilError(format!("CSV_PARSE$: read record failed: {}", e)))?;
                                let mut obj = serde_json::Map::new();
                                for (i, field) in rec.iter().enumerate() {
                                    let key = headers.get(i).unwrap_or("").to_string();
                                    obj.insert(key, JValue::String(field.to_string()));
                                }
                                rows.push(JValue::Object(obj));
                            }
                            let out = serde_json::to_string(&rows)
                                .map_err(|e| BasilError(format!("CSV_PARSE$: serialize failed: {}", e)))?;
                            self.stack.push(Value::Str(out));
                        }
                        #[cfg(feature = "obj-csv")]
                        129 => { // CSV_WRITE$(rows_json$)
                            if argc != 1 { return Err(BasilError("CSV_WRITE$ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let rows: JValue = serde_json::from_str(&s)
                                .map_err(|e| BasilError(format!("CSV_WRITE$: invalid JSON: {}", e)))?;
                            let arr = rows.as_array().ok_or_else(|| BasilError("CSV_WRITE$: expected JSON array of objects".into()))?;
                            let mut headers: Vec<String> = Vec::new();
                            let mut seen = std::collections::HashSet::new();
                            if let Some(first) = arr.first().and_then(|v| v.as_object()) {
                                for k in first.keys() {
                                    headers.push(k.clone());
                                    seen.insert(k.clone());
                                }
                            }
                            for v in arr.iter() {
                                if let Some(obj) = v.as_object() {
                                    for k in obj.keys() {
                                        if !seen.contains(k) {
                                            headers.push(k.clone());
                                            seen.insert(k.clone());
                                        }
                                    }
                                }
                            }
                            let mut wtr = WriterBuilder::new().from_writer(vec![]);
                            wtr.write_record(headers.iter())
                                .map_err(|e| BasilError(format!("CSV_WRITE$: write headers failed: {}", e)))?;
                            for v in arr.iter() {
                                let obj = v.as_object().ok_or_else(|| BasilError("CSV_WRITE$: array items must be objects".into()))?;
                                let mut row: Vec<String> = Vec::with_capacity(headers.len());
                                for h in headers.iter() {
                                    let cell = match obj.get(h) {
                                        Some(JValue::String(s)) => s.clone(),
                                        Some(JValue::Number(n)) => n.to_string(),
                                        Some(JValue::Bool(b)) => if *b { "true".to_string() } else { "false".to_string() },
                                        Some(JValue::Null) => String::new(),
                                        Some(other) => serde_json::to_string(other).unwrap_or_default(),
                                        None => String::new(),
                                    };
                                    row.push(cell);
                                }
                                wtr.write_record(&row)
                                    .map_err(|e| BasilError(format!("CSV_WRITE$: write row failed: {}", e)))?;
                            }
                            let bytes = wtr.into_inner().map_err(|e| BasilError(format!("CSV_WRITE$: finalize failed: {}", e)))?;
                            let out = String::from_utf8(bytes).map_err(|e| BasilError(format!("CSV_WRITE$: utf8 failed: {}", e)))?;
                            self.stack.push(Value::Str(out));
                        }
                        #[cfg(feature = "obj-sqlite")]
                        130 => { // SQLITE_OPEN%(path$)
                            if argc != 1 { return Err(BasilError("SQLITE_OPEN% expects 1 argument".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let h = sqlite_utils::sqlite_open(&path);
                            self.stack.push(Value::Int(h));
                        }
                        #[cfg(feature = "obj-sqlite")]
                        131 => { // SQLITE_CLOSE(handle%)
                            if argc != 1 { return Err(BasilError("SQLITE_CLOSE expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            sqlite_utils::sqlite_close(h);
                            self.stack.push(Value::Null);
                        }
                        #[cfg(feature = "obj-sqlite")]
                        132 => { // SQLITE_EXEC%(handle%, sql$)
                            if argc != 2 { return Err(BasilError("SQLITE_EXEC% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let sql = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let n = sqlite_utils::sqlite_exec(h, &sql);
                            self.stack.push(Value::Int(n));
                        }
                        #[cfg(feature = "obj-sqlite")]
                        133 => { // SQLITE_QUERY2D$(handle%, sql$)
                            if argc != 2 { return Err(BasilError("SQLITE_QUERY2D$ expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let sql = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                            let v = sqlite_utils::sqlite_query2d(h, &sql)?;
                            self.stack.push(v);
                        }
                        #[cfg(feature = "obj-sqlite")]
                        134 => { // SQLITE_LAST_INSERT_ID%(handle%)
                            if argc != 1 { return Err(BasilError("SQLITE_LAST_INSERT_ID% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let id = sqlite_utils::sqlite_last_insert_id(h);
                            self.stack.push(Value::Int(id));
                        }
                        // --- DAW helpers ---
                        #[cfg(feature = "obj-daw")]
                        180 => { // DAW_STOP()
                            if argc != 0 { return Err(BasilError("DAW_STOP expects 0 arguments".into())); }
                            daw_utils::stop();
                            self.stack.push(Value::Str(String::new()));
                        }
                        #[cfg(feature = "obj-daw")]
                        181 => { // DAW_ERR$()
                            if argc != 0 { return Err(BasilError("DAW_ERR$ expects 0 arguments".into())); }
                            let s = daw_utils::get_err();
                            self.stack.push(Value::Str(s));
                        }
                        #[cfg(feature = "obj-daw")]
                        182 => { // AUDIO_RECORD%(inputSubstr$, outPath$, seconds%)
                            if argc != 3 { return Err(BasilError("AUDIO_RECORD% expects 3 arguments".into())); }
                            let a = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let b = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let secs = self.to_i64(&args[2])?;
                            let rc = daw_utils::audio_record(&a, &b, secs);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-daw")]
                        183 => { // AUDIO_PLAY%(outputSubstr$, filePath$)
                            if argc != 2 { return Err(BasilError("AUDIO_PLAY% expects 2 arguments".into())); }
                            let a = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let b = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let rc = daw_utils::audio_play(&a, &b);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-daw")]
                        184 => { // AUDIO_MONITOR%(inputSubstr$, outputSubstr$)
                            if argc != 2 { return Err(BasilError("AUDIO_MONITOR% expects 2 arguments".into())); }
                            let a = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let b = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let rc = daw_utils::audio_monitor(&a, &b);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-daw")]
                        185 => { // MIDI_CAPTURE%(portSubstr$, outJsonlPath$)
                            if argc != 2 { return Err(BasilError("MIDI_CAPTURE% expects 2 arguments".into())); }
                            let a = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let b = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let rc = daw_utils::midi_capture(&a, &b);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-daw")]
                        186 => { // SYNTH_LIVE%(midiPortSubstr$, outputSubstr$, poly%)
                            if argc != 3 { return Err(BasilError("SYNTH_LIVE% expects 3 arguments".into())); }
                            let a = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let b = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let poly = self.to_i64(&args[2])?;
                            let rc = daw_utils::synth_live(&a, &b, poly);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-daw")]
                        187 => { // DAW_RESET
                            if argc != 0 { return Err(BasilError("DAW_RESET expects 0 arguments".into())); }
                            daw_utils::reset();
                            self.stack.push(Value::Str(String::new()));
                        }
                        // --- Audio low-level ---
                        #[cfg(feature = "obj-audio")]
                        190 => { // AUDIO_OUTPUTS$[]
                            if argc != 0 { return Err(BasilError("AUDIO_OUTPUTS$ expects 0 arguments".into())); }
                            let v = audio_utils::audio_outputs();
                            let arr = VM::make_string_array(v);
                            self.stack.push(arr);
                        }
                        #[cfg(feature = "obj-audio")]
                        191 => { // AUDIO_INPUTS$[]
                            if argc != 0 { return Err(BasilError("AUDIO_INPUTS$ expects 0 arguments".into())); }
                            let v = audio_utils::audio_inputs();
                            let arr = VM::make_string_array(v);
                            self.stack.push(arr);
                        }
                        #[cfg(feature = "obj-audio")]
                        192 => { // AUDIO_DEFAULT_RATE%()
                            if argc != 0 { return Err(BasilError("AUDIO_DEFAULT_RATE% expects 0 arguments".into())); }
                            self.stack.push(Value::Int(audio_utils::audio_default_rate()));
                        }
                        #[cfg(feature = "obj-audio")]
                        193 => { // AUDIO_DEFAULT_CHANS%()
                            if argc != 0 { return Err(BasilError("AUDIO_DEFAULT_CHANS% expects 0 arguments".into())); }
                            self.stack.push(Value::Int(audio_utils::audio_default_chans()));
                        }
                        #[cfg(feature = "obj-audio")]
                        194 => { // AUDIO_OPEN_IN@(deviceSubstr$)
                            if argc != 1 { return Err(BasilError("AUDIO_OPEN_IN@ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            match audio_utils::audio_open_in(&s) {
                                Ok(h) => self.stack.push(Value::Int(h)),
                                Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); }
                            }
                        }
                        #[cfg(feature = "obj-audio")]
                        195 => { // AUDIO_OPEN_OUT@(deviceSubstr$)
                            if argc != 1 { return Err(BasilError("AUDIO_OPEN_OUT@ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            match audio_utils::audio_open_out(&s) {
                                Ok(h) => self.stack.push(Value::Int(h)),
                                Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); }
                            }
                        }
                        #[cfg(feature = "obj-audio")]
                        196 => { // AUDIO_START%(handle@)
                            if argc != 1 { return Err(BasilError("AUDIO_START% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match audio_utils::audio_start(h) { Ok(_) => 0, Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        197 => { // AUDIO_STOP%(handle@)
                            if argc != 1 { return Err(BasilError("AUDIO_STOP% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match audio_utils::audio_stop(h) { Ok(_) => 0, Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        198 => { // AUDIO_CLOSE%(handle@)
                            if argc != 1 { return Err(BasilError("AUDIO_CLOSE% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match audio_utils::audio_close(h) { Ok(_) => 0, Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        199 => { // AUDIO_RING_CREATE@(frames%)
                            if argc != 1 { return Err(BasilError("AUDIO_RING_CREATE@ expects 1 argument".into())); }
                            let n = self.to_i64(&args[0])?;
                            match audio_utils::ring_create(n) { Ok(h) => self.stack.push(Value::Int(h)), Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); } }
                        }
                        #[cfg(feature = "obj-audio")]
                        200 => { // AUDIO_RING_PUSH%(ring@, frames![])
                            if argc != 2 { return Err(BasilError("AUDIO_RING_PUSH% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let data: Vec<f32> = match &args[1] {
                                Value::Array(arr_rc) => {
                                    let arr = arr_rc.as_ref();
                                    let v = arr.data.borrow();
                                    let mut out = Vec::with_capacity(v.len());
                                    for e in v.iter() { match e { Value::Num(f)=> out.push(*f as f32), Value::Int(i)=> out.push(*i as f32), _=> out.push(0.0) } }
                                    out
                                }
                                other => return Err(BasilError(format!("AUDIO_RING_PUSH% expects array, got {}", self.type_of(other))))
                            };
                            let rc = match audio_utils::ring_push(h, &data) { Ok(n)=> n, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } -1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        201 => { // AUDIO_RING_POP%(ring@, OUT frames![])
                            if argc != 2 { return Err(BasilError("AUDIO_RING_POP% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let (len, arr_rc) = match &args[1] {
                                Value::Array(arr_rc) => { let len = arr_rc.data.borrow().len(); (len, Rc::clone(arr_rc)) },
                                other => return Err(BasilError(format!("AUDIO_RING_POP% expects array, got {}", self.type_of(other))))
                            };
                            let mut tmp = vec![0.0f32; len];
                            let n = match audio_utils::ring_pop(h, &mut tmp) { Ok(n)=> n as usize, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 0 } };
                            // fill array with popped values (remaining unchanged)
                            {
                                let mut data = arr_rc.data.borrow_mut();
                                for i in 0..n { data[i] = Value::Num(tmp[i] as f64); }
                            }
                            self.stack.push(Value::Int(n as i64));
                        }
                        #[cfg(feature = "obj-audio")]
                        202 => { // WAV_WRITER_OPEN@(path$, rate%, chans%)
                            if argc != 3 { return Err(BasilError("WAV_WRITER_OPEN@ expects 3 arguments".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let rate = self.to_i64(&args[1])?;
                            let chans = self.to_i64(&args[2])?;
                            match audio_utils::wav_writer_open(&path, rate, chans) { Ok(h)=> self.stack.push(Value::Int(h)), Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); } }
                        }
                        #[cfg(feature = "obj-audio")]
                        203 => { // WAV_WRITER_WRITE%(writer@, frames![])
                            if argc != 2 { return Err(BasilError("WAV_WRITER_WRITE% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let data: Vec<f32> = match &args[1] {
                                Value::Array(arr_rc) => {
                                    let arr = arr_rc.as_ref();
                                    let v = arr.data.borrow();
                                    let mut out = Vec::with_capacity(v.len());
                                    for e in v.iter() { match e { Value::Num(f)=> out.push(*f as f32), Value::Int(i)=> out.push(*i as f32), _=> out.push(0.0) } }
                                    out
                                }
                                other => return Err(BasilError(format!("WAV_WRITER_WRITE% expects array, got {}", self.type_of(other))))
                            };
                            let rc = match audio_utils::wav_writer_write(h, &data) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        204 => { // WAV_WRITER_CLOSE%(writer@)
                            if argc != 1 { return Err(BasilError("WAV_WRITER_CLOSE% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match audio_utils::wav_writer_close(h) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        205 => { // WAV_READ_ALL![](path$)
                            if argc != 1 { return Err(BasilError("WAV_READ_ALL![] expects 1 argument".into())); }
                            let path = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            let frames = audio_utils::wav_read_all(&path)?;
                            let mut data: Vec<Value> = Vec::with_capacity(frames.len());
                            for f in frames { data.push(Value::Num(f as f64)); }
                            let arr = Rc::new(ArrayObj { elem: ElemType::Num, dims: vec![data.len()], data: std::cell::RefCell::new(data) });
                            self.stack.push(Value::Array(arr));
                        }
                        #[cfg(feature = "obj-audio")]
                        206 => { // AUDIO_CONNECT_IN_TO_RING%(in@, ring@)
                            if argc != 2 { return Err(BasilError("AUDIO_CONNECT_IN_TO_RING% expects 2 arguments".into())); }
                            let in_h = self.to_i64(&args[0])?;
                            let ring_h = self.to_i64(&args[1])?;
                            let rc = match audio_utils::audio_connect_in_to_ring(in_h, ring_h) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        207 => { // AUDIO_CONNECT_RING_TO_OUT%(ring@, out@)
                            if argc != 2 { return Err(BasilError("AUDIO_CONNECT_RING_TO_OUT% expects 2 arguments".into())); }
                            let ring_h = self.to_i64(&args[0])?;
                            let out_h = self.to_i64(&args[1])?;
                            let rc = match audio_utils::audio_connect_ring_to_out(ring_h, out_h) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        220 => { // SYNTH_NEW@(rate%, poly%)
                            if argc != 2 { return Err(BasilError("SYNTH_NEW@ expects 2 arguments".into())); }
                            let rate = self.to_i64(&args[0])?;
                            let poly = self.to_i64(&args[1])?;
                            match audio_utils::synth_new(rate, poly) {
                                Ok(h) => self.stack.push(Value::Int(h)),
                                Err(e) => { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); }
                            }
                        }
                        #[cfg(feature = "obj-audio")]
                        221 => { // SYNTH_NOTE_ON%(synth@, note%, vel%)
                            if argc != 3 { return Err(BasilError("SYNTH_NOTE_ON% expects 3 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let note = self.to_i64(&args[1])?;
                            let vel = self.to_i64(&args[2])?;
                            let rc = match audio_utils::synth_note_on(h, note, vel) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        222 => { // SYNTH_NOTE_OFF%(synth@, note%)
                            if argc != 2 { return Err(BasilError("SYNTH_NOTE_OFF% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let note = self.to_i64(&args[1])?;
                            let rc = match audio_utils::synth_note_off(h, note) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-audio")]
                        223 => { // SYNTH_RENDER%(synth@, OUT frames![])
                            if argc != 2 { return Err(BasilError("SYNTH_RENDER% expects 2 arguments".into())); }
                            let h = self.to_i64(&args[0])?;
                            let (len, arr_rc) = match &args[1] {
                                Value::Array(arr_rc) => { let len = arr_rc.data.borrow().len(); (len, Rc::clone(arr_rc)) },
                                other => return Err(BasilError(format!("SYNTH_RENDER% expects array, got {}", self.type_of(other))))
                            };
                            let mut tmp = vec![0.0f32; len];
                            let n = match audio_utils::synth_render(h, &mut tmp) { Ok(n)=> n as usize, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 0 } };
                            {
                                let mut data = arr_rc.data.borrow_mut();
                                for i in 0..n { data[i] = Value::Num(tmp[i] as f64); }
                            }
                            self.stack.push(Value::Int(n as i64));
                        }
                        #[cfg(feature = "obj-audio")]
                        224 => { // SYNTH_DELETE%(synth@)
                            if argc != 1 { return Err(BasilError("SYNTH_DELETE% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match audio_utils::synth_delete(h) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        // --- MIDI ---
                        #[cfg(feature = "obj-midi")]
                        210 => { // MIDI_PORTS$[]
                            if argc != 0 { return Err(BasilError("MIDI_PORTS$ expects 0 arguments".into())); }
                            let v = midi_utils::midi_ports();
                            let arr = VM::make_string_array(v);
                            self.stack.push(arr);
                        }
                        #[cfg(feature = "obj-midi")]
                        211 => { // MIDI_OPEN_IN@(portSubstr$)
                            if argc != 1 { return Err(BasilError("MIDI_OPEN_IN@ expects 1 argument".into())); }
                            let s = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other)};
                            match midi_utils::midi_open_in(&s) { Ok(h)=> self.stack.push(Value::Int(h)), Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } self.stack.push(Value::Int(-1)); } }
                        }
                        #[cfg(feature = "obj-midi")]
                        212 => { // MIDI_POLL%(in@)
                            if argc != 1 { return Err(BasilError("MIDI_POLL% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let n = match midi_utils::midi_poll(h) { Ok(n)=>n, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } -1 } };
                            self.stack.push(Value::Int(n));
                        }
                        #[cfg(feature = "obj-midi")]
                        213 => { // MIDI_GET_EVENT$[](in@)
                            if argc != 1 { return Err(BasilError("MIDI_GET_EVENT$[] expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let (s, d1, d2) = midi_utils::midi_get_event(h)?;
                            let arr = VM::make_string_array(vec![s.to_string(), d1.to_string(), d2.to_string()]);
                            self.stack.push(arr);
                        }
                        #[cfg(feature = "obj-midi")]
                        214 => { // MIDI_CLOSE%(in@)
                            if argc != 1 { return Err(BasilError("MIDI_CLOSE% expects 1 argument".into())); }
                            let h = self.to_i64(&args[0])?;
                            let rc = match midi_utils::midi_close(h) { Ok(_)=>0, Err(e)=> { #[cfg(feature="obj-daw")] { daw_utils::set_err(format!("{}", e)); } 1 } };
                            self.stack.push(Value::Int(rc));
                        }
                        138 => { // INTERNAL: STR2D_TO_ARRAY$(rowsxcols)
                            if argc != 1 { return Err(BasilError("internal builtin 138 expects 1 argument".into())); }
                            match &args[0] {
                                Value::StrArray2D { rows, cols, data } => {
                                    // build 2D string array with dims [rows, cols]
                                    let dims = vec![*rows, *cols];
                                    let mut arr_data: Vec<Value> = Vec::with_capacity(data.len());
                                    for s in data.iter() { arr_data.push(Value::Str(s.clone())); }
                                    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: std::cell::RefCell::new(arr_data) });
                                    self.stack.push(Value::Array(arr));
                                }
                                other => return Err(BasilError(format!("builtin 138 expects StrArray2D, got {}", self.type_of(other)))),
                            }
                        }
                        139 => { // ARRAY_ROWS%(arr$())
                            if argc != 1 { return Err(BasilError("ARRAY_ROWS% expects 1 argument".into())); }
                            match &args[0] {
                                Value::Array(rc) => {
                                    let a = rc.as_ref();
                                    if a.dims.len() != 2 { return Err(BasilError("ARRAY_ROWS%: expected 2-D array".into())); }
                                    self.stack.push(Value::Int(a.dims[0] as i64));
                                }
                                _ => return Err(BasilError("ARRAY_ROWS%: expected array".into())),
                            }
                        }
                        140 => { // ARRAY_COLS%(arr$())
                            if argc != 1 { return Err(BasilError("ARRAY_COLS% expects 1 argument".into())); }
                            match &args[0] {
                                Value::Array(rc) => {
                                    let a = rc.as_ref();
                                    if a.dims.len() != 2 { return Err(BasilError("ARRAY_COLS%: expected 2-D array".into())); }
                                    self.stack.push(Value::Int(a.dims[1] as i64));
                                }
                                _ => return Err(BasilError("ARRAY_COLS%: expected array".into())),
                            }
                        }
                        #[cfg(feature = "obj-term")]
                        230 => { // CLS / CLEAR / HOME
                            if argc != 0 { return Err(BasilError("CLS expects 0 arguments".into())); }
                            let rc = basil_objects::term::cls();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        231 => { // LOCATE(x%, y%)
                            if argc != 2 { return Err(BasilError("LOCATE expects 2 arguments".into())); }
                            let rc = basil_objects::term::locate(&args[0], &args[1]);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        232 => { // COLOR(fg, bg)
                            if argc != 2 { return Err(BasilError("COLOR expects 2 arguments".into())); }
                            let rc = basil_objects::term::color(&args[0], &args[1]);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        233 => { // COLOR_RESET
                            if argc != 0 { return Err(BasilError("COLOR_RESET expects 0 arguments".into())); }
                            let rc = basil_objects::term::color_reset();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        234 => { // ATTR(bold%, underline%, reverse%)
                            if argc != 3 { return Err(BasilError("ATTR expects 3 arguments".into())); }
                            let rc = basil_objects::term::attr(&args[0], &args[1], &args[2]);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        235 => { // ATTR_RESET
                            if argc != 0 { return Err(BasilError("ATTR_RESET expects 0 arguments".into())); }
                            let rc = basil_objects::term::attr_reset();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        236 => { // CURSOR_SAVE
                            if argc != 0 { return Err(BasilError("CURSOR_SAVE expects 0 arguments".into())); }
                            let rc = basil_objects::term::cursor_save();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        237 => { // CURSOR_RESTORE
                            if argc != 0 { return Err(BasilError("CURSOR_RESTORE expects 0 arguments".into())); }
                            let rc = basil_objects::term::cursor_restore();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        238 => { // TERM_COLS%()
                            if argc != 0 { return Err(BasilError("TERM_COLS% expects 0 arguments".into())); }
                            let n = basil_objects::term::term_cols();
                            self.stack.push(Value::Int(n));
                        }
                        #[cfg(feature = "obj-term")]
                        239 => { // TERM_ROWS%()
                            if argc != 0 { return Err(BasilError("TERM_ROWS% expects 0 arguments".into())); }
                            let n = basil_objects::term::term_rows();
                            self.stack.push(Value::Int(n));
                        }
                        #[cfg(feature = "obj-term")]
                        241 => { // CURSOR_HIDE
                            if argc != 0 { return Err(BasilError("CURSOR_HIDE expects 0 arguments".into())); }
                            let rc = basil_objects::term::cursor_hide();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        242 => { // CURSOR_SHOW
                            if argc != 0 { return Err(BasilError("CURSOR_SHOW expects 0 arguments".into())); }
                            let rc = basil_objects::term::cursor_show();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        243 => { // TERM_ERR$()
                            if argc != 0 { return Err(BasilError("TERM_ERR$ expects 0 arguments".into())); }
                            let s = basil_objects::term::term_err();
                            self.stack.push(Value::Str(s));
                        }
                        #[cfg(feature = "obj-term")]
                        244 => { // TERM.INIT
                            if argc != 0 { return Err(BasilError("TERM.INIT expects 0 arguments".into())); }
                            let rc = basil_objects::term::term_init();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        245 => { // TERM.END
                            if argc != 0 { return Err(BasilError("TERM.END expects 0 arguments".into())); }
                            let rc = basil_objects::term::term_end();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        246 => { // TERM.RAW ON|OFF
                            if argc != 1 { return Err(BasilError("TERM.RAW expects 1 argument (ON/OFF or 0/1)".into())); }
                            let rc = basil_objects::term::term_raw(&args[0]);
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        247 => { // ALTSCREEN_ON
                            if argc != 0 { return Err(BasilError("ALTSCREEN_ON expects 0 arguments".into())); }
                            let rc = basil_objects::term::altscreen_on();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        248 => { // ALTSCREEN_OFF
                            if argc != 0 { return Err(BasilError("ALTSCREEN_OFF expects 0 arguments".into())); }
                            let rc = basil_objects::term::altscreen_off();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        249 => { // TERM.FLUSH
                            if argc != 0 { return Err(BasilError("TERM.FLUSH expects 0 arguments".into())); }
                            let rc = basil_objects::term::term_flush();
                            self.stack.push(Value::Int(rc));
                        }
                        #[cfg(feature = "obj-term")]
                        250 => { // TERM.POLLKEY$()
                            if argc != 0 { return Err(BasilError("TERM.POLLKEY$ expects 0 arguments".into())); }
                            let s = basil_objects::term::term_pollkey_s();
                            self.stack.push(Value::Str(s));
                        }
                        251 => { // MAKE_LIST([...])
                            // args are already in call order
                            let list = Rc::new(std::cell::RefCell::new(args));
                            self.stack.push(Value::List(list));
                        }
                        252 => { // MAKE_DICT({key: val, ...})
                            if argc % 2 != 0 { return Err(BasilError("MAKE_DICT expects an even number of arguments (key,value pairs)".into())); }
                            let mut map: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
                            let mut i = 0usize;
                            while i < args.len() {
                                let key = match &args[i] { Value::Str(s) => s.clone(), other => return Err(BasilError(format!("Dictionary key must be string, got {}", self.type_of(other)))) };
                                let val = args[i+1].clone();
                                map.insert(key, val);
                                i += 2;
                            }
                            self.stack.push(Value::Dict(Rc::new(std::cell::RefCell::new(map))));
                        }
                        253 => { // INDEX GET: obj[idx]
                            if argc != 2 { return Err(BasilError("INDEX[] get expects 2 arguments".into())); }
                            let target = &args[0];
                            let index = &args[1];
                            match target {
                                Value::List(rc) => {
                                    let idx = self.to_i64(index)?;
                                    if idx <= 0 { return Err(BasilError(format!("List index out of range: {}", idx))); }
                                    let idx0 = (idx - 1) as usize;
                                    let v = rc.borrow();
                                    if idx0 >= v.len() { return Err(BasilError(format!("List index out of range: {}", idx))); }
                                    self.stack.push(v[idx0].clone());
                                }
                                Value::Dict(rc) => {
                                    let key = match index { Value::Str(s) => s.clone(), other => return Err(BasilError(format!("Dictionary key must be string, got {}", self.type_of(other)))) };
                                    let m = rc.borrow();
                                    if let Some(v) = m.get(&key) { self.stack.push(v.clone()); }
                                    else { return Err(BasilError(format!("Dictionary missing key: \"{}\"", key))); }
                                }
                                _ => { return Err(BasilError("Attempted [] on a non-list/dict value.".into())); }
                            }
                        }
                        254 => { // INDEX SET: obj[idx] = value
                            if argc != 3 { return Err(BasilError("INDEX[] set expects 3 arguments".into())); }
                            let target = &args[0];
                            let index = &args[1];
                            let value = args[2].clone();
                            match target {
                                Value::List(rc) => {
                                    let idx = self.to_i64(index)?;
                                    if idx <= 0 { return Err(BasilError(format!("List index out of range: {}", idx))); }
                                    let idx0 = (idx - 1) as usize;
                                    let mut v = rc.borrow_mut();
                                    if idx0 >= v.len() { return Err(BasilError(format!("List index out of range: {}", idx))); }
                                    v[idx0] = value;
                                    self.stack.push(Value::Null);
                                }
                                Value::Dict(rc) => {
                                    let key = match index { Value::Str(s) => s.clone(), other => return Err(BasilError(format!("Dictionary key must be string, got {}", self.type_of(other)))) };
                                    rc.borrow_mut().insert(key, value);
                                    self.stack.push(Value::Null);
                                }
                                _ => { return Err(BasilError("Attempted [] on a non-list/dict value.".into())); }
                            }
                        }
                        _ => return Err(BasilError(format!("unknown builtin id {}", bid))),
                    }
                }

                Op::Halt => {
                    if self.frames.len() == 1 { break; }
                    else { return Err(BasilError("HALT inside function".into())); }
                }
               // other => { return Err(BasilError(format!("unhandled opcode {:?}", other))); }
            }
        }
        if let Some(dbg) = &self.debugger { dbg.emit(debug::DebugEvent::Exited); }
        if !self.gosub_stack.is_empty() {
            eprintln!("warning: program terminated with {} pending GOSUB frames (missing RETURN?)", self.gosub_stack.len());
        }
        Ok(())
    }

    // Debugger integration API
    pub fn set_debugger(&mut self, dbg: Arc<debug::Debugger>) { self.debugger = Some(dbg); }
    pub fn with_debugger(mut self, dbg: Arc<debug::Debugger>) -> Self { self.debugger = Some(dbg); self }
    pub fn get_call_stack(&self) -> Vec<debug::FrameInfo> {
        let file = self.script_path.clone().unwrap_or_else(|| "<unknown>".into());
        let line = self.current_line as usize;
        vec![debug::FrameInfo { function: "<top>".into(), file, line }]
    }
    pub fn get_scopes(&self) -> Vec<debug::Scope> {
        let mut globals: Vec<debug::Variable> = Vec::new();
        for (i, name) in self.global_names.iter().enumerate() {
            let v = self.globals.get(i).cloned().unwrap_or(Value::Null);
            let tn = match &v {
                Value::Null => "NULL".to_string(),
                Value::Bool(_) => "BOOL".to_string(),
                Value::Num(_) => "FLOAT".to_string(),
                Value::Int(_) => "INTEGER".to_string(),
                Value::Str(_) => "STRING".to_string(),
                Value::Func(_) => "FUNCTION".to_string(),
                Value::Array(_) => "ARRAY".to_string(),
                Value::Object(_) => "OBJECT".to_string(),
                Value::List(_) => "LIST".to_string(),
                Value::Dict(_) => "DICT".to_string(),
                Value::StrArray2D { .. } => "STRARRAY2D".to_string(),
            };
            globals.push(debug::Variable { name: name.clone(), value: format!("{}", v), type_name: tn });
        }
        let scopes = vec![debug::Scope { name: "Globals".into(), vars: globals }];
        scopes
    }

    // --- helpers ---

    fn read_op(&mut self) -> Result<Op> {
        let f = self.cur();
        let byte = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))?;
        f.ip += 1;
        let op = match byte {
            1=>Op::Const, 2=>Op::LoadGlobal, 3=>Op::StoreGlobal,
            11=>Op::LoadLocal, 12=>Op::StoreLocal,
            20=>Op::Add, 21=>Op::Sub, 22=>Op::Mul, 23=>Op::Div, 24=>Op::Neg, 25=>Op::Mod,
            30=>Op::Eq, 31=>Op::Ne, 32=>Op::Lt, 33=>Op::Le, 34=>Op::Gt, 35=>Op::Ge,
            40=>Op::Jump, 41=>Op::JumpIfFalse, 42=>Op::JumpBack,
            50=>Op::Call, 51=>Op::Ret,
            60=>Op::Print, 61=>Op::Pop, 62=>Op::ToInt, 63=>Op::Builtin, 64=>Op::SetLine,
            70=>Op::ArrMake, 71=>Op::ArrGet, 72=>Op::ArrSet,
            80=>Op::NewObj, 81=>Op::GetProp, 82=>Op::SetProp, 83=>Op::CallMethod, 84=>Op::DescribeObj,
            90=>Op::EnumNew, 91=>Op::EnumMoveNext, 92=>Op::EnumCurrent, 93=>Op::EnumDispose,
            100=>Op::NewClass, 101=>Op::GetMember, 102=>Op::SetMember, 103=>Op::CallMember, 104=>Op::DestroyInstance,
            105=>Op::ExecString, 106=>Op::EvalString,
            110=>Op::Gosub, 111=>Op::GosubBack, 112=>Op::GosubRet, 113=>Op::GosubPop,
            120=>Op::TryPush, 121=>Op::TryPop, 122=>Op::Raise, 123=>Op::Reraise,
            255=>Op::Halt,
            _ => return Err(BasilError(format!("bad opcode {}", byte))),
        };
        Ok(op)
    }
    fn read_u8(&mut self) -> Result<u8> {
        let f = self.cur();
        let b = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))?;
        f.ip += 1; Ok(b)
    }
    fn read_u16(&mut self) -> Result<u16> {
        let f = self.cur();
        let lo = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))? as u16;
        let hi = *f.chunk.code.get(f.ip+1).ok_or_else(|| BasilError("ip out of range".into()))? as u16;
        f.ip += 2; Ok(lo | (hi<<8))
    }
    fn pop(&mut self) -> Result<Value> { self.stack.pop().ok_or_else(|| BasilError("stack underflow".into())) }

    fn as_num(&self, v: Value) -> Result<f64> {
        match v {
            Value::Num(n) => Ok(n),
            Value::Int(i) => Ok(i as f64),
            Value::Bool(b) => Ok(if b { 1.0 } else { 0.0 }),
            _ => Err(BasilError("expected number".into())),
        }
    }
    fn bin_num<F: Fn(f64,f64)->f64>(&mut self, f: F) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        let b = self.as_num(b)?; let a = self.as_num(a)?;
        self.stack.push(Value::Num(f(a,b))); Ok(())
    }
    fn bin_num_cmp<F: Fn(f64,f64)->bool>(&mut self, f: F) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        let b = self.as_num(b)?; let a = self.as_num(a)?;
        self.stack.push(Value::Bool(f(a,b))); Ok(())
    }

    fn bin_eq(&mut self) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        // Numeric-aware equality for Int/Num/Bool; fall back to structural equality otherwise
        let res = match (&a, &b) {
            (Value::Num(_)|Value::Int(_)|Value::Bool(_), Value::Num(_)|Value::Int(_)|Value::Bool(_)) => {
                let an = self.as_num(a)?; let bn = self.as_num(b)?; an == bn
            }
            _ => a == b,
        };
        self.stack.push(Value::Bool(res));
        Ok(())
    }

    fn bin_ne(&mut self) -> Result<()> {
        let b = self.pop()?; let a = self.pop()?;
        // Numeric-aware inequality for Int/Num/Bool; fall back to structural inequality otherwise
        let res = match (&a, &b) {
            (Value::Num(_)|Value::Int(_)|Value::Bool(_), Value::Num(_)|Value::Int(_)|Value::Bool(_)) => {
                let an = self.as_num(a)?; let bn = self.as_num(b)?; an != bn
            }
            _ => a != b,
        };
        self.stack.push(Value::Bool(res));
        Ok(())
    }

    fn type_of(&self, v: &Value) -> String {
        match v {
            Value::Null => "NULL".to_string(),
            Value::Bool(_) => "BOOL".to_string(),
            Value::Num(_) => "FLOAT".to_string(),
            Value::Int(_) => "INTEGER".to_string(),
            Value::Str(_) => "STRING".to_string(),
            Value::Func(_) => "FUNCTION".to_string(),
            Value::Array(arr_rc) => {
                let arr = arr_rc.as_ref();
                let base = match &arr.elem {
                    ElemType::Num => "FLOAT".to_string(),
                    ElemType::Int => "INTEGER".to_string(),
                    ElemType::Str => "STRING".to_string(),
                    ElemType::Obj(Some(tn)) => tn.clone(),
                    ElemType::Obj(None) => "OBJECT".to_string(),
                };
                format!("{}[]", base)
            }
            Value::Object(rc) => rc.borrow().type_name().to_string(),
            Value::List(_) => "LIST".to_string(),
            Value::Dict(_) => "DICT".to_string(),
            Value::StrArray2D { .. } => "STRING[][]".to_string(),
        }
    }

    fn resolve_class_candidates(&self, fname: &str) -> Vec<std::path::PathBuf> {
        use std::path::{Path, PathBuf};
        let mut out: Vec<PathBuf> = Vec::new();
        let p = Path::new(fname);
        let is_abs = p.is_absolute();
        let has_dir = p.components().count() > 1;
        let mut bases: Vec<PathBuf> = Vec::new();
        if is_abs || has_dir {
            bases.push(PathBuf::from(fname));
        } else {
            if let Some(sp) = &self.script_path {
                let dir = Path::new(sp).parent().unwrap_or(Path::new("."));
                bases.push(dir.join(fname));
            }
            bases.push(PathBuf::from(fname));
        }
        for b in bases {
            if b.extension().is_none() {
                out.push(b.with_extension("basil"));
                out.push(b.with_extension("basilx"));
            } else {
                let ext = b.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                if ext == "basilx" {
                    out.push(b.clone());
                } else if ext == "basil" {
                    out.push(b.clone());
                    out.push(b.with_extension("basilx"));
                } else {
                    out.push(b.clone());
                }
            }
        }
        // dedupe while preserving order
        let mut seen = std::collections::HashSet::new();
        out.into_iter().filter(|pb| seen.insert(pb.clone())).collect()
    }

    fn load_class_program(&self, fname: &str) -> Result<(BCProgram, String)> {
        use std::fs;
        for cand in self.resolve_class_candidates(fname) {
            let exists = fs::metadata(&cand).is_ok();
            if !exists { continue; }
            let ext = cand.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if ext == "basilx" {
                let bytes = fs::read(&cand).map_err(|e| BasilError(format!("Failed to read {}: {}", cand.display(), e)))?;
                let prog = deserialize_program(&bytes).map_err(|_| BasilError("Bad .basilx file".into()))?;
                return Ok((prog, cand.to_string_lossy().to_string()));
            } else {
                // Treat others as .basil source
                let src = fs::read_to_string(&cand).map_err(|e| BasilError(format!("Failed to read {}: {}", cand.display(), e)))?;
                let ast = parse_basil(&src)?;
                let prog = compile_basil(&ast)?;
                return Ok((prog, cand.to_string_lossy().to_string()));
            }
        }
        Err(BasilError("Class file not found.".into()))
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Num(n) => *n != 0.0,
        Value::Int(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Func(_) => true,
        Value::Array(_) => true,
        Value::Object(_) => true,
        Value::StrArray2D { rows, cols, data } => (*rows > 0) && (*cols > 0) && (!data.is_empty()),
        Value::List(rc) => !rc.borrow().is_empty(),
        Value::Dict(rc) => !rc.borrow().is_empty(),
    }
}


impl Drop for VM {
    fn drop(&mut self) {
        let keys: Vec<i64> = self.file_table.keys().copied().collect();
        for k in keys { let _ = self.fh_close(k); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_deterministic() {
        let mut a = MockInputProvider::new(123456);
        let mut b = MockInputProvider::new(123456);
        for _ in 0..50 {
            assert_eq!(a.read_line(), b.read_line());
            assert_eq!(a.read_char(), b.read_char());
        }
    }

    #[test]
    fn mock_provider_values_in_set() {
        let mut m = MockInputProvider::new(1);
        for _ in 0..50 {
            let s = m.read_line();
            assert!(s == "" || s == "Y" || s == "N" || s == "0" || s == "1" || s == "9");
            let c = m.read_char().unwrap();
            assert!(c == '\r' || c == 'Y' || c == 'N' || c == '0' || c == '1' || c == '9');
        }
    }
}
