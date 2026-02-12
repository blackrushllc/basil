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
Copyright (C) 2026, Blackrush LLC
Created by Erik Lee Olson, Tarpon Springs, Florida
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

use std::env;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use basil_parser::parse;
use basil_compiler::compile;
use basil_compiler::service::{analyze_source, CompilerDiagnostics};
use basil_vm::{VM, MockInputProvider};
use basil_vm::debug::{Debugger, DebugEvent};
use basil_lexer::Lexer; // add this near the other use lines
use basil_bytecode::{serialize_program, deserialize_program};
use basil_bytecode::SourceMapMini;
use std::collections::HashMap;
use serde_json;
use once_cell::sync::OnceCell;

mod template;
mod repl;
mod runtime;
mod embedded;
use template::{precompile_template, parse_directives_and_bom, Directives};
use basil_preprocessor as pre;

#[derive(Clone, Default)]
struct PreprocCliCfg {
    include_paths: Vec<PathBuf>,
    embedded_allowed: bool,
    defines: HashMap<String, pre::MacroValue>,
}

static PREPROC_CFG: OnceCell<PreprocCliCfg> = OnceCell::new();

pub(crate) fn current_preproc_cli_cfg() -> PreprocCliCfg {
    PREPROC_CFG.get().cloned().unwrap_or_else(|| PreprocCliCfg { include_paths: Vec::new(), embedded_allowed: true, defines: HashMap::new() })
}

fn parse_preproc_flags(args: &mut Vec<String>) {
    let mut cfg = PreprocCliCfg { include_paths: Vec::new(), embedded_allowed: true, defines: HashMap::new() };
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        let a = &args[i];
        if a == "-I" || a == "--include-path" {
            if i + 1 >= args.len() { eprintln!("{} requires a directory", a); std::process::exit(2); }
            cfg.include_paths.push(PathBuf::from(args[i+1].clone()));
            i += 2; continue;
        } else if a.starts_with("-I") && a.len() > 2 {
            cfg.include_paths.push(PathBuf::from(a[2..].to_string()));
            i += 1; continue;
        } else if let Some(rest) = a.strip_prefix("--include-path=") {
            cfg.include_paths.push(PathBuf::from(rest.to_string()));
            i += 1; continue;
        } else if a == "--no-embedded-includes" {
            cfg.embedded_allowed = false;
            i += 1; continue;
        } else if a == "--D" {
            if i + 1 >= args.len() { eprintln!("--D requires NAME or NAME=VALUE"); std::process::exit(2); }
            add_define(&mut cfg, &args[i+1]);
            i += 2; continue;
        } else if let Some(rest) = a.strip_prefix("--D=") {
            add_define(&mut cfg, rest);
            i += 1; continue;
        } else {
            out.push(a.clone());
            i += 1; continue;
        }
    }
    *args = out; // keep only unconsumed args
    let _ = PREPROC_CFG.set(cfg);
}

fn add_define(cfg: &mut PreprocCliCfg, spec: &str) {
    if spec.is_empty() { return; }
    if let Some((name, val)) = spec.split_once('=') {
        if let Ok(iv) = val.parse::<i64>() {
            cfg.defines.insert(name.to_string(), pre::MacroValue::Int(iv));
        } else if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2) || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2) {
            let trimmed = &val[1..val.len()-1];
            cfg.defines.insert(name.to_string(), pre::MacroValue::Str(trimmed.to_string()));
        } else {
            cfg.defines.insert(name.to_string(), pre::MacroValue::Str(val.to_string()));
        }
    } else {
        cfg.defines.insert(spec.to_string(), pre::MacroValue::Bool(true));
    }
}

// Global embedded provider adapter for the preprocessor
struct EmbeddedAdapter;
impl pre::IncludeProvider for EmbeddedAdapter {
    fn try_read(&self, logical: &str) -> Option<&'static [u8]> {
        if let Some(f) = crate::embedded::find_file(logical) { Some(f.contents) } else { None }
    }
}
static EMBEDDED_ADAPTER: EmbeddedAdapter = EmbeddedAdapter;

