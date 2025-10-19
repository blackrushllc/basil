use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crossbeam_channel as chan;
use crossbeam_channel::{Receiver, Sender};
use gag::BufferRedirect;
use parking_lot::Mutex;

use basil_parser::parse;
use basil_compiler::compile;
use basil_bytecode::{Program as BCProgram, Value};
use basil_vm::VM;

use basil_host::{HostRequest, PendingConfig};

// Public runner API used by the Basilica GUI
#[derive(Debug, Clone)]
pub struct BasilRunner {
    pub tx: Sender<RunnerCmd>,
    pub rx: Receiver<RunnerEvent>,
}

#[derive(Debug, Clone)]
pub struct RunnerOptions {
    pub with_app: bool,
    pub with_web: bool,
    pub basilica_menu: Option<Arc<Mutex<PendingConfig>>>,
    pub host_tx: Option<Sender<HostRequest>>, // Host→GUI bridge
}

#[derive(Debug, Clone)]
pub enum RunMode { Run, Test, Cli }

#[derive(Debug, Clone)]
pub enum RunnerCmd {
    EvalLine(String),
    RunFile { mode: RunMode, path: String, args: Option<String> },
    Exit,
    // Future: Dispatch web event back into Basil by label
    _DispatchWeb { event: String, id: String },
}

#[derive(Debug, Clone)]
pub enum RunnerEvent {
    Output(String),
    Error(String),
    Suspended,
    Exited,
}

impl BasilRunner {
    pub fn spawn(start_cli: bool, opts: RunnerOptions) -> Self {
        let (tx_cmd, rx_cmd) = chan::unbounded::<RunnerCmd>();
        let (tx_evt, rx_evt) = chan::unbounded::<RunnerEvent>();

        thread::spawn(move || {
            // Install thread context for host surfaces
            basil_host::set_thread_context(opts.host_tx.clone(), opts.basilica_menu.clone());

            let mut sess = Session::new();
            if start_cli {
                // Emit a small banner
                let _ = tx_evt.send(RunnerEvent::Output("Basil CLI ready. Type PRINT \"hello\" or RUN \"path.basil\". Type exit to quit.\n".into()));
            }
            while let Ok(cmd) = rx_cmd.recv() {
                match cmd {
                    RunnerCmd::Exit => { let _ = tx_evt.send(RunnerEvent::Exited); break; }
                    RunnerCmd::EvalLine(line) => {
                        // Normalize and handle built-ins
                        let norm = line.trim().to_ascii_lowercase();
                        if norm == "exit" || norm == "quit" { let _ = tx_evt.send(RunnerEvent::Exited); break; }
                        if norm.starts_with("run ") {
                            // Extract quoted or raw path after RUN
                            let p = line[4..].trim().trim_matches('"').to_string();
                            run_file_simple(&opts, &mut sess, &p, &tx_evt);
                            continue;
                        }
                        // Evaluate as a snippet/program
                        eval_snippet_simple(&opts, &mut sess, &line, &tx_evt);
                    }
                    RunnerCmd::RunFile { mode, path, .. } => {
                        match mode {
                            RunMode::Run | RunMode::Cli | RunMode::Test => {
                                run_file_simple(&opts, &mut sess, &path, &tx_evt);
                            }
                        }
                    }
                    RunnerCmd::_DispatchWeb { .. } => {
                        // Not implemented in this iteration
                    }
                }
            }
        });

        Self { tx: tx_cmd, rx: rx_evt }
    }
}

struct Session {
    globals: HashMap<String, Value>,
    order: Vec<String>,
    suspended_vm: Option<VM>,
    script_path: Option<String>,
}

impl Session {
    fn new() -> Self { Self { globals: HashMap::new(), order: Vec::new(), suspended_vm: None, script_path: None } }
}

fn merge_globals(sess: &mut Session, vm: &VM, origin: Option<&str>) {
    let (names, values) = vm.globals_snapshot();
    for (i, name) in names.iter().enumerate() {
        if !sess.globals.contains_key(name) { sess.order.push(name.clone()); }
        let val = values.get(i).cloned().unwrap_or(Value::Null);
        sess.globals.insert(name.clone(), val);
        // origin currently unused in this embedder
        let _ = origin;
    }
}

fn seed_globals(vm: &mut VM, sess: &Session) {
    for name in vm.globals_snapshot().0.iter() {
        if let Some(v) = sess.globals.get(name) {
            let _ = vm.set_global_by_name(name, v.clone());
        }
    }
}

