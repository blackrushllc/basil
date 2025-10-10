use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, PropDesc, MethodDesc};
use std::sync::atomic::{AtomicBool, Ordering};

// Minimal, compilable MVP for audio object helpers.
// Notes:
// - This module intentionally stubs device I/O to keep build green across CI.
// - Ring buffer and WAV write/read are implemented; device open/start/stop return errors.
// - Higher-level helpers are provided in crate::daw (feature obj-daw).

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

pub fn daw_stop() {
    STOP_FLAG.store(true, Ordering::SeqCst);
}
pub fn daw_stop_clear() {
    STOP_FLAG.store(false, Ordering::SeqCst);
}
pub fn daw_should_stop() -> bool { STOP_FLAG.load(Ordering::SeqCst) }

// Global error string for this thread (very simple implementation for now)
thread_local! {
    static LAST_ERR: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}
pub fn set_err(msg: impl Into<String>) { LAST_ERR.with(|e| *e.borrow_mut() = msg.into()); }
pub fn take_err() -> String { LAST_ERR.with(|e| e.borrow().clone()) }

// Simple ring buffer for interleaved f32 audio frames.
#[derive(Clone)]
pub struct RingBuf {
    buf: Vec<f32>,
    cap: usize,
    read: usize,
    write: usize,
    len: usize,
}
impl RingBuf {
    pub fn new(frames: usize) -> Self {
        let cap = frames.max(1);
        Self { buf: vec![0.0; cap], cap, read: 0, write: 0, len: 0 }
    }
    pub fn push(&mut self, data: &[f32]) -> usize {
        let mut n = 0;
        for &x in data.iter() {
            if self.len == self.cap { break; }
            self.buf[self.write] = x;
            self.write = (self.write + 1) % self.cap;
            self.len += 1; n += 1;
        }
        n
    }
    pub fn pop(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        for o in out.iter_mut() {
            if self.len == 0 { break; }
            *o = self.buf[self.read];
            self.read = (self.read + 1) % self.cap;
            self.len -= 1; n += 1;
        }
        n
    }
}

// Basic synth: polyphonic sine generator
pub struct Synth {
    pub rate: u32,
    pub voices: Vec<Option<Voice>>, // fixed polyphony
    pub phase: f32,
}
#[derive(Copy, Clone)]
pub struct Voice { freq: f32, vel: f32, phase: f32 }

impl Synth {
    pub fn new(rate: u32, poly: usize) -> Self {
        Self { rate, voices: vec![None; poly], phase: 0.0 }
    }
    pub fn note_on(&mut self, note: u8, vel: u8) {
        let freq = 440.0 * 2f32.powf((note as f32 - 69.0)/12.0);
        if let Some(slot) = self.voices.iter_mut().position(|v| v.is_none()) {
            self.voices[slot] = Some(Voice { freq, vel: vel as f32 / 127.0, phase: 0.0 });
        } else {
            // steal voice 0
            self.voices[0] = Some(Voice { freq, vel: vel as f32 / 127.0, phase: 0.0 });
        }
    }
    pub fn note_off(&mut self, note: u8) {
        let freq = 440.0 * 2f32.powf((note as f32 - 69.0)/12.0);
        for v in self.voices.iter_mut() {
            if let Some(voice) = v {
                if (voice.freq - freq).abs() < 0.5 { *v = None; }
            }
        }
    }
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / self.rate as f32;
        for s in out.iter_mut() {
            let mut acc = 0.0f32;
            for v in self.voices.iter_mut() {
                if let Some(voice) = v.as_mut() {
                    voice.phase += 2.0 * std::f32::consts::PI * voice.freq * dt;
                    if voice.phase > 2.0 * std::f32::consts::PI { voice.phase -= 2.0 * std::f32::consts::PI; }
                    acc += voice.vel * voice.phase.sin();
                }
            }
            *s = acc.max(-1.0).min(1.0);
        }
    }
}