pub(crate) fn build_pre_opts(root_path: PathBuf, env_paths: Vec<PathBuf>) -> pre::PreprocessOptions<'static> {
    let cfg = current_preproc_cli_cfg();
    pre::PreprocessOptions {
        root_path,
        include_paths: cfg.include_paths.clone(),
        env_paths,
        embedded: if cfg.embedded_allowed { Some(&EMBEDDED_ADAPTER) } else { None },
        defines: cfg.defines.clone(),
        engine_name: Some("basilc".to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

fn cmd_analyze(path: String, json: bool) {
    let abs_path: PathBuf = match fs::canonicalize(&path) { Ok(p)=>p, Err(_)=>PathBuf::from(&path) };
    let src = match std::fs::read_to_string(&abs_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("read {}: {}", abs_path.display(), e); std::process::exit(1); }
    };
    // Preprocess prior to analysis
    let env_paths: Vec<PathBuf> = match env::var("BASIL_PATH") {
        Ok(val) => { let sep = if cfg!(windows) { ';' } else { ':' }; val.split(sep).filter(|s| !s.trim().is_empty()).map(|s| PathBuf::from(s)).collect() },
        Err(_) => Vec::new(),
    };
    let pp_opts = build_pre_opts(abs_path.clone(), env_paths);
    let preprocessed = match pre::preprocess(&src, &pp_opts) { Ok(p)=>p, Err(e)=>{ eprintln!("preprocess error: {}", e); std::process::exit(1);} };
    let diags: CompilerDiagnostics = analyze_source(&preprocessed.text, &path);
    if json {
        match serde_json::to_string_pretty(&diags) {
            Ok(s) => println!("{}", s),
            Err(e) => { eprintln!("json: {}", e); std::process::exit(1); }
        }
    } else {
        if diags.errors.is_empty() { println!("No errors."); } else {
            println!("Errors:");
            for e in &diags.errors {
                println!("- {:?} at {}:{}: {}", e.severity, e.line, e.column, e.message);
            }
        }
        if !diags.symbols.is_empty() {
            println!("Symbols:");
            for s in &diags.symbols {
                println!("- {:?} {} @{}:{}", s.kind, s.name, s.line, s.col);
            }
        }
    }
}

fn cmd_debug(path: Option<String>) {
    let input_path = match path {
        Some(p) => p,
        None => { eprintln!("usage: basilc --debug <file.basil>"); std::process::exit(2); }
    };
    let abs_path: PathBuf = match fs::canonicalize(&input_path) { Ok(p)=>p, Err(_)=>PathBuf::from(&input_path) };
    let src = match std::fs::read_to_string(&abs_path) { Ok(s)=>s, Err(e)=>{ eprintln!("{}", e); std::process::exit(1);} };
    let pre = template::PrecompileResult { basil_source: src.clone(), directives: Directives::default() };
    // Preprocess
    let env_paths: Vec<PathBuf> = match env::var("BASIL_PATH") {
        Ok(val) => {
            let sep = if cfg!(windows) { ';' } else { ':' };
            val.split(sep).filter(|s| !s.trim().is_empty()).map(|s| PathBuf::from(s)).collect()
        },
        Err(_) => Vec::new(),
    };
    let pp_opts = build_pre_opts(abs_path.clone(), env_paths);
    let preprocessed = match pre::preprocess(&pre.basil_source, &pp_opts) {
        Ok(p) => p,
        Err(e) => { eprintln!("preprocess error: {}", e); std::process::exit(1); }
    };
    let ast = match parse(&preprocessed.text) { Ok(a)=>a, Err(e)=>{ eprintln!("parse error: {}", e); std::process::exit(1);} };
    let mut program = match compile(&ast) { Ok(p)=>p, Err(e)=>{ eprintln!("compile error: {}", e); std::process::exit(1);} };
    // Phase B: embed source map
    let debug_smap = SourceMapMini { files: preprocessed.source_map.files.clone(), lines: preprocessed.source_map.lines.clone() };
    program.source_map = Some(debug_smap.clone());
    let dbg = Debugger::new();
    let rx = dbg.subscribe();
    // Spawn a thread to print JSON events
    std::thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            let s = serde_json::to_string(&match &ev {
                DebugEvent::Started => serde_json::json!({"event":"Started"}),
                DebugEvent::StoppedBreakpoint{file,line} => serde_json::json!({"event":"Stopped","reason":"Breakpoint","file":file,"line":line}),
                DebugEvent::Continued => serde_json::json!({"event":"Continued"}),
                DebugEvent::Exited => serde_json::json!({"event":"Exited"}),
                DebugEvent::Output(text) => serde_json::json!({"event":"Output","text":text}),
            }).unwrap();
            println!("{}", s);
        }
    });
    let mut vm = VM::new(program);
    vm.set_script_path(abs_path.to_string_lossy().to_string());
    vm.set_debugger(dbg);
    if let Err(e) = vm.run() {
        let line = vm.current_line() as usize;
        if line > 0 {
            let sm = &debug_smap;
            if line < sm.lines.len() {
                let (fi, ln) = sm.lines[line];
                let fname = sm.files.get(fi as usize).map(|s| std::path::Path::new(s).file_name().and_then(|s| s.to_str()).unwrap_or(s)).unwrap_or("<unknown>");
                eprintln!("runtime error at line {} in {}: {}", ln, fname, e);
            } else {
                eprintln!("runtime error at line {}: {}", line, e);
            }
        } else { eprintln!("runtime error: {}", e); }
        std::process::exit(1);
    }
}



