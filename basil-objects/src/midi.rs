use basil_common::{Result, BasilError};

// Minimal MIDI helpers (stubs to keep build green without OS MIDI).
// Provides a simple per-handle queue API compatible with the VM-facing functions.

pub fn register(_reg: &mut crate::Registry) { let _ = _reg; }

// Error string (per-thread)
thread_local! {
    static LAST_ERR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}
pub fn set_err(msg: impl Into<String>) { LAST_ERR.with(|e| *e.borrow_mut() = msg.into()); }
pub fn take_err() -> String { LAST_ERR.with(|e| e.borrow().clone()) }

pub fn midi_ports() -> Vec<String> { Vec::new() }

use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct MidiEvent { pub status: u8, pub d1: u8, pub d2: u8 }
struct InPort { _name: String, q: std::collections::VecDeque<MidiEvent> }
static IN_TABLE: OnceLock<Mutex<Vec<Option<InPort>>>> = OnceLock::new();
fn in_tab() -> &'static Mutex<Vec<Option<InPort>>> { IN_TABLE.get_or_init(|| Mutex::new(Vec::new())) }

fn alloc_handle<T>(tab: &mut Vec<Option<T>>, val: T) -> i64 {
    for (i, slot) in tab.iter_mut().enumerate() { if slot.is_none() { *slot = Some(val); return i as i64; } }
    tab.push(Some(val)); (tab.len()-1) as i64
}

pub fn midi_open_in(substr: &str) -> Result<i64> {
    let name = format!("{} (stub)", substr);
    // In stub mode, we create a queue but never receive real events.
    let mut tab = in_tab().lock().unwrap();
    let h = alloc_handle(&mut *tab, InPort { _name: name, q: std::collections::VecDeque::new() });
    Ok(h)
}
pub fn midi_close(h: i64) -> Result<i64> {
    let mut tab = in_tab().lock().unwrap();
    if let Some(slot) = tab.get_mut(h as usize) { *slot = None; }
    Ok(0)
}
pub fn midi_poll(h: i64) -> Result<i64> {
    let tab = in_tab().lock().unwrap();
    let port = tab.get(h as usize).and_then(|o| o.as_ref()).ok_or_else(|| BasilError("Invalid MIDI handle".into()))?;
    Ok(port.q.len() as i64)
}
pub fn midi_get_event(h: i64) -> Result<(u8,u8,u8)> {
    let mut tab = in_tab().lock().unwrap();
    let port = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid MIDI handle".into()))?;
    if let Some(ev) = port.q.pop_front() { Ok((ev.status, ev.d1, ev.d2)) } else { Err(BasilError("No MIDI events".into())) }
}