// WAV I/O via hound (when enabled)
#[cfg(feature = "obj-audio")]
pub mod wavio {
    use super::*;
    pub fn write_interleaved_f32(path: &str, rate: u32, chans: u16, frames: &[f32]) -> Result<()> {
        let spec = hound::WavSpec { channels: chans, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut wr = hound::WavWriter::create(path, spec)
            .map_err(|e| BasilError(format!("WAV writer open failed: {e}")))?;
        for &s in frames.iter() {
            let v = (s.max(-1.0).min(1.0) * 32767.0) as i16;
            wr.write_sample(v).map_err(|e| BasilError(format!("WAV write failed: {e}")))?;
        }
        wr.finalize().map_err(|e| BasilError(format!("WAV finalize failed: {e}")))?;
        Ok(())
    }
    pub fn read_all_f32(path: &str) -> Result<Vec<f32>> {
        let mut rdr = hound::WavReader::open(path).map_err(|e| BasilError(format!("WAV open failed: {e}")))?;
        let spec = rdr.spec();
        let mut out = Vec::new();
        match spec.sample_format {
            hound::SampleFormat::Int => {
                for s in rdr.samples::<i16>() { let v = s.map_err(|e| BasilError(format!("WAV read failed: {e}")))?; out.push(v as f32 / 32768.0); }
            }
            hound::SampleFormat::Float => {
                for s in rdr.samples::<f32>() { let v = s.map_err(|e| BasilError(format!("WAV read failed: {e}")))?; out.push(v); }
            }
        }
        Ok(out)
    }
}

// Registry hook (placeholder – no object types yet)
pub fn register(_reg: &mut crate::Registry) { let _ = _reg; }

// Helper functions used by VM builtins
pub fn audio_default_rate() -> i64 { 48000 }
pub fn audio_default_chans() -> i64 { 2 }

pub fn audio_inputs() -> Vec<String> { Vec::new() }
pub fn audio_outputs() -> Vec<String> { Vec::new() }

// Device stubs
pub fn audio_open_in(_substr: &str) -> Result<i64> { set_err("AUDIO_OPEN_IN@: device I/O not compiled in"); Err(BasilError("AUDIO_OPEN_IN@: not available".into())) }
pub fn audio_open_out(_substr: &str) -> Result<i64> { set_err("AUDIO_OPEN_OUT@: device I/O not compiled in"); Err(BasilError("AUDIO_OPEN_OUT@: not available".into())) }
pub fn audio_start(_h: i64) -> Result<i64> { set_err("AUDIO_START%: not available"); Err(BasilError("AUDIO_START%: not available".into())) }
pub fn audio_stop(_h: i64) -> Result<i64> { Ok(0) }
pub fn audio_close(_h: i64) -> Result<i64> { Ok(0) }

// Ring buffer handle table (simple)
use std::sync::{Mutex, OnceLock};
struct RbEntry { rb: RingBuf }
static RB_TABLE: OnceLock<Mutex<Vec<Option<RbEntry>>>> = OnceLock::new();
fn rb_tab() -> &'static Mutex<Vec<Option<RbEntry>>> { RB_TABLE.get_or_init(|| Mutex::new(Vec::new())) }
fn alloc_handle<T>(tab: &mut Vec<Option<T>>, val: T) -> i64 {
    for (i, slot) in tab.iter_mut().enumerate() {
        if slot.is_none() { *slot = Some(val); return i as i64; }
    }
    tab.push(Some(val));
    (tab.len()-1) as i64
}

pub fn ring_create(frames: i64) -> Result<i64> {
    let mut tab = rb_tab().lock().unwrap();
    let h = alloc_handle(&mut *tab, RbEntry{ rb: RingBuf::new(frames as usize) });
    Ok(h)
}
pub fn ring_push(h: i64, data: &[f32]) -> Result<i64> {
    let mut tab = rb_tab().lock().unwrap();
    let entry = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
    let n = entry.rb.push(data);
    Ok(n as i64)
}
pub fn ring_pop(h: i64, out: &mut [f32]) -> Result<i64> {
    let mut tab = rb_tab().lock().unwrap();
    let entry = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
    let n = entry.rb.pop(out);
    Ok(n as i64)
}

// WAV writer handle table (accumulate then write on close)
struct WavAcc { path: String, rate: u32, chans: u16, buf: Vec<f32> }
static WAV_TABLE: OnceLock<Mutex<Vec<Option<WavAcc>>>> = OnceLock::new();
fn wav_tab() -> &'static Mutex<Vec<Option<WavAcc>>> { WAV_TABLE.get_or_init(|| Mutex::new(Vec::new())) }

pub fn wav_writer_open(path: &str, rate: i64, chans: i64) -> Result<i64> {
    let mut tab = wav_tab().lock().unwrap();
    let h = alloc_handle(&mut *tab, WavAcc { path: path.to_string(), rate: rate as u32, chans: chans as u16, buf: Vec::new() });
    Ok(h)
}
pub fn wav_writer_write(h: i64, frames: &[f32]) -> Result<i64> {
    let mut tab = wav_tab().lock().unwrap();
    let acc = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid WAV writer handle".into()))?;
    acc.buf.extend_from_slice(frames);
    Ok(0)
}
pub fn wav_writer_close(h: i64) -> Result<i64> {
    let mut tab = wav_tab().lock().unwrap();
    let acc = tab.get_mut(h as usize).and_then(|o| o.take()).ok_or_else(|| BasilError("Invalid WAV writer handle".into()))?;
    #[cfg(feature = "obj-audio")]
    {
        wavio::write_interleaved_f32(&acc.path, acc.rate, acc.chans, &acc.buf)?;
        return Ok(0);
    }
    #[cfg(not(feature = "obj-audio"))]
    {
        let _ = acc; // suppress unused warning
        set_err("WAV writer not available (hound disabled)");
        return Err(BasilError("WAV writer not available".into()));
    }
}

pub fn wav_read_all(path: &str) -> Result<Vec<f32>> {
    #[cfg(feature = "obj-audio")]
    { return wavio::read_all_f32(path); }
    #[cfg(not(feature = "obj-audio"))]
    { set_err("WAV read not available (hound disabled)"); Err(BasilError("WAV read not available".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_push_pop_counts() {
        let mut rb = RingBuf::new(8);
        assert_eq!(rb.push(&[1.0, 2.0, 3.0, 4.0]), 4);
        let mut out = [0.0f32; 3];
        let n = rb.pop(&mut out);
        assert_eq!(n, 3);
        assert_eq!(&out, &[1.0, 2.0, 3.0]);
        assert_eq!(rb.push(&[5.0, 6.0, 7.0, 8.0, 9.0]), 5); // total 6 in buf, cap 8
    }

    #[test]
    fn synth_render_len() {
        let mut s = Synth::new(48000, 8);
        s.note_on(60, 100);
        let mut out = vec![0.0f32; 480];
        s.render(&mut out);
        assert_eq!(out.len(), 480);
        assert!(out.iter().any(|&x| x != 0.0));
        assert!(out.iter().all(|&x| x >= -1.0 && x <= 1.0));
        s.note_off(60);
        let mut out2 = vec![0.0f32; 10];
        s.render(&mut out2);
    }

    #[cfg(feature = "obj-audio")]
    #[test]
    fn wav_writer_roundtrip() {
        let tmp = std::env::temp_dir().join("basil_audio_test.wav");
        let path = tmp.to_string_lossy().to_string();
        let h = wav_writer_open(&path, 48000, 1).unwrap();
        // write 100 samples of a simple sine
        let mut buf = vec![0.0f32; 100];
        let freq = 440.0f32;
        let rate = 48000.0f32;
        for i in 0..100 { buf[i] = (2.0 * std::f32::consts::PI * freq * (i as f32) / rate).sin() * 0.2; }
        wav_writer_write(h, &buf).unwrap();
        wav_writer_close(h).unwrap();
        let frames = wav_read_all(&path).unwrap();
        assert_eq!(frames.len(), 100);
        let _ = std::fs::remove_file(&path);
    }
}