// Map fun aliases → canonical commands
fn canonicalize(cmd: &str) -> &str {
    match cmd.to_ascii_lowercase().as_str() {
        // serious
        "init" => "init",
        "run" => "run",
        "build" => "build",
        "test" => "test",
        "fmt" => "fmt",
        "add" => "add",
        "clean" => "clean",
        "dev" => "dev",
        "serve" => "serve",
        "doc" => "doc",
        // punny
        "seed" => "init",
        "sprout" => "run",
        "harvest" => "build",
        "cultivate" => "test",
        "prune" => "fmt",
        "infuse" => "add",
        "compost" => "clean",
        "steep" => "dev",
        "greenhouse" => "serve",
        "bouquet" => "doc",
        "lex" => "lex",
        "chop" => "lex",   // fun alias
        _ => cmd,
    }
}

fn print_help() {
    println!("Basil CLI (80's version)\n");
    println!("Commands (aliases in parentheses):");
    println!("  run  (sprout)      Parse → compile → run a .basil file");
    println!("  test (cultivate)   Run program in test mode with auto-mocked input");
    println!("  lex  (chop)        Dump tokens from a .basil file (debug)");
    println!("  make               Write embedded includes to the current directory (see --list)\n");
    //println!("  init (seed)        Create a new Basil project");
    //println!("  build (harvest)    Build project (stub)");
    //println!("  fmt  (prune)       Format sources (stub)");
    //println!("  add  (infuse)      Add dependency (stub)");
    //println!("  clean (compost)    Remove build artifacts (stub)");
    //println!("  dev  (steep)       Start dev mode (stub)");
    //println!("  serve (greenhouse) Serve local HTTP (stub)");
    //println!("  doc  (bouquet)     Generate docs (stub)\n");
    //println!("  --ai               Start AI REPL (streaming chat)");
    println!("  --analyze <file> [--json]  Run compiler analysis and print diagnostics/symbols");
    println!("  --debug <file>             Run Basil VM with JSON debug events\n");
    println!("Usage:");
    println!("  basilc <command> [args]\n");
    println!("Examples:");
    println!("  basilc run examples/hello.basil");
    println!("  basilc lex examples/hello.basil");
    println!("  basilc make --list");
    println!("  basilc make examples");
    println!("  basilc make examples/hello.basil");
    println!("  basilc make upgrade");
    println!("  basilc test testprogs/bigtest.basil");
    println!("  basilc --analyze examples/hello.basil --json");
    println!("  basilc --debug examples/hello.basil");
    //println!("  basilc --ai");
    println!("");
    println!("For more information and lots of docs, visit https://github.com/blackrushllc/basil");
    println!("");
    println!("Type 'status' for build information and to see your objects, or try PRINT \"Hello, World!\";;");
    println!("");


}

fn cmd_init(target: Option<String>) -> io::Result<()> {
    let name = target.unwrap_or_else(|| "basil_app".to_string());
    let root = Path::new(&name);
    if root.exists() {
        eprintln!("error: path '{}' already exists", name);
        std::process::exit(1);
    }
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/main.basil"), "PRINT \"Hello, Basil!\";\n")?;
    let toml = format!(
        "package = \"{}\"\nversion = \"0.0.1\"\nedition = \"2026\"\n\n[dependencies]\n",
        name
    );
    fs::write(root.join("basil.toml"), toml)?;
    println!("Initialized Basil project at ./{}", name);
    Ok(())
}

fn cmd_lex(path: Option<String>) {
    let Some(path) = path else { eprintln!("usage: basilc lex <file.basil>"); std::process::exit(2) };
    let abs_path: PathBuf = match fs::canonicalize(&path) { Ok(p)=>p, Err(_)=>PathBuf::from(&path) };
    let src = std::fs::read_to_string(&abs_path).expect("read file");
    // Preprocess before lexing
    let env_paths: Vec<PathBuf> = match env::var("BASIL_PATH") {
        Ok(val) => { let sep = if cfg!(windows) { ';' } else { ':' }; val.split(sep).filter(|s| !s.trim().is_empty()).map(|s| PathBuf::from(s)).collect() },
        Err(_) => Vec::new(),
    };
    let pp_opts = build_pre_opts(abs_path.clone(), env_paths);
    let preprocessed = match pre::preprocess(&src, &pp_opts) { Ok(p)=>p, Err(e)=>{ eprintln!("preprocess error: {}", e); std::process::exit(1);} };
    let mut lx = Lexer::new(&preprocessed.text);
    match lx.tokenize() {
        Ok(toks) => {
            for t in toks {
                println!("{:?}\t'{}'\t@{}..{}", t.kind, t.lexeme, t.span.start, t.span.end);
            }
        }
        Err(e) => { eprintln!("lex error: {}", e); std::process::exit(1); }
    }
}

fn print_embedded_inventory() {
    println!("Embedded files:");
    for p in embedded::list_all_paths() {
        println!("  {}", p);
    }
    let dirs = embedded::list_top_level_dirs();
    if !dirs.is_empty() {
        println!("\nTop-level dirs: {}", dirs.join(", "));
    }
}

