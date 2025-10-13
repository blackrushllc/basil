use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, mpsc::{Sender, channel}};

#[derive(Clone, Debug, PartialEq)]
pub enum StepMode {
    None,
    In,
    Over { depth: usize },
    Out { target_depth: usize },
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: String,
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub name: String,
    pub vars: Vec<Variable>,
}

#[derive(Clone, Debug)]
pub struct FrameInfo {
    pub function: String,
    pub file: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum DebugEvent {
    Started,
    StoppedBreakpoint { file: String, line: usize },
    Continued,
    Exited,
    Output(String),
}

#[derive(Debug)]
pub struct DebugState {
    pub paused: bool,
    pub step: StepMode,
    pub subscribers: Vec<Sender<DebugEvent>>,
}

impl DebugState {
    pub fn new() -> Self { Self { paused: false, step: StepMode::None, subscribers: Vec::new() } }
}

pub struct Debugger {
    pub breakpoints: Arc<Mutex<HashMap<String, HashSet<usize>>>>, // filename -> lines
    pub state: Arc<Mutex<DebugState>>, 
}

impl Debugger {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { breakpoints: Arc::new(Mutex::new(HashMap::new())), state: Arc::new(Mutex::new(DebugState::new())) })
    }

    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<DebugEvent> {
        let (tx, rx) = channel::<DebugEvent>();
        if let Ok(mut st) = self.state.lock() { st.subscribers.push(tx); }
        rx
    }

    pub fn emit(&self, ev: DebugEvent) {
        if let Ok(st) = self.state.lock() {
            for tx in &st.subscribers { let _ = tx.send(ev.clone()); }
        }
    }

    pub fn set_breakpoint(&self, file: String, line: usize) {
        if let Ok(mut bp) = self.breakpoints.lock() {
            bp.entry(norm(file)).or_default().insert(line);
        }
    }

    pub fn clear_breakpoint(&self, file: String, line: usize) {
        if let Ok(mut bp) = self.breakpoints.lock() {
            if let Some(set) = bp.get_mut(&norm(file)) { set.remove(&line); }
        }
    }

    pub fn clear_all(&self) { if let Ok(mut bp) = self.breakpoints.lock() { bp.clear(); } }

    pub fn pause(&self) { if let Ok(mut st) = self.state.lock() { st.paused = true; st.step = StepMode::None; } }
    pub fn resume(&self) { if let Ok(mut st) = self.state.lock() { st.paused = false; st.step = StepMode::None; } self.emit(DebugEvent::Continued); }
    pub fn step_in(&self) { if let Ok(mut st) = self.state.lock() { st.paused = false; st.step = StepMode::In; } self.emit(DebugEvent::Continued); }
    pub fn step_over(&self, cur_depth: usize) { if let Ok(mut st) = self.state.lock() { st.paused = false; st.step = StepMode::Over { depth: cur_depth }; } self.emit(DebugEvent::Continued); }
    pub fn step_out(&self, target_depth: usize) { if let Ok(mut st) = self.state.lock() { st.paused = false; st.step = StepMode::Out { target_depth }; } self.emit(DebugEvent::Continued); }

    // Called by VM at each SetLine and on call/ret boundaries
    pub fn check_pause_point(&self, file: &str, line: usize, cur_depth: usize) -> bool {
        // returns whether VM should pause now
        {
            let st = self.state.lock().unwrap();
            if st.paused { return true; }
        }
        // breakpoint?
        let hit_bp = if let Ok(bp) = self.breakpoints.lock() {
            bp.get(&norm(file.to_string())).map(|s| s.contains(&line)).unwrap_or(false)
        } else { false };
        if hit_bp {
            if let Ok(mut st) = self.state.lock() { st.paused = true; st.step = StepMode::None; }
            self.emit(DebugEvent::StoppedBreakpoint { file: file.to_string(), line });
            return true;
        }
        // stepping modes
        let mut should_pause = false;
        if let Ok(mut st) = self.state.lock() {
            match st.step {
                StepMode::None => {}
                StepMode::In => { should_pause = true; st.step = StepMode::None; st.paused = true; }
                StepMode::Over { depth } => {
                    if cur_depth <= depth { should_pause = true; st.step = StepMode::None; st.paused = true; }
                }
                StepMode::Out { target_depth } => {
                    if cur_depth <= target_depth { should_pause = true; st.step = StepMode::None; st.paused = true; }
                }
            }
        }
        if should_pause {
            self.emit(DebugEvent::StoppedBreakpoint { file: file.to_string(), line });
            true
        } else { false }
    }
}

fn norm(p: String) -> String {
    // normalize to uppercase for case-insensitive match on Windows
    p.replace('\\', "/").to_ascii_uppercase()
}
