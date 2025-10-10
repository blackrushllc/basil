use anyhow::*;
use midir::{MidiInput, Ignore};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::time::Instant;
use std::result::Result::Ok;

#[derive(Serialize)]
struct MidiEvent { t_ms: u128, data: Vec<u8> }

pub fn capture_to_jsonl(port_query: &str, path: &str) -> Result<()> {
    let mut inp = MidiInput::new("cli-daw")?; inp.ignore(Ignore::None);
    let ports = inp.ports();
    let mut sel = None;
    for port in &ports {
        let name = inp.port_name(port)?;
        if name.to_lowercase().contains(&port_query.to_lowercase()) { sel = Some(port.clone()); break; }
    }
    let port = sel.ok_or_else(|| anyhow!("MIDI port not found"))?;
    let name = inp.port_name(&port)?; println!("Capturing from MIDI '{}'", name);

    let start = Instant::now();
    let file = Arc::new(Mutex::new(File::create(path)?));
    let file_c = file.clone();

    let _conn = inp.connect(&port, "capture", move |_, msg, _| {
        let ev = MidiEvent { t_ms: start.elapsed().as_millis(), data: msg.to_vec() };
        if let Ok(mut f) = file_c.lock() { let _ = writeln!(f, "{}", serde_json::to_string(&ev).unwrap()); }
    }, ())?;

    println!("Press Q then Enter to stop…");

    // Stop flag set by Ctrl+C or by typing Q + Enter
    let stop = Arc::new(AtomicBool::new(false));
    let stop_c = stop.clone();
    let _ = ctrlc::set_handler(move || { stop_c.store(true, Ordering::SeqCst); });
    {
        let stop_kb = stop.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            loop {
                line.clear();
                if io::stdin().read_line(&mut line).is_err() { break; }
                if line.trim().eq_ignore_ascii_case("q") { stop_kb.store(true, Ordering::SeqCst); break; }
            }
        });
    }

    while !stop.load(Ordering::SeqCst) { std::thread::sleep(std::time::Duration::from_millis(200)); }
    Ok(())
}