fn cmd_make(args: Vec<String>) {
    // basilc make --list | -l
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("usage: basilc make [--list|-l] <target>\n\nTargets come from the built-in includes/ tree. Examples:\n  basilc make --list\n  basilc make examples\n  basilc make examples/hello.basil\n  basilc make upgrade");
        return;
    }

    if args[0] == "--list" || args[0] == "-l" {
        print_embedded_inventory();
        return;
    }

    let target = &args[0];
    if embedded::is_unsafe_target(target) {
        eprintln!("Refusing unsafe target: {}", target);
        std::process::exit(2);
    }

    let cwd = match env::current_dir() { Ok(p) => p, Err(e) => { eprintln!("cwd: {}", e); std::process::exit(1); } };

    let file = embedded::find_file(target);
    let is_dir = embedded::has_dir(target);

    if let Some(_f) = file {
        match embedded::write_single(target, &cwd) {
            Ok(out) => {
                println!("Wrote file: {}", out.display());
                // Optionally run when it's a single file
                let skip = env::var("BASILC_MAKE_SKIP_RUN").ok().unwrap_or_default();
                if skip != "1" {
                    cmd_run(Some(out.display().to_string()));
                }
            }
            Err(e) => { eprintln!("write: {}", e); std::process::exit(1); }
        }
        return;
    }

    if is_dir {
        if let Err(e) = embedded::extract_dir(target, &cwd) {
            eprintln!("extract: {}", e);
            std::process::exit(1);
        }
        println!("Wrote directory: {}/", target);
        return;
    }

    eprintln!("No embedded file or dir named '{target}'. Try 'basilc make --list'.");
    std::process::exit(2);
}

fn cmd_run(path: Option<String>) {
    // Require a path
    let input_path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: basilc run <file.basil>");
            std::process::exit(2);
        }
    };

    // Optional: refuse obvious non-source invocations (helps catch /usr/lib/cgi-bin/basil.cgi)
    if !(input_path.ends_with(".basil") || input_path.ends_with(".bas")) {
        eprintln!("Refusing to run a non-.basil/.bas file: {}", input_path);
        std::process::exit(2);
    }

    // Resolve absolute path for reading/caching, but set CWD using the user-provided path to avoid Windows \\?\ prefixes.
    let abs_path: PathBuf = match fs::canonicalize(&input_path) {
        Ok(p) => p,
        Err(_) => PathBuf::from(&input_path),
    };
    // IMPORTANT (Windows): Do not use canonicalized path for CWD, because it may contain the \\?\ prefix that cmd.exe rejects.
    let script_dir_for_cwd = Path::new(&input_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    if let Err(e) = env::set_current_dir(&script_dir_for_cwd) {
        eprintln!("warning: failed to set current dir to script dir ({}): {}", script_dir_for_cwd.display(), e);
    }

    // Read the source once, with good error messages (use absolute path to avoid cwd side-effects)
    let src = match std::fs::read_to_string(&abs_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            eprintln!("File is not UTF-8 text: {}", abs_path.display());
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("Failed to read {}: {}", abs_path.display(), e);
            std::process::exit(1);
        }
    };

    // Decide whether this is a template or plain Basil.
    // In CLI mode, do NOT fall back to templating on parse errors — that would echo source.
    // Only treat as template if explicit template markers are present.
    let looks_like_template = src.contains("<?");
    if env::var("BASIL_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("[basilc] CLI run: looks_like_template={} (contains'<?'={})", looks_like_template, src.contains("<?"));
    }
    let pre = if looks_like_template {
        if env::var("BASIL_DEBUG").ok().as_deref() == Some("1") { eprintln!("[basilc] Using template precompiler in CLI"); }
        match precompile_template(&src) {
            Ok(r) => r,
            Err(e) => { eprintln!("template error: {}", e); std::process::exit(1); }
        }
    } else {
        if env::var("BASIL_DEBUG").ok().as_deref() == Some("1") { eprintln!("[basilc] Treating as plain Basil"); }
        template::PrecompileResult { basil_source: src.clone(), directives: Directives::default() }
    };

    // Phase 1 preprocessor (scaffold): call preprocessor before parsing
    let env_paths: Vec<PathBuf> = match env::var("BASIL_PATH") {
        Ok(val) => {
            let sep = if cfg!(windows) { ';' } else { ':' };
            val.split(sep).filter(|s| !s.trim().is_empty()).map(|s| PathBuf::from(s)).collect()
        },
        Err(_) => Vec::new(),
    };
    let pp_opts = build_pre_opts(abs_path.clone(), env_paths);
    let preprocessed = match pre::preprocess(&pre.basil_source, &pp_opts) {
        Ok(p) => p,
        Err(e) => { eprintln!("preprocess error: {}", e); std::process::exit(1); }
    };

    // Prepare cache fingerprint
    let meta = match fs::metadata(&abs_path) { Ok(m)=>m, Err(e)=>{ eprintln!("stat {}: {}", abs_path.display(), e); std::process::exit(1);} };
    let source_size = meta.len();
    let source_mtime_ns: u64 = meta.modified().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // Flags for cache: bit0 = short_tags_on, bit1 = templating_used
    let templating_used = src.contains("<?");
    let flags: u32 = (if pre.directives.short_tags_on { 1u32 } else { 0u32 })
                   | (if templating_used { 2u32 } else { 0u32 });

    // Cache path (next to the script)
    let mut cache_path = abs_path.clone();
    cache_path.set_extension("basilx");

    // Try cache load
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
                match deserialize_program(prog_bytes) { Ok(p)=>program_opt=Some(p), Err(_)=>{ /* fall through to recompile */ } }
            }
        }
    }

    // Keep a local copy of the source map for error mapping even if loading from cache
    let (program, local_smap_opt) = if let Some(p) = program_opt {
        // Program from cache; reuse embedded source map
        let sm = p.source_map.clone();
        (p, sm)
    } else {
        // Parse → compile the precompiled Basil source
        let ast = match parse(&preprocessed.text) { Ok(a)=>a, Err(e)=>{ eprintln!("parse error: {}", e); std::process::exit(1);} };
        let mut prog = match compile(&ast) { Ok(p)=>p, Err(e)=>{ eprintln!("compile error: {}", e); std::process::exit(1);} };
        // Phase B: attach source map
        let map = SourceMapMini { files: preprocessed.source_map.files.clone(), lines: preprocessed.source_map.lines.clone() };
        prog.source_map = Some(map.clone());
        // Write cache atomically
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
        if let Ok(mut f) = File::create(&tmp) {
            let _ = f.write_all(&hdr);
            let _ = f.sync_all();
            let _ = fs::rename(&tmp, &cache_path);
        }
        (prog, Some(map))
    };

    // Run VM
    let mut vm = VM::new(program);
    // Provide script path so CLASS() can resolve relative class files
    vm.set_script_path(abs_path.to_string_lossy().to_string());
    if let Err(e) = vm.run() {
        let line = vm.current_line() as usize;
        if line > 0 {
            if let Some(sm) = &local_smap_opt {
                if line < sm.lines.len() {
                    let (fi, ln) = sm.lines[line];
                    let fname = sm.files.get(fi as usize).map(|s| std::path::Path::new(s).file_name().and_then(|s| s.to_str()).unwrap_or(s)).unwrap_or("<unknown>");
                    eprintln!("runtime error at line {} in {}: {}", ln, fname, e);
                } else { eprintln!("runtime error at line {}: {}", line, e); }
            } else { eprintln!("runtime error at line {}: {}", line, e); }
        } else { eprintln!("runtime error: {}", e); }
        std::process::exit(1);
    } else if vm.is_suspended() {
        // In RUN mode, when STOP is encountered, remain suspended with no prompt.
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    }
}



