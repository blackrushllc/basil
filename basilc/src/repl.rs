use std::collections::{HashMap, BTreeMap};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use basil_parser::parse;
use basil_compiler::compile;
use basil_bytecode::Value;
use basil_vm::VM;

use crate::template::{precompile_template, Directives};
use basil_bytecode::{serialize_program, deserialize_program};
use std::time::UNIX_EPOCH;

#[derive(Default)]
pub struct SessionSettings {
    pub show_backtraces: bool,
}

pub struct Session {
    // name  value (case-insensitive names per BASIC tradition; we store canonical as compiled)
    globals: HashMap<String, Value>,
    order: Vec<String>,
    // track origin file(s) for each symbol
    origins: HashMap<String, Vec<String>>,
    pub history: Vec<String>,
    #[allow(dead_code)]
    pub next_snippet_id: usize,
    pub settings: SessionSettings,
    script_path: Option<String>,
}

impl Session {
    pub fn new(settings: SessionSettings) -> Self {
        Self { globals: HashMap::new(), order: Vec::new(), origins: HashMap::new(), history: Vec::new(), next_snippet_id: 0, settings, script_path: None }
    }

    pub fn run_program(&mut self, path: &str) -> Result<(), String> {
        // Largely adapted from cmd_run
        let src = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))?;
        let looks_like_template = src.contains("<?");
        let pre = if looks_like_template {
            precompile_template(&src).map_err(|e| format!("template error: {}", e))?
        } else {
            crate::template::PrecompileResult { basil_source: src.clone(), directives: Directives::default() }
        };
        let meta = fs::metadata(path).map_err(|e| format!("stat {}: {}", path, e))?;
        let source_size = meta.len();
        let source_mtime_ns: u64 = meta.modified().ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let templating_used = src.contains("<?");
        let flags: u32 = (if pre.directives.short_tags_on { 1u32 } else { 0u32 })
                       | (if templating_used { 2u32 } else { 0u32 });
        let mut cache_path = PathBuf::from(path);
        cache_path.set_extension("basilx");
        let mut program_opt: Option<basil_bytecode::Program> = None;
        if let Ok(bytes) = fs::read(&cache_path) {
            if bytes.len() > 32 && &bytes[0..4] == b"BSLX" {
                let fmt_ver = u32::from_le_bytes([bytes[4],bytes[5],bytes[6],bytes[7]]);
                let abi_ver = u32::from_le_bytes([bytes[8],bytes[9],bytes[10],bytes[11]]);
                let flags_stored = u32::from_le_bytes([bytes[12],bytes[13],bytes[14],bytes[15]]);
                let sz = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
                let mt = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
                if fmt_ver == 3 && abi_ver == 1 && flags_stored == flags && sz == source_size && mt == source_mtime_ns {
                    let prog_bytes = &bytes[32..];
                    if let Ok(p) = deserialize_program(prog_bytes) { program_opt = Some(p); }
                }
            }
        }
        let program = if let Some(p) = program_opt { p } else {
            let ast = parse(&pre.basil_source).map_err(|e| format!("parse error: {}", e))?;
            let prog = compile(&ast).map_err(|e| format!("compile error: {}", e))?;
            let body = serialize_program(&prog);
            let mut hdr = Vec::with_capacity(32 + body.len());
            hdr.extend_from_slice(b"BSLX");
            hdr.extend_from_slice(&3u32.to_le_bytes()); // fmt ver
            hdr.extend_from_slice(&1u32.to_le_bytes()); // abi ver
            hdr.extend_from_slice(&flags.to_le_bytes());
            hdr.extend_from_slice(&source_size.to_le_bytes());
            hdr.extend_from_slice(&source_mtime_ns.to_le_bytes());
            hdr.extend_from_slice(&body);
            let tmp = cache_path.with_extension("basilx.tmp");
            if let Ok(mut f) = fs::File::create(&tmp) {
                let _ = std::io::Write::write_all(&mut f, &hdr);
                let _ = f.sync_all();
                let _ = fs::rename(&tmp, &cache_path);
            }
            prog
        };
        let mut vm = VM::new(program);
        vm.set_script_path(path.to_string());
        self.script_path = Some(path.to_string());
        // Seed known globals into this VM so the program can reference preloaded names
        for name in vm.globals_snapshot().0.iter() {
            if let Some(v) = self.globals.get(name) {
                let _ = vm.set_global_by_name(name, v.clone());
            }
        }
        vm.run().map_err(|e| {
            let line = vm.current_line();
            if self.settings.show_backtraces { format!("runtime error at line {}: {}", line, e) }
            else { format!("runtime error: {}", e) }
        })?;
        let (names, values) = vm.globals_snapshot();
        self.merge_globals(&names, &values, Some(path));
        Ok(())
    }

    fn merge_globals(&mut self, names: &[String], values: &[Value], origin: Option<&str>) {
        for (i, name) in names.iter().enumerate() {
            let val = values.get(i).cloned().unwrap_or(Value::Null);
            if !self.globals.contains_key(name) { self.order.push(name.clone()); }
            self.globals.insert(name.clone(), val);
            if let Some(src) = origin {
                let e = self.origins.entry(name.clone()).or_default();
                if !e.iter().any(|s| s.eq_ignore_ascii_case(src)) { e.push(src.to_string()); }
            }
        }
    }

    pub fn list_globals(&self, filter: Option<&str>) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let filt = filter.map(|s| s.to_ascii_uppercase());
        for name in &self.order {
            if let Some(fu) = &filt {
                if !name.to_ascii_uppercase().contains(fu) { continue; }
            }
            if let Some(v) = self.globals.get(name) {
                out.push((name.clone(), format!("{}", v)));
            }
        }
        out
    }

    pub fn eval_snippet(&mut self, src: &str) -> Result<(), String> {
        let ast = parse(src).map_err(|e| format!("parse error: {}", e))?;
        // Detect single expression: ignoring line markers
        let mut real_stmts = Vec::new();
        for s in &ast { if !matches!(s, basil_ast::Stmt::Line(_)) { real_stmts.push(s.clone()); } }
        let ast2 = if real_stmts.len() == 1 {
            if let basil_ast::Stmt::ExprStmt(e) = real_stmts.remove(0) {
                vec![basil_ast::Stmt::Print { expr: e }]
            } else { ast.clone() }
        } else { ast.clone() };
        let prog = compile(&ast2).map_err(|e| format!("compile error: {}", e))?;
        let mut vm = VM::new(prog);
        if let Some(p) = &self.script_path { vm.set_script_path(p.clone()); }
        // Seed known globals into this snippet VM
        for name in vm.globals_snapshot().0.iter() { // get names cheaply
            if let Some(v) = self.globals.get(name) {
                let _ = vm.set_global_by_name(name, v.clone());
            }
        }
        if let Err(e) = vm.run() {
            let line = vm.current_line();
            let msg = if self.settings.show_backtraces { format!("runtime error at line {}: {}", line, e) }
                      else { format!("runtime error: {}", e) };
            return Err(msg);
        }
        let (names, values) = vm.globals_snapshot();
        self.merge_globals(&names, &values, Some("<repl>"));
        Ok(())
    }
}