fn run_file_simple(opts: &RunnerOptions, sess: &mut Session, path: &str, tx_evt: &Sender<RunnerEvent>) {
    let pbuf = PathBuf::from(path);
    let script_path = match std::fs::canonicalize(&pbuf) { Ok(p)=>p, Err(_)=>pbuf.clone() };
    let src = match std::fs::read_to_string(&script_path) {
        Ok(s) => s,
        Err(e) => { let _ = tx_evt.send(RunnerEvent::Error(format!("read {}: {}", script_path.display(), e))); return; }
    };
    let ast = match parse(&src) { Ok(a)=>a, Err(e)=>{ let _ = tx_evt.send(RunnerEvent::Error(format!("parse error: {}", e))); return; } };
    let prog: BCProgram = match compile(&ast) { Ok(p)=>p, Err(e)=>{ let _ = tx_evt.send(RunnerEvent::Error(format!("compile error: {}", e))); return; } };
    let mut vm = VM::new(prog);
    // Register host surfaces so APP.*, WEB.*, BASILICA.MENU.* are available when enabled via thread context
    basil_host::register_hosts(vm.registry_mut());
    vm.set_script_path(script_path.to_string_lossy().to_string());
    seed_globals(&mut vm, sess);
    // Ensure host objects like WEB/APP are instantiated when requested
    seed_host_globals(&mut vm, opts);
    // Capture stdout
    let mut out = String::new();
    let _stdout = BufferRedirect::stdout().expect("redirect stdout");
    let result = vm.run();
    // read captured stdout
    let mut handle = _stdout;
    let _ = handle.read_to_string(&mut out);
    if !out.is_empty() { let _ = tx_evt.send(RunnerEvent::Output(out)); }
    if let Err(e) = result { let line = vm.current_line(); let _ = tx_evt.send(RunnerEvent::Error(if line>0 { format!("runtime error at line {}: {}", line, e) } else { format!("runtime error: {}", e) })); }
    merge_globals(sess, &vm, Some(path));
    if vm.is_suspended() { sess.suspended_vm = Some(vm); let _ = tx_evt.send(RunnerEvent::Suspended); }
}

fn eval_snippet_simple(opts: &RunnerOptions, sess: &mut Session, src: &str, tx_evt: &Sender<RunnerEvent>) {
    let ast = match parse(src) { Ok(a)=>a, Err(e)=>{ let _ = tx_evt.send(RunnerEvent::Error(format!("parse error: {}", e))); return; } };
    let prog: BCProgram = match compile(&ast) { Ok(p)=>p, Err(e)=>{ let _ = tx_evt.send(RunnerEvent::Error(format!("compile error: {}", e))); return; } };
    let mut vm = VM::new(prog);
    // Ensure host surfaces are present for snippet execution
    basil_host::register_hosts(vm.registry_mut());
    if let Some(p) = &sess.script_path { vm.set_script_path(p.clone()); }
    seed_globals(&mut vm, sess);
    seed_host_globals(&mut vm, opts);
    let mut out = String::new();
    let _stdout = BufferRedirect::stdout().expect("redirect stdout");
    let result = vm.run();
    let mut handle = _stdout;
    let _ = handle.read_to_string(&mut out);
    if !out.is_empty() { let _ = tx_evt.send(RunnerEvent::Output(out)); }
    if let Err(e) = result { let line = vm.current_line(); let _ = tx_evt.send(RunnerEvent::Error(if line>0 { format!("runtime error at line {}: {}", line, e) } else { format!("runtime error: {}", e) })); }
    merge_globals(sess, &vm, Some("<repl>"));
}


// Seed host-provided global objects (APP, WEB, BASILICA.MENU) based on runner options
fn seed_host_globals(vm: &mut VM, opts: &RunnerOptions) {
    let names: Vec<String> = vm.globals_snapshot().0;
    let has = |n: &str| names.iter().any(|g| g.eq_ignore_ascii_case(n));
    // APP
    if opts.with_app && has("APP") {
        if let Ok(obj) = vm.registry_mut().make("APP", &[]) {
            let _ = vm.set_global_by_name("APP", Value::Object(obj));
        }
    }
    // WEB
    if opts.with_web && has("WEB") {
        if let Ok(obj) = vm.registry_mut().make("WEB", &[]) {
            let _ = vm.set_global_by_name("WEB", Value::Object(obj));
        }
    }
    // BASILICA.MENU is only meaningful when pending config is present
    if opts.basilica_menu.is_some() && has("BASILICA.MENU") {
        if let Ok(obj) = vm.registry_mut().make("BASILICA.MENU", &[]) {
            let _ = vm.set_global_by_name("BASILICA.MENU", Value::Object(obj));
        }
    }
}