/// --- New: mode detection ---

fn is_cgi_invocation() -> bool {
    // Apache/CGI set these; lighttpd/nginx-fastcgi set similar.
    env::var("GATEWAY_INTERFACE").is_ok() && env::var("REQUEST_METHOD").is_ok()
}

/// --- Your existing CLI entry, unchanged logic moved here ---

fn cli_main() {
    // === BEGIN: your old main() body ===
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    // Parse and strip preprocessor-related flags early so they don't confuse subcommand parsing
    parse_preproc_flags(&mut args);
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        let path = args.get(0).cloned();
        let sess = repl::Session::new(repl::SessionSettings::default());
        repl::start_repl(sess, path);
        return;
    }
    // Early flag handling for analysis/debug modes used by IDE tooling
    if args[0] == "--analyze" || args[0] == "-A" {
        if args.len() < 2 { eprintln!("usage: basilc --analyze <file.basil> [--json]"); std::process::exit(2); }
        let file = args[1].clone();
        let json = args.iter().any(|a| a == "--json");
        cmd_analyze(file, json);
        return;
    }
    if args[0] == "--debug" {
        let path = args.get(1).cloned();
        cmd_debug(path);
        return;
    }
    let cmd = canonicalize(&args[0]).to_string();
    args.remove(0);

    match cmd.as_str() {
        "init" => {
            let name = args.get(0).cloned();
            if let Err(e) = cmd_init(name) {
                eprintln!("init error: {}", e);
                std::process::exit(1);
            }
        }
        "run" => {
            cmd_run(args.get(0).cloned());
        }
        "make" => {
            cmd_make(args);
        }
        "cli" => {
            // basilc cli [path]
            let path = args.get(0).cloned();
            let sess = repl::Session::new(repl::SessionSettings::default());
            repl::start_repl(sess, path);
        }
        "test" => {
            cmd_test(args);
        }
        "build" | "fmt" | "add" | "clean" | "dev" | "serve" | "doc" => {
            println!("[stub] '{}' not implemented yet in the prototype", cmd);
        }
        "lex" => { cmd_lex(args.get(0).cloned()); }
        other => {
            eprintln!("unknown command: '{}'\n", other);
            print_help();
            std::process::exit(2);
        }
    }
    // === END: your old main() body ===
}