pub fn start_repl(mut sess: Session, maybe_path: Option<String>) {
    if let Some(p) = maybe_path {
        if let Err(e) = sess.run_program(&p) {
            eprintln!("Error running program: {}", e);
            // continue into REPL anyway
        }
    }

    // Try rustyline; fallback to stdio
    let mut rl: Option<rustyline::DefaultEditor> = rustyline::DefaultEditor::new().ok();

    // Old-school banner
    println!("BASIL - A BASIC Bytecode Interpreter and Compiler");
    println!("Copyright (C) Blackrush LLC - All Rights Reserved.");
    println!("Open Source Software under MIT License");

    // Line-numbered program buffer and last-used filename
    let mut program_buf: BTreeMap<usize, String> = BTreeMap::new();
    let mut last_file: String = "_.basil".to_string();

    let mut buffer = String::new();

    loop {
        // Old-school prompt: print an OK banner before each new input when not buffering a snippet
        if buffer.is_empty() {
            print!("\nOK\n\n");
            let _ = io::stdout().flush();
        }
        let line = if let Some(editor) = rl.as_mut() {
            match editor.readline("") { Ok(l)=>{ if !l.trim().is_empty() { let _=editor.add_history_entry(l.as_str()); } l }, Err(_)=> break }
        } else {
            // No prompt in stdio mode; just read a line
            let mut l = String::new(); if io::stdin().read_line(&mut l).is_err() { break; } l
        };
        let line_trim = line.trim_end_matches(['\r','\n']);
        let trimmed = line_trim.trim();
        if trimmed.is_empty() { continue; }

        // Aliases
        if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("system") { break; }

        // Line-numbered program entry (only when not buffering a snippet)
        if buffer.is_empty() {
            // Detect leading integer line number followed by optional code
            let mut di = 0usize;
            for ch in trimmed.chars() { if ch.is_ascii_digit() { di += ch.len_utf8(); } else { break; } }
            if di > 0 && (di == trimmed.len() || trimmed[di..].chars().next().map(|c| c.is_whitespace()).unwrap_or(true)) {
                if let Ok(ln) = trimmed[..di].parse::<usize>() {
                    let code = trimmed[di..].trim_start();
                    if code.is_empty() { program_buf.remove(&ln); } else { program_buf.insert(ln, code.to_string()); }
                    continue;
                }
            }
            // CLI-only commands: LIST, CLEAR, RUN, LOAD, SAVE
            let upper = trimmed.to_ascii_uppercase();
            if upper == "LIST" {
                for (ln, txt) in &program_buf {
                    println!("{} {}", ln, txt);
                }
                continue;
            } else if upper == "CLEAR" {
                program_buf.clear();
                continue;
            } else if upper.starts_with("RUN") {
                // Extract optional filename (no spaces supported inside path)
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let file = parts[1];
                    // Clear buffer and load file (but also run it)
                    program_buf.clear();
                    match fs::read_to_string(file) {
                        Ok(s) => {
                            for (i, line) in s.lines().enumerate() {
                                let ln = i + 1;
                                if !line.trim_end().is_empty() { program_buf.insert(ln, line.to_string()); }
                            }
                            last_file = file.to_string();
                        }
                        Err(e) => { eprintln!("load error: {}", e); continue; }
                    }
                    if let Err(e) = sess.run_program(file) { eprintln!("Error running program: {}", e); }
                } else {
                    // Save program_buf to last_file (default _.basil) with blank-line gaps, then run
                    let path = last_file.clone();
                    let max_ln = program_buf.keys().copied().max().unwrap_or(0);
                    let mut out = String::new();
                    if max_ln > 0 {
                        for i in 1..=max_ln { if let Some(t) = program_buf.get(&i) { out.push_str(t); out.push('\n'); } else { out.push('\n'); } }
                    }
                    if let Err(e) = fs::write(&path, out) { eprintln!("save error: {}", e); continue; }
                    if let Err(e) = sess.run_program(&path) { eprintln!("Error running program: {}", e); }
                }
                continue;
            } else if upper.starts_with("LOAD ") || upper == "LOAD" {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 { println!("usage: LOAD <filename>"); continue; }
                let file = parts[1];
                program_buf.clear();
                match fs::read_to_string(file) {
                    Ok(s) => {
                        for (i, line) in s.lines().enumerate() {
                            let ln = i + 1;
                            if !line.trim_end().is_empty() { program_buf.insert(ln, line.to_string()); }
                        }
                        last_file = file.to_string();
                    }
                    Err(e) => { eprintln!("load error: {}", e); }
                }
                continue;
            } else if upper.starts_with("SAVE") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 { last_file = parts[1].to_string(); }
                let path = last_file.clone();
                let max_ln = program_buf.keys().copied().max().unwrap_or(0);
                let mut out = String::new();
                if max_ln > 0 {
                    for i in 1..=max_ln { if let Some(t) = program_buf.get(&i) { out.push_str(t); out.push('\n'); } else { out.push('\n'); } }
                }
                if let Err(e) = fs::write(&path, out) { eprintln!("save error: {}", e); }
                continue;
            } else if upper == "STATUS" {
                // Program listing (like LIST)
                for (ln, txt) in &program_buf {
                    println!("{} {}", ln, txt);
                }
                // Then symbol table with types, values, and origins
                println!("-- SYMBOLS --");
                for name in &sess.order {
                    if let Some(v) = sess.globals.get(name) {
                        let ty = match v {
                            Value::Null => "NULL",
                            Value::Bool(_) => "BOOL",
                            Value::Num(_) => "FLOAT",
                            Value::Int(_) => "INTEGER",
                            Value::Str(_) => "STRING",
                            Value::Func(f) => { let _ = f; "FUNCTION" },
                            Value::Array(_) => "ARRAY",
                            Value::Object(obj) => { let _ = obj; "OBJECT" },
                            Value::StrArray2D { .. } => "STRARRAY2D",
                        };
                        let origins = sess.origins.get(name).cloned().unwrap_or_default();
                        if origins.is_empty() {
                            println!("{} : {} = {}", name, ty, v);
                        } else {
                            println!("{} : {} = {}    from: {}", name, ty, v, origins.join(", "));
                        }
                    }
                }
                continue;
            }
        }

        // Meta cmds
        if trimmed.starts_with(":") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            match parts.get(0).copied().unwrap_or("") {
                ":help" => {
                    println!(":help, :vars [filter], :types [name], :methods <var>, :disasm <name>, :history, :save <file>, :load <file>, :bt on|off, :env, :exit");
                }
                ":vars" => {
                    let filt = parts.get(1).map(|s| *s);
                    for (n, v) in sess.list_globals(filt) { println!("{} = {}", n, v); }
                }
                ":types" => {
                    if let Some(name) = parts.get(1) {
                        // Best-effort: show runtime type
                        if let Some(v) = sess.globals.get(&name.to_string()) {
                            let ty = match v {
                                Value::Null => "NULL",
                                Value::Bool(_) => "BOOL",
                                Value::Num(_) => "FLOAT",
                                Value::Int(_) => "INTEGER",
                                Value::Str(_) => "STRING",
                                Value::Func(_) => "FUNCTION",
                                Value::Array(_) => "ARRAY",
                                Value::Object(_) => "OBJECT",
                                Value::StrArray2D { .. } => "STRARRAY2D",
                            };
                            println!("{} : {}", name, ty);
                        } else { println!("not found"); }
                    } else {
                        println!("types: functions/classes not indexed in this build");
                    }
                }
                ":methods" => {
                    println!("methods: reflection metadata not available in this build");
                }
                ":disasm" => {
                    if let Some(name) = parts.get(1) {
                        if let Some(Value::Func(f)) = sess.globals.get(&name.to_string()) {
                            // very small disasm: print op bytes
                            let c = f.chunk.as_ref();
                            println!("; function {} /{}", f.name.clone().unwrap_or(name.to_string()), f.arity);
                            println!("code bytes: {}", c.code.len());
                        } else { println!("not found or not a function"); }
                    } else { println!("usage: :disasm <name>"); }
                }
                ":history" => {
                    for (i, h) in sess.history.iter().enumerate() { let first = h.lines().next().unwrap_or(""); println!("{:>4}: {}", i+1, first); }
                }
                ":save" => {
                    if let Some(file) = parts.get(1) {
                        let to_save = if !buffer.trim().is_empty() { buffer.clone() } else { sess.history.last().cloned().unwrap_or_default() };
                        if to_save.is_empty() { println!("nothing to save"); }
                        else { if let Err(e) = fs::write(file, to_save) { eprintln!("save error: {}", e); } }
                    } else { println!("usage: :save <file>"); }
                }
                ":load" => {
                    if let Some(file) = parts.get(1) {
                        match fs::read_to_string(file) { Ok(s)=>{ buffer.push_str(&s); println!("(loaded into buffer; enter ';;' to run)"); }, Err(e)=> eprintln!("load error: {}", e) }
                    } else { println!("usage: :load <file>"); }
                }
                ":bt" => {
                    match parts.get(1).copied() { Some("on")=>{sess.settings.show_backtraces=true; println!("backtraces: on");}, Some("off")=>{sess.settings.show_backtraces=false; println!("backtraces: off");}, _=> println!("usage: :bt on|off") }
                }
                ":env" => {
                    println!("REPL: features/search paths not tracked in this build");
                }
                ":exit" => { break; }
                other => { println!("unknown meta command: {}", other); }
            }
            continue;
        }

        // Shell escapes
        if trimmed.starts_with('!') {
            let cmd = trimmed.trim_start_matches('!').trim();
            if cmd.is_empty() {
                // interactive shell
                let _ = std::process::Command::new("cmd").spawn().and_then(|mut c| c.wait());
            } else {
                let output = if cfg!(windows) {
                    std::process::Command::new("cmd").arg("/C").arg(cmd).output()
                } else {
                    std::process::Command::new("sh").arg("-lc").arg(cmd).output()
                };
                match output {
                    Ok(o) => {
                        if !o.stdout.is_empty() { print!("{}", String::from_utf8_lossy(&o.stdout)); }
                        if !o.stderr.is_empty() { eprint!("{}", String::from_utf8_lossy(&o.stderr)); }
                    }
                    Err(e) => eprintln!("shell: {}", e),
                }
            }
            continue;
        }

        // Accumulate into buffer until we see ';;' at end of line (allow trailing whitespace)
        if trimmed.ends_with(";;") {
            let mut snippet = buffer.clone();
            let mut ln = line_trim.to_string();
            // strip trailing ';;'
            if let Some(p) = ln.rfind(";;") { ln.replace_range(p.., ""); }
            if !snippet.is_empty() && !snippet.ends_with('\n') { snippet.push('\n'); }
            snippet.push_str(&ln);

            // Evaluate
            match sess.eval_snippet(&snippet) {
                Ok(()) => { sess.history.push(snippet); }
                Err(e) => { eprintln!("{}", e); }
            }
            buffer.clear();
        } else {
            if !buffer.is_empty() { buffer.push('\n'); }
            buffer.push_str(line_trim);
        }
    }
}
