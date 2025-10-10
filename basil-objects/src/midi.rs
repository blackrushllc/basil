use basil_common::{Result, BasilError};
use std::sync::{Mutex, OnceLock, Arc};
use std::collections::VecDeque;

// MIDI integration using midir for input capture.
pub fn register(_reg: &mut crate::Registry) { let _ = _reg; }

// Error string (per-thread)
thread_local! {
    static LAST_ERR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}
pub fn set_err(msg: impl Into<String>) { LAST_ERR.with(|e| *e.borrow_mut() = msg.into()); }
pub fn take_err() -> String { LAST_ERR.with(|e| e.borrow().clone()) }

#[derive(Clone, Debug)]
pub struct MidiEvent { pub status: u8, pub d1: u8, pub d2: u8 }

struct InPort {
    _name: String,
    q: Arc<Mutex<VecDeque<MidiEvent>>>,
    // Keep the OS connection alive while handle exists
    conn: Option<midir::MidiInputConnection<()>>,
}

static IN_TABLE: OnceLock<Mutex<Vec<Option<InPort>>>> = OnceLock::new();
fn in_tab() -> &'static Mutex<Vec<Option<InPort>>> { IN_TABLE.get_or_init(|| Mutex::new(Vec::new())) }

fn alloc_handle<T>(tab: &mut Vec<Option<T>>, val: T) -> i64 {
    for (i, slot) in tab.iter_mut().enumerate() { if slot.is_none() { *slot = Some(val); return i as i64; } }
    tab.push(Some(val)); (tab.len()-1) as i64
}

pub fn midi_ports() -> Vec<String> {
    let mut out = Vec::new();
    let midi_in = match midir::MidiInput::new("basil-midi-list") {
        Ok(m) => m,
        Err(_) => return out,
    };
    for p in midi_in.ports() { if let Ok(name) = midi_in.port_name(&p) { out.push(name); } }
    out
}

pub fn midi_open_in(substr: &str) -> Result<i64> {
    let mut midi_in = midir::MidiInput::new("basil-midi-in").map_err(|e| BasilError(format!("MIDI_OPEN_IN@: init failed: {e}")))?;
    midi_in.ignore(midir::Ignore::None);
    let ports = midi_in.ports();
    if ports.is_empty() {
        return Err(BasilError("MIDI_OPEN_IN@: no MIDI input ports available".into()));
    }
    // Select port by case-insensitive substring; if empty, use first
    let needle = substr.to_lowercase();
    let mut sel: Option<(midir::MidiInputPort, String)> = None;
    for p in ports.iter() {
        let name = midi_in.port_name(p).unwrap_or_else(|_| "(unknown)".to_string());
        if needle.is_empty() || name.to_lowercase().contains(&needle) {
            sel = Some((p.clone(), name));
            break;
        }
    }
    let (port, name) = match sel {
        Some(t) => t,
        None => {
            // Fallback: first port
            let p = ports[0].clone();
            let n = midi_in.port_name(&p).unwrap_or_else(|_| "(unknown)".to_string());
            (p, n)
        }
    };

    let q: Arc<Mutex<VecDeque<MidiEvent>>> = Arc::new(Mutex::new(VecDeque::new()));
    let q_cb = q.clone();

    let conn = midi_in.connect(&port, "basil-midi-in", move |_ts, msg, _| {
        // Parse note on/off and generic 3-byte channel voice messages
        if msg.len() >= 3 {
            // Heuristic: consume in 3-byte chunks starting at 0 if plausible
            let mut i = 0;
            while i + 2 < msg.len() {
                let status = msg[i];
                let d1 = msg[i+1];
                let d2 = msg[i+2];
                // Only push common 3-byte messages (0x8n..0xEn). Skip system messages.
                if status & 0x80 != 0 && (status & 0xF0) >= 0x80 && (status & 0xF0) <= 0xE0 {
                    if let Ok(mut ql) = q_cb.lock() {
                        ql.push_back(MidiEvent { status, d1, d2 });
                        // Keep queue from growing unbounded
                        if ql.len() > 1024 { let _ = ql.pop_front(); }
                    }
                    i += 3;
                } else {
                    // If not a standard 3-byte message, stop chunking to avoid desync
                    break;
                }
            }
        } else if msg.len() == 2 {
            // Some devices may send 2-byte aftertouch or running-status style data; ignore for now
        } else if msg.len() == 1 {
            // Active sensing, etc. ignored
        }
    }, ()).map_err(|e| BasilError(format!("MIDI_OPEN_IN@: connect failed: {e}")))?;

    let mut tab = in_tab().lock().unwrap();
    let h = alloc_handle(&mut *tab, InPort { _name: name, q, conn: Some(conn) });
    Ok(h)
}

pub fn midi_close(h: i64) -> Result<i64> {
    let mut tab = in_tab().lock().unwrap();
    if let Some(slot) = tab.get_mut(h as usize) {
        if let Some(port) = slot.as_mut() {
            // Drop connection to close
            port.conn.take();
        }
        *slot = None;
        return Ok(0);
    }
    Err(BasilError("Invalid MIDI handle".into()))
}

pub fn midi_poll(h: i64) -> Result<i64> {
    let tab = in_tab().lock().unwrap();
    let port = tab.get(h as usize).and_then(|o| o.as_ref()).ok_or_else(|| BasilError("Invalid MIDI handle".into()))?;
    let q = port.q.lock().map_err(|_| BasilError("MIDI queue lock poisoned".into()))?;
    Ok(q.len() as i64)
}

pub fn midi_get_event(h: i64) -> Result<(u8,u8,u8)> {
    let tab = in_tab().lock().unwrap();
    let port = tab.get(h as usize).and_then(|o| o.as_ref()).ok_or_else(|| BasilError("Invalid MIDI handle".into()))?;
    let mut q = port.q.lock().map_err(|_| BasilError("MIDI queue lock poisoned".into()))?;
    if let Some(ev) = q.pop_front() { Ok((ev.status, ev.d1, ev.d2)) } else { Err(BasilError("No MIDI events".into())) }
}

pub fn midi_cleanup_all() {
    let mut tab = in_tab().lock().unwrap();
    for slot in tab.iter_mut() {
        if let Some(port) = slot.as_mut() {
            port.conn.take();
        }
        *slot = None;
    }
    tab.clear();
}
