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
use std::io::{self, Write, Read, Seek, SeekFrom};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use std::env;
use std::time::Duration;
use crossterm::event::{read, Event, KeyEvent, KeyCode};
use crossterm::event::poll;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

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

pub struct VM {
    frames: Vec<Frame>,
    stack: Vec<Value>,
    globals: Vec<Value>,
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
        Self {
            frames: vec![frame],
            stack: Vec::new(),
            globals,
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
        }
    }

    pub fn new_with_test(p: BCProgram, mock: MockInputProvider, trace: bool, script_path: Option<String>, comments_map: Option<HashMap<u32, Vec<String>>>, max_mocked_inputs: Option<usize>) -> Self {
        let mut vm = VM::new(p);
        vm.test_mode = true;
        vm.trace = trace;
        vm.script_path = script_path;
        vm.comments_map = comments_map;
        vm.max_mocked_inputs = max_mocked_inputs;
        vm.mock = Some(mock);
        vm
    }

    pub fn current_line(&self) -> u32 { self.current_line }

    // Provide script path so CLASS() can resolve relative file names
    pub fn set_script_path(&mut self, p: String) { self.script_path = Some(p); }

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

                Op::Print => { let v = self.pop()?; print!("{}", v); }
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
                            let file = opts.open(&path).map_err(|e| BasilError(format!("FOPEN {}: {}", path, e)))?;
                            let fh = self.next_fh; self.next_fh += 1;
                            let entry = FileHandleEntry { file, text, readable, writable, owner_depth: self.frames.len() };
                            self.file_table.insert(fh, entry);
                            self.stack.push(Value::Int(fh));
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
        Ok(())
    }

    // --- helpers ---

    fn read_op(&mut self) -> Result<Op> {
        let f = self.cur();
        let byte = *f.chunk.code.get(f.ip).ok_or_else(|| BasilError("ip out of range".into()))?;
        f.ip += 1;
        let op = match byte {
            1=>Op::Const, 2=>Op::LoadGlobal, 3=>Op::StoreGlobal,
            11=>Op::LoadLocal, 12=>Op::StoreLocal,
            20=>Op::Add, 21=>Op::Sub, 22=>Op::Mul, 23=>Op::Div, 24=>Op::Neg,
            30=>Op::Eq, 31=>Op::Ne, 32=>Op::Lt, 33=>Op::Le, 34=>Op::Gt, 35=>Op::Ge,
            40=>Op::Jump, 41=>Op::JumpIfFalse, 42=>Op::JumpBack,
            50=>Op::Call, 51=>Op::Ret,
            60=>Op::Print, 61=>Op::Pop, 62=>Op::ToInt, 63=>Op::Builtin, 64=>Op::SetLine,
            70=>Op::ArrMake, 71=>Op::ArrGet, 72=>Op::ArrSet,
            80=>Op::NewObj, 81=>Op::GetProp, 82=>Op::SetProp, 83=>Op::CallMethod, 84=>Op::DescribeObj,
            90=>Op::EnumNew, 91=>Op::EnumMoveNext, 92=>Op::EnumCurrent, 93=>Op::EnumDispose,
            100=>Op::NewClass, 101=>Op::GetMember, 102=>Op::SetMember, 103=>Op::CallMember, 104=>Op::DestroyInstance,
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
