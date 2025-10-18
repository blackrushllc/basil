use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use basil_bytecode::{BasicObject, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};
use basil_objects::{Registry, TypeInfo};
use crossbeam_channel::Sender;
use parking_lot::Mutex;

// ---------- Config and messaging types ----------
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub mode: String, // "run"|"test"|"cli"
    pub kind: String, // "bare"|"file"
    pub path: Option<String>,
    pub args: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PendingConfig {
    pub cli_scripts: Vec<MenuItem>,
    pub gui_scripts: Vec<MenuItem>,
    pub saved: bool,
}

#[derive(Clone, Debug)]
pub enum HostRequest {
    // APP.* surface
    AppAlert(String),
    AppStartAnim,
    AppStopAnim,
    // WEB.* surface
    WebSetHtml(String),
    WebEval(String),
    // Registration of event route: (event, id) -> Basil label
    WebOn { event: String, id: String, label: String },
}

// ---------- Thread-local embed context ----------
thread_local! {
    static TL_HOST_TX: RefCell<Option<Sender<HostRequest>>> = RefCell::new(None);
    static TL_PENDING: RefCell<Option<Arc<Mutex<PendingConfig>>>> = RefCell::new(None);
}

pub fn set_thread_context(host_tx: Option<Sender<HostRequest>>, pending_menu: Option<Arc<Mutex<PendingConfig>>>) {
    TL_HOST_TX.with(|c| *c.borrow_mut() = host_tx);
    TL_PENDING.with(|c| *c.borrow_mut() = pending_menu);
}

// ---------- Registration entry ----------
pub fn register_hosts(reg: &mut Registry) {
    reg.register("APP", TypeInfo { factory: app_factory, descriptor: app_desc, constants: app_consts });
    reg.register("WEB", TypeInfo { factory: web_factory, descriptor: web_desc, constants: web_consts });
    reg.register("BASILICA.MENU", TypeInfo { factory: menu_factory, descriptor: menu_desc, constants: menu_consts });
}

// ---------- APP object ----------
struct AppObj { tx: Option<Sender<HostRequest>> }

fn app_factory(_args: &[Value]) -> Result<Rc<RefCell<dyn BasicObject>>> {
    let tx = TL_HOST_TX.with(|c| c.borrow().clone());
    Ok(Rc::new(RefCell::new(AppObj { tx })))
}
fn app_desc() -> ObjectDescriptor {
    ObjectDescriptor { type_name: "APP".into(), version: "1.0".into(), summary: "Basilica host app".into(), properties: vec![], methods: vec![
        MethodDesc { name: "OPEN_FILE$".into(), arity: 0, arg_names: vec![], return_type: "STRING".into() },
        MethodDesc { name: "ALERT%".into(), arity: 1, arg_names: vec!["msg$".into()], return_type: "INT".into() },
        MethodDesc { name: "START_ANIM%".into(), arity: 0, arg_names: vec![], return_type: "INT".into() },
        MethodDesc { name: "STOP_ANIM%".into(), arity: 0, arg_names: vec![], return_type: "INT".into() },
    ], examples: vec![] }
}
fn app_consts() -> Vec<(String, Value)> { vec![] }