/// --- New: CGI entrypoint that wraps your CLI 'run' ---

fn cgi_main() {
    // 1) Resolve the Basil script path the request mapped to
    let script_path = resolve_script_path().unwrap_or_else(|| "/var/www/html/index.basil".to_string());

    // let script_path = env::var("SCRIPT_FILENAME")
    //     .or_else(|_| env::var("PATH_TRANSLATED"))
    //     .or_else(|_| env::var("PATH_INFO").map(|p| format!("/var/www{}", p)))
    //     .unwrap_or_else(|_| "/var/www/html/index.basil".to_string());

    if !Path::new(&script_path).exists() {
        println!("Status: 404 Not Found");
        println!("Content-Type: text/plain; charset=utf-8");
        println!();
        println!("Basil file not found: {}", script_path);
        return;
    }

    // 2) Gather request bits
    let method = env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
    let query  = env::var("QUERY_STRING").unwrap_or_default();
    let ctype  = env::var("CONTENT_TYPE").unwrap_or_default();
    let clen: usize = env::var("CONTENT_LENGTH").ok().and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut body = Vec::with_capacity(clen);
    if clen > 0 {
        let stdin = io::stdin();
        stdin.take(clen as u64).read_to_end(&mut body).ok();
    }

    // 3) Spawn *this* binary in CLI mode to run the script
    //    We force CLI mode so the child doesn't enter cgi_main() again.
    let self_exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to locate current executable: {e}");
            return;
        }
    };

    let mut child = match Command::new(self_exe)
        .arg("run")
        .arg(&script_path)
        .env("BASIL_FORCE_MODE", "cli")       // <- prevents recursion
        .env("QUERY_STRING", &query)          // pass through web context
        .env("REQUEST_METHOD", &method)
        .env("CONTENT_TYPE", &ctype)
        .env("CONTENT_LENGTH", clen.to_string())
        .env("SCRIPT_FILENAME", &script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to spawn Basil runner: {e}");
            return;
        }
    };

    // 4) Pipe request body to the child (if your Basil runtime wants it)
    if clen > 0 {
        if let Some(mut sin) = child.stdin.take() {
            let _ = sin.write_all(&body);
        }
    }

    // 5) Collect output
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to run Basil script: {e}");
            return;
        }
    };

    // Prepare outputs for reuse
    let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
    let has_stderr = !stderr_text.trim().is_empty();
    let exit_ok = output.status.success();
    // Send child's stderr to Apache error log (very helpful)
    if has_stderr {
        eprintln!("{}", stderr_text);
    }

    // Parse directives from the source to determine header policy
    let src_for_dirs = match fs::read_to_string(&script_path) { Ok(s)=>s, Err(_)=>String::new() };
    let (dirs, _) = parse_directives_and_bom(&src_for_dirs);

    let stdout = output.stdout;
    let stdout_empty = stdout.is_empty();

    if dirs.cgi_no_header {
        // Manual header mode: verify the program sent valid CGI headers (terminated by blank line)
        let has_blank = stdout.windows(4).any(|w| w == b"\r\n\r\n");
        if has_blank {
            io::stdout().write_all(&stdout).ok();
        } else {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            if has_stderr {
                println!("Basil runtime error while executing: {}\n", script_path);
                print!("{}", stderr_text);
            } else {
                println!("No CGI header sent. Add headers or remove #CGI_NO_HEADER.");
            }
        }
        return;
    }

    // Automatic header mode:
    // If the child failed or produced only stderr (no stdout), surface an error page with 500.
    if !exit_ok || (stdout_empty && has_stderr) {
        println!("Status: 500 Internal Server Error");
        println!("Content-Type: text/plain; charset=utf-8");
        println!();
        if has_stderr {
            println!("Basil runtime error while executing: {}\n", script_path);
            print!("{}", stderr_text);
        } else {
            println!("Basil runtime error with no additional details available.");
        }
        return;
    }

    // Otherwise, send default header (override if provided) then body
    let header = if let Some(h) = dirs.cgi_default_header { h } else { "Content-Type: text/html; charset=utf-8".to_string() };
    println!("{}", header);
    println!("");
    io::stdout().write_all(&stdout).ok();
}

