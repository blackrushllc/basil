use basil_common::{Result, BasilError};

// DAW high-level helpers and global error/stop facilities

pub fn register(_reg: &mut crate::Registry) { let _ = _reg; }

thread_local! {
    static LAST_ERR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn set_err(msg: impl Into<String>) { LAST_ERR.with(|e| *e.borrow_mut() = msg.into()); }
pub fn get_err() -> String { LAST_ERR.with(|e| e.borrow().clone()) }

pub fn stop() { crate::audio::daw_stop(); }
pub fn stop_clear() { crate::audio::daw_stop_clear(); }
pub fn should_stop() -> bool { crate::audio::daw_should_stop() }

// High-level stubs (return non-zero and set error when device I/O not available)
pub fn audio_record(_input_sub: &str, _out_path: &str, _seconds: i64) -> i64 {
    set_err("AUDIO_RECORD%: device I/O not available in this build");
    1
}
pub fn audio_play(_output_sub: &str, _file_path: &str) -> i64 {
    set_err("AUDIO_PLAY%: device I/O not available in this build");
    1
}
pub fn audio_monitor(_input_sub: &str, _output_sub: &str) -> i64 {
    set_err("AUDIO_MONITOR%: device I/O not available in this build");
    1
}
pub fn midi_capture(_port_sub: &str, _out_jsonl: &str) -> i64 {
    set_err("MIDI_CAPTURE%: MIDI I/O not available in this build");
    1
}
pub fn synth_live(_midi_port_sub: &str, _output_sub: &str, _poly: i64) -> i64 {
    set_err("SYNTH_LIVE%: audio/MIDI I/O not available in this build");
    1
}