impl BasicObject for AppObj {
    fn type_name(&self) -> &str { "APP" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("Unknown property".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("Unknown property".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "OPEN_FILE$" => {
                if let Some(p) = rfd::FileDialog::new().pick_file() { Ok(Value::Str(p.to_string_lossy().to_string())) }
                else { Ok(Value::Str(String::new())) }
            }
            "ALERT%" => {
                let msg = match args.get(0) { Some(Value::Str(s))=>s.clone(), Some(v)=>format!("{}", v), None=>String::new() };
                if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::AppAlert(msg)); }
                Ok(Value::Int(1))
            }
            "START_ANIM%" => { if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::AppStartAnim); } Ok(Value::Int(1)) }
            "STOP_ANIM%"  => { if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::AppStopAnim); } Ok(Value::Int(1)) }
            other => Err(BasilError(format!("APP.{} not found", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { app_desc() }
}

// ---------- WEB object ----------
struct WebObj { tx: Option<Sender<HostRequest>> }

fn web_factory(_args: &[Value]) -> Result<Rc<RefCell<dyn BasicObject>>> {
    let tx = TL_HOST_TX.with(|c| c.borrow().clone());
    Ok(Rc::new(RefCell::new(WebObj { tx })))
}
fn web_desc() -> ObjectDescriptor {
    ObjectDescriptor { type_name: "WEB".into(), version: "1.0".into(), summary: "Basilica webview".into(), properties: vec![], methods: vec![
        MethodDesc { name: "SET_HTML$".into(), arity: 1, arg_names: vec!["html$".into()], return_type: "STRING".into() },
        MethodDesc { name: "EVAL$".into(), arity: 1, arg_names: vec!["js$".into()], return_type: "STRING".into() },
        MethodDesc { name: "ON%".into(), arity: 3, arg_names: vec!["event$".into(), "id$".into(), "label".into()], return_type: "INT".into() },
    ], examples: vec![] }
}
fn web_consts() -> Vec<(String, Value)> { vec![] }

impl BasicObject for WebObj {
    fn type_name(&self) -> &str { "WEB" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("Unknown property".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("Unknown property".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "SET_HTML$" => { let html = to_s(args.get(0)); if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::WebSetHtml(html)); } Ok(Value::Str(String::new())) }
            "EVAL$"     => { let js   = to_s(args.get(0)); if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::WebEval(js)); } Ok(Value::Str(String::new())) }
            "ON%"       => {
                let event = to_s(args.get(0)); let id = to_s(args.get(1)); let label = to_s(args.get(2));
                if let Some(tx) = &self.tx { let _ = tx.send(HostRequest::WebOn { event, id, label }); }
                Ok(Value::Int(1))
            }
            other => Err(BasilError(format!("WEB.{} not found", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { web_desc() }
}

// ---------- BASILICA.MENU object ----------
struct MenuObj { pending: Option<Arc<Mutex<PendingConfig>>> }

fn menu_factory(_args: &[Value]) -> Result<Rc<RefCell<dyn BasicObject>>> {
    let pending = TL_PENDING.with(|c| c.borrow().clone());
    Ok(Rc::new(RefCell::new(MenuObj { pending })))
}
fn menu_desc() -> ObjectDescriptor {
    ObjectDescriptor { type_name: "BASILICA.MENU".into(), version: "1.0".into(), summary: "Basilica bootstrap menu editor".into(), properties: vec![], methods: vec![
        MethodDesc { name: "CLEAR%".into(), arity: 0, arg_names: vec![], return_type: "INT".into() },
        MethodDesc { name: "ADD_CLI%".into(), arity: 5, arg_names: vec!["name$".into(), "mode$".into(), "kind$".into(), "path$".into(), "args$".into()], return_type: "INT".into() },
        MethodDesc { name: "ADD_GUI%".into(), arity: 5, arg_names: vec!["name$".into(), "mode$".into(), "kind$".into(), "path$".into(), "args$".into()], return_type: "INT".into() },
        MethodDesc { name: "SAVE%".into(), arity: 0, arg_names: vec![], return_type: "INT".into() },
    ], examples: vec![] }
}
fn menu_consts() -> Vec<(String, Value)> { vec![] }

impl BasicObject for MenuObj {
    fn type_name(&self) -> &str { "BASILICA.MENU" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("Unknown property".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("Unknown property".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        let Some(p) = &self.pending else { return Err(BasilError("BASILICA.MENU not enabled in this run".into())); };
        match method.to_ascii_uppercase().as_str() {
            "CLEAR%" => { let mut m = p.lock(); m.cli_scripts.clear(); m.gui_scripts.clear(); m.saved = false; Ok(Value::Int(1)) }
            "ADD_CLI%" => { add_item(p, true, args); Ok(Value::Int(1)) }
            "ADD_GUI%" => { add_item(p, false, args); Ok(Value::Int(1)) }
            "SAVE%" => { let mut m = p.lock(); m.saved = true; Ok(Value::Int(1)) }
            other => Err(BasilError(format!("BASILICA.MENU.{} not found", other)))
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { menu_desc() }
}

// ---------- helpers ----------
fn add_item(pending: &Arc<Mutex<PendingConfig>>, cli: bool, args: &[Value]) {
    let name = to_s(args.get(0));
    let mode = to_s(args.get(1));
    let kind = to_s(args.get(2));
    let path = to_s(args.get(3));
    let argz = to_s(args.get(4));
    let mut p = pending.lock();
    let base = slugify(&name);
    let mut id = base.clone();
    let mut i = 2;
    while p.cli_scripts.iter().any(|m| m.id == id) || p.gui_scripts.iter().any(|m| m.id == id) {
        id = format!("{}-{}", base, i); i += 1;
    }
    let item = MenuItem { id, name, mode, kind: kind.clone(), path: if kind.eq_ignore_ascii_case("file") && !path.is_empty() { Some(path) } else { None }, args: if argz.is_empty() { None } else { Some(argz) } };
    if cli { p.cli_scripts.push(item); } else { p.gui_scripts.push(item); }
}
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() { out.push(c.to_ascii_lowercase()); }
        else if c.is_whitespace() || c == '-' || c == '_' { if !out.ends_with('-') { out.push('-'); } }
    }
    out.trim_matches('-').to_string()
}
fn to_s(v: Option<&Value>) -> String { match v { Some(Value::Str(s))=>s.clone(), Some(x)=>format!("{}", x), None=>String::new() } }