// use std::env;
// use std::path::{Path, PathBuf};

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Ok(h), Ok(l)) = (u8::from_str_radix(&s[i+1..i+2], 16), u8::from_str_radix(&s[i+2..i+3], 16)) {
                out.push((h << 4 | l) as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn resolve_script_path() -> Option<String> {
    // 1) Prefer SCRIPT_FILENAME if it points to a .basil file
    if let Ok(sf) = env::var("SCRIPT_FILENAME") {
        if sf.ends_with(".basil") && Path::new(&sf).is_file() {
            return Some(sf);
        }
    }
    // 2) PATH_TRANSLATED is often correct under Action
    if let Ok(pt) = env::var("PATH_TRANSLATED") {
        if pt.ends_with(".basil") && Path::new(&pt).is_file() {
            return Some(pt);
        }
    }
    // 3) Try DOCUMENT_ROOT + PATH_INFO
    if let (Ok(docroot), Ok(pi)) = (env::var("DOCUMENT_ROOT"), env::var("PATH_INFO")) {
        let cand = PathBuf::from(docroot).join(pi.trim_start_matches('/'));
        if cand.extension().and_then(|e| e.to_str()) == Some("basil") && cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    // 4) Try DOCUMENT_ROOT + REQUEST_URI (strip query)
    if let (Ok(docroot), Ok(uri)) = (env::var("DOCUMENT_ROOT"), env::var("REQUEST_URI")) {
        let path_part = uri.split('?').next().unwrap_or("");
        let dec = url_decode(path_part);
        let cand = PathBuf::from(docroot).join(dec.trim_start_matches('/'));
        if cand.extension().and_then(|e| e.to_str()) == Some("basil") && cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}


/// --- New: tiny dispatcher ---

fn main() {
    // Ensure the entire CLI runs within a Tokio runtime context
    let _rt_guard = crate::runtime::TOKIO_MAIN_RT.enter();
    // Explicit escape hatch for any subprocess we spawn:
    if env::var("BASIL_FORCE_MODE").ok().as_deref() == Some("cli") {
        cli_main();
        return;
    }

    if is_cgi_invocation() {
        cgi_main();
    } else {
        cli_main();
    }
}


// --- Test mode support ---
fn extract_comments_map(src: &str) -> HashMap<u32, Vec<String>> {
    let mut map: HashMap<u32, Vec<String>> = HashMap::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut pending: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let mut in_str = false;
        let chars: Vec<char> = line.chars().collect();
        let mut idx = 0usize;
        let mut found: Option<(usize, String)> = None;
        while idx < chars.len() {
            let c = chars[idx];
            if c == '"' { in_str = !in_str; idx += 1; continue; }
            if !in_str {
                // C++-style comment
                if c == '/' && idx + 1 < chars.len() && chars[idx+1] == '/' {
                    let text: String = chars[idx+2..].iter().collect();
                    found = Some((idx, text.trim_start().to_string()));
                    break;
                }
                // BASIC single-quote comment
                if c == '\'' {
                    let text: String = chars[idx+1..].iter().collect();
                    found = Some((idx, text.trim_start().to_string()));
                    break;
                }
                // REM comment (only when starting a token)
                if (c == 'R' || c == 'r') && idx + 2 < chars.len() {
                    let c1 = chars[idx+1].to_ascii_uppercase();
                    let c2 = chars[idx+2].to_ascii_uppercase();
                    if c1 == 'E' && c2 == 'M' {
                        if idx == 0 || chars[idx-1].is_whitespace() {
                            let text: String = chars[idx+3..].iter().collect();
                            found = Some((idx, text.trim_start().to_string()));
                            break;
                        }
                    }
                }
            }
            idx += 1;
        }

        let trimmed = line.trim_start();
        let is_code_line = !trimmed.is_empty()
            && !trimmed.starts_with('\'')
            && !trimmed.starts_with('#')
            && !trimmed.to_ascii_uppercase().starts_with("REM")
            && !trimmed.starts_with("//");

        if let Some((pos, text)) = found {
            // If nothing but whitespace precedes the comment, treat as standalone and queue for next code line
            if line[..pos].trim().is_empty() {
                pending.push(text);
            } else {
                // Inline with code: flush any pending (earlier lines) to this line, then attach this comment
                if !pending.is_empty() {
                    let entry = map.entry((i as u32) + 1).or_default();
                    entry.extend(pending.drain(..));
                }
                map.entry((i as u32) + 1).or_default().push(text);
            }
        }

        if is_code_line && !pending.is_empty() {
            let entry = map.entry((i as u32) + 1).or_default();
            entry.extend(pending.drain(..));
        }
    }

    map
}

fn cmd_test(mut args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: basilc test <file.basil> [--seed <u64>] [--max-inputs <n>] [--trace]");
        std::process::exit(2);
    }
    let mut path = args.remove(0);
    if !(path.ends_with(".basil") || path.ends_with(".bas")) {
        eprintln!("Refusing to test a non-.basil/.bas file: {}", path);
        std::process::exit(2);
    }
    // If a .bas file was provided but doesn't exist, try .basil fallback
    if !std::path::Path::new(&path).exists() {
        if path.to_ascii_lowercase().ends_with(".bas") {
            let mut p = std::path::PathBuf::from(&path);
            p.set_extension("basil");
            if p.exists() { path = p.to_string_lossy().to_string(); }
        }
    }

    let mut seed_opt: Option<u64> = None;
    let mut max_inputs: Option<usize> = None;
    let mut trace = false;

    let mut i = 0usize;
    while i < args.len() {
        let a = &args[i];
        if a == "--seed" {
            if i + 1 >= args.len() { eprintln!("--seed requires a value"); std::process::exit(2); }
            seed_opt = args[i+1].parse::<u64>().ok();
            i += 2; continue;
        } else if a.starts_with("--seed=") {
            let v = &a[7..]; seed_opt = v.parse::<u64>().ok(); i += 1; continue;
        } else if a == "--max-inputs" {
            if i + 1 >= args.len() { eprintln!("--max-inputs requires a value"); std::process::exit(2); }
            max_inputs = args[i+1].parse::<usize>().ok(); i += 2; continue;
        } else if a.starts_with("--max-inputs=") {
            let v = &a[13..]; max_inputs = v.parse::<usize>().ok(); i += 1; continue;
        } else if a == "--trace" { trace = true; i += 1; continue; }
        else {
            // Unknown or extra arg; ignore
            i += 1; continue;
        }
    }

    // Read source like cmd_run
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => { eprintln!("File is not UTF-8 text: {}", path); std::process::exit(3); }
        Err(e) => { eprintln!("Failed to read {}: {}", path, e); std::process::exit(1); }
    };

    let looks_like_template = src.contains("<?");
    let pre = if looks_like_template {
        match precompile_template(&src) { Ok(r)=>r, Err(e)=>{ eprintln!("template error: {}", e); std::process::exit(1); } }
    } else {
        template::PrecompileResult { basil_source: src.clone(), directives: Directives::default() }
    };

    // Cache path and fingerprint like cmd_run
    let meta = match fs::metadata(&path) { Ok(m)=>m, Err(e)=>{ eprintln!("stat {}: {}", path, e); std::process::exit(1);} };
    let source_size = meta.len();
    let source_mtime_ns: u64 = meta.modified().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let templating_used = src.contains("<?");
    let flags: u32 = (if pre.directives.short_tags_on { 1u32 } else { 0u32 })
                   | (if templating_used { 2u32 } else { 0u32 });

    let mut cache_path = PathBuf::from(&path);
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
        // Preprocess before parsing
        let env_paths: Vec<PathBuf> = match env::var("BASIL_PATH") {
            Ok(val) => { let sep = if cfg!(windows) { ';' } else { ':' }; val.split(sep).filter(|s| !s.trim().is_empty()).map(|s| PathBuf::from(s)).collect() },
            Err(_) => Vec::new(),
        };
        let pp_opts = build_pre_opts(PathBuf::from(&path), env_paths);
        let preprocessed = match pre::preprocess(&pre.basil_source, &pp_opts) { Ok(p)=>p, Err(e)=>{ eprintln!("preprocess error: {}", e); std::process::exit(1);} };
        let ast = match parse(&preprocessed.text) { Ok(a)=>a, Err(e)=>{ eprintln!("parse error: {}", e); std::process::exit(1);} };
        match compile(&ast) { Ok(p)=>{
            let body = serialize_program(&p);
            let mut hdr = Vec::with_capacity(32 + body.len());
            hdr.extend_from_slice(b"BSLX");
            hdr.extend_from_slice(&3u32.to_le_bytes());
            hdr.extend_from_slice(&1u32.to_le_bytes());
            hdr.extend_from_slice(&flags.to_le_bytes());
            hdr.extend_from_slice(&source_size.to_le_bytes());
            hdr.extend_from_slice(&source_mtime_ns.to_le_bytes());
            hdr.extend_from_slice(&body);
            let tmp = cache_path.with_extension("basilx.tmp");
            if let Ok(mut f) = File::create(&tmp) { let _ = f.write_all(&hdr); let _ = f.sync_all(); let _ = fs::rename(&tmp, &cache_path); }
            p
        }, Err(e)=>{ eprintln!("compile error: {}", e); std::process::exit(1)} }
    };

    let comments_map = extract_comments_map(&pre.basil_source);
    let seed: u64 = seed_opt.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_nanos() as u64).unwrap_or(0)
    });
    let mock = MockInputProvider::new(seed);
    let mut vm = VM::new_with_test(program, mock, trace, Some(path.clone()), Some(comments_map), max_inputs);
    if let Err(e) = vm.run() {
        let line = vm.current_line();
        if line > 0 { eprintln!("runtime error at line {}: {}", line, e); }
        else { eprintln!("runtime error: {}", e); }
        std::process::exit(1);
    }
}
