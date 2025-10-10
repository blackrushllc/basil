use basil_common::{Result, BasilError};
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

// Lock-free SPSC ring using rtrb for interleaved f32 frames.
// We expose push/pop via handle-based API below; no public RingBuf type is needed here.

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
#[cfg(feature = "obj-audio")]
pub fn audio_default_rate() -> i64 {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    if let Some(dev) = host.default_output_device() {
        if let Ok(cfg) = dev.default_output_config() { return cfg.sample_rate().0 as i64; }
    }
    48000
}
#[cfg(not(feature = "obj-audio"))]
pub fn audio_default_rate() -> i64 { 48000 }

#[cfg(feature = "obj-audio")]
pub fn audio_default_chans() -> i64 {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    if let Some(dev) = host.default_output_device() {
        if let Ok(cfg) = dev.default_output_config() { return cfg.channels() as i64; }
    }
    2
}
#[cfg(not(feature = "obj-audio"))]
pub fn audio_default_chans() -> i64 { 2 }

#[cfg(feature = "obj-audio")]
pub fn audio_inputs() -> Vec<String> {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    let mut v = Vec::new();
    if let Ok(devs) = host.input_devices() {
        for d in devs { v.push(d.name().unwrap_or_default()); }
    }
    v
}
#[cfg(not(feature = "obj-audio"))]
pub fn audio_inputs() -> Vec<String> { Vec::new() }

#[cfg(feature = "obj-audio")]
pub fn audio_outputs() -> Vec<String> {
    use cpal::traits::{HostTrait, DeviceTrait};
    let host = cpal::default_host();
    let mut v = Vec::new();
    if let Ok(devs) = host.output_devices() {
        for d in devs { v.push(d.name().unwrap_or_default()); }
    }
    v
}
#[cfg(not(feature = "obj-audio"))]
pub fn audio_outputs() -> Vec<String> { Vec::new() }

// Device I/O
#[cfg(feature = "obj-audio")]
mod devio {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::cell::RefCell;
    use std::sync::{Mutex, Arc};

    pub struct InEntry { pub stream: cpal::Stream, pub ring_sel: Arc<Mutex<Option<rtrb::Producer<f32>>>>, pub chans: u16, pub rate: u32 }
    pub struct OutEntry { pub stream: cpal::Stream, pub ring_sel: Arc<Mutex<Option<rtrb::Consumer<f32>>>>, pub chans: u16, pub rate: u32 }

    thread_local! {
        static IN_TABLE: RefCell<Vec<Option<InEntry>>> = RefCell::new(Vec::new());
        static OUT_TABLE: RefCell<Vec<Option<OutEntry>>> = RefCell::new(Vec::new());
    }
    fn in_tab_mut<F: FnOnce(&mut Vec<Option<InEntry>>)> (f: F) { IN_TABLE.with(|t| f(&mut *t.borrow_mut())) }
    fn out_tab_mut<F: FnOnce(&mut Vec<Option<OutEntry>>)> (f: F) { OUT_TABLE.with(|t| f(&mut *t.borrow_mut())) }

    pub fn cleanup_all() {
        // Dropping streams by clearing the tables releases OS audio resources.
        IN_TABLE.with(|t| t.borrow_mut().clear());
        OUT_TABLE.with(|t| t.borrow_mut().clear());
    }

    fn find_input(substr: &str) -> Option<cpal::Device> {
        let host = cpal::default_host();
        if substr.trim().is_empty() { return host.default_input_device(); }
        let needle = substr.to_lowercase();
        if let Ok(devs) = host.input_devices() {
            for d in devs { if d.name().ok()?.to_lowercase().contains(&needle) { return Some(d); } }
        }
        host.default_input_device()
    }
    fn find_output(substr: &str) -> Option<cpal::Device> {
        let host = cpal::default_host();
        if substr.trim().is_empty() { return host.default_output_device(); }
        let needle = substr.to_lowercase();
        if let Ok(devs) = host.output_devices() {
            for d in devs { if d.name().ok()?.to_lowercase().contains(&needle) { return Some(d); } }
        }
        host.default_output_device()
    }

    pub fn open_in(substr: &str) -> Result<i64> {
        let dev = find_input(substr).ok_or_else(|| BasilError(format!("AUDIO_OPEN_IN@: no input device matches '{}'", substr)))?;
        let cfg = dev.default_input_config().map_err(|e| BasilError(format!("AUDIO_OPEN_IN@: get config failed: {e}")))?;
        let sample_format = cfg.sample_format();
        let mut config = cfg.config().clone();
        // Best-effort smaller buffer for lower latency
        config.buffer_size = cpal::BufferSize::Fixed(256);
        let chans = config.channels;
        let rate = config.sample_rate.0;
        let ring_sel: Arc<Mutex<Option<rtrb::Producer<f32>>>> = Arc::new(Mutex::new(None));
        let ring_sel_cb = ring_sel.clone();
        let err_fn = |e| eprintln!("cpal input error: {e}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let rs = ring_sel_cb.clone();
                dev.build_input_stream(&config, move |buf: &[f32], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(prod) = sel.as_mut() {
                            for &s in buf.iter() {
                                if prod.push(s).is_err() { break; }
                            }
                        }
                    }
                }, err_fn, None)
            }
            cpal::SampleFormat::I16 => {
                // Reuse scratch to avoid per-callback allocations
                let rs = ring_sel_cb.clone();
                let mut scratch: Vec<f32> = Vec::new();
                dev.build_input_stream(&config, move |buf: &[i16], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(prod) = sel.as_mut() {
                            if scratch.len() < buf.len() { scratch.resize(buf.len(), 0.0); }
                            for (i, &s) in buf.iter().enumerate() { scratch[i] = (s as f32) / 32768.0; }
                            for &v in &scratch[..buf.len()] {
                                if prod.push(v).is_err() { break; }
                            }
                        }
                    }
                }, err_fn, None)
            }
            cpal::SampleFormat::U16 => {
                // Reuse scratch to avoid per-callback allocations
                let rs = ring_sel_cb.clone();
                let mut scratch: Vec<f32> = Vec::new();
                dev.build_input_stream(&config, move |buf: &[u16], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(prod) = sel.as_mut() {
                            if scratch.len() < buf.len() { scratch.resize(buf.len(), 0.0); }
                            for (i, &s) in buf.iter().enumerate() { scratch[i] = (s as f32 / u16::MAX as f32) * 2.0 - 1.0; }
                            for &v in &scratch[..buf.len()] {
                                if prod.push(v).is_err() { break; }
                            }
                        }
                    }
                }, err_fn, None)
            }
            _ => return Err(BasilError("AUDIO_OPEN_IN@: unsupported sample format".into())),
        }.map_err(|e| BasilError(format!("AUDIO_OPEN_IN@: build stream failed: {e}")))?;
        let mut handle = -1i64;
        in_tab_mut(|tab| {
            handle = super::alloc_handle(tab, InEntry { stream, ring_sel, chans, rate });
        });
        Ok(handle)
    }

    pub fn open_out(substr: &str) -> Result<i64> {
        let dev = find_output(substr).ok_or_else(|| BasilError(format!("AUDIO_OPEN_OUT@: no output device matches '{}'", substr)))?;
        let cfg = dev.default_output_config().map_err(|e| BasilError(format!("AUDIO_OPEN_OUT@: get config failed: {e}")))?;
        let sample_format = cfg.sample_format();
        let mut config = cfg.config().clone();
        // Best-effort smaller buffer for lower latency
        config.buffer_size = cpal::BufferSize::Fixed(256);
        let ring_sel: Arc<Mutex<Option<rtrb::Consumer<f32>>>> = Arc::new(Mutex::new(None));
        let ring_sel_cb = ring_sel.clone();
        let err_fn = |e| eprintln!("cpal output error: {e}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let rs = ring_sel_cb.clone();
                dev.build_output_stream(&config, move |buf: &mut [f32], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(cons) = sel.as_mut() {
                            for s in buf.iter_mut() { *s = cons.pop().unwrap_or(0.0); }
                        } else {
                            for s in buf.iter_mut() { *s = 0.0; }
                        }
                    } else {
                        for s in buf.iter_mut() { *s = 0.0; }
                    }
                }, err_fn, None)
            }
            cpal::SampleFormat::I16 => {
                // Reuse scratch to avoid per-callback allocations
                let rs = ring_sel_cb.clone();
                let mut scratch: Vec<f32> = Vec::new();
                dev.build_output_stream(&config, move |buf: &mut [i16], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(cons) = sel.as_mut() {
                            if scratch.len() < buf.len() { scratch.resize(buf.len(), 0.0); }
                            for i in 0..buf.len() {
                                let v = cons.pop().unwrap_or(0.0);
                                scratch[i] = v;
                                buf[i] = (v.max(-1.0).min(1.0) * i16::MAX as f32) as i16;
                            }
                        } else { for s in buf.iter_mut() { *s = 0; } }
                    } else { for s in buf.iter_mut() { *s = 0; } }
                }, err_fn, None)
            }
            cpal::SampleFormat::U16 => {
                // Reuse scratch to avoid per-callback allocations
                let rs = ring_sel_cb.clone();
                let mut scratch: Vec<f32> = Vec::new();
                dev.build_output_stream(&config, move |buf: &mut [u16], _| {
                    if let Ok(mut sel) = rs.lock() {
                        if let Some(cons) = sel.as_mut() {
                            if scratch.len() < buf.len() { scratch.resize(buf.len(), 0.0); }
                            for i in 0..buf.len() {
                                let v = cons.pop().unwrap_or(0.0);
                                scratch[i] = v;
                                let vv = (v.max(-1.0).min(1.0) * 0.5 + 0.5) * (u16::MAX as f32);
                                buf[i] = vv as u16;
                            }
                        } else { for s in buf.iter_mut() { *s = u16::MIN; } }
                    } else { for s in buf.iter_mut() { *s = u16::MIN; } }
                }, err_fn, None)
            }
            _ => return Err(BasilError("AUDIO_OPEN_OUT@: unsupported sample format".into())),
        }.map_err(|e| BasilError(format!("AUDIO_OPEN_OUT@: build stream failed: {e}")))?;
        let mut handle = -1i64;
        out_tab_mut(|tab| {
            handle = super::alloc_handle(tab, OutEntry { stream, ring_sel, chans: config.channels, rate: config.sample_rate.0 });
        });
        Ok(handle)
    }

    pub fn start(h: i64) -> Result<i64> {
        let mut found = false;
        let mut err: Option<String> = None;
        in_tab_mut(|tab| {
            if let Some(Some(entry)) = tab.get_mut(h as usize) {
                found = true;
                if let Err(e) = entry.stream.play() { err = Some(format!("{}", e)); }
            }
        });
        if found { return if let Some(e) = err { Err(BasilError(format!("AUDIO_START%: {}", e))) } else { Ok(0) }; }
        out_tab_mut(|tab| {
            if let Some(Some(entry)) = tab.get_mut(h as usize) {
                found = true;
                if let Err(e) = entry.stream.play() { err = Some(format!("{}", e)); }
            }
        });
        if found { return if let Some(e) = err { Err(BasilError(format!("AUDIO_START%: {}", e))) } else { Ok(0) }; }
        Err(BasilError("AUDIO_START%: invalid handle".into()))
    }
    pub fn stop(h: i64) -> Result<i64> {
        let mut found = false;
        let mut err: Option<String> = None;
        in_tab_mut(|tab| {
            if let Some(Some(entry)) = tab.get_mut(h as usize) {
                found = true;
                if let Err(e) = entry.stream.pause() { err = Some(format!("{}", e)); }
            }
        });
        if found { return if let Some(e) = err { Err(BasilError(format!("AUDIO_STOP%: {}", e))) } else { Ok(0) }; }
        out_tab_mut(|tab| {
            if let Some(Some(entry)) = tab.get_mut(h as usize) {
                found = true;
                if let Err(e) = entry.stream.pause() { err = Some(format!("{}", e)); }
            }
        });
        if found { return if let Some(e) = err { Err(BasilError(format!("AUDIO_STOP%: {}", e))) } else { Ok(0) }; }
        Err(BasilError("AUDIO_STOP%: invalid handle".into()))
    }
    pub fn close(h: i64) -> Result<i64> {
        let mut closed = false;
        in_tab_mut(|tab| {
            if let Some(slot) = tab.get_mut(h as usize) { *slot = None; closed = true; }
        });
        if !closed {
            out_tab_mut(|tab| {
                if let Some(slot) = tab.get_mut(h as usize) { *slot = None; closed = true; }
            });
        }
        if closed { Ok(0) } else { Err(BasilError("AUDIO_CLOSE%: invalid handle".into())) }
    }

    pub fn in_params(h: i64) -> Result<(u32,u16)> {
        let mut res: Option<(u32,u16)> = None;
        IN_TABLE.with(|t| {
            let tab = t.borrow();
            if let Some(Some(e)) = tab.get(h as usize) { res = Some((e.rate, e.chans)); }
        });
        res.ok_or_else(|| BasilError("Invalid input handle".into()))
    }
    pub fn out_params(h: i64) -> Result<(u32,u16)> {
        let mut res: Option<(u32,u16)> = None;
        OUT_TABLE.with(|t| {
            let tab = t.borrow();
            if let Some(Some(e)) = tab.get(h as usize) { res = Some((e.rate, e.chans)); }
        });
        res.ok_or_else(|| BasilError("Invalid output handle".into()))
    }

    pub fn connect_in_to_ring(in_h: i64, ring_h: i64) -> Result<i64> {
        // Take the Producer half from the ring handle and install into the input stream entry
        let mut prod_opt = {
            let mut tab = super::rb_tab().lock().unwrap();
            let e = tab.get_mut(ring_h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
            e.prod.take()
        };
        let mut ok = false;
        in_tab_mut(|tab| {
            if let Some(entry) = tab.get_mut(in_h as usize).and_then(|o| o.as_mut()) {
                if let Ok(mut sel) = entry.ring_sel.lock() {
                    if sel.is_none() {
                        if let Some(p) = prod_opt.take() { *sel = Some(p); ok = true; }
                    }
                }
            }
        });
        if ok { Ok(0) } else { Err(BasilError("Invalid input handle or ring producer already connected".into())) }
    }
    pub fn connect_ring_to_out(ring_h: i64, out_h: i64) -> Result<i64> {
        // Take the Consumer half from the ring handle and install into the output stream entry
        let mut cons_opt = {
            let mut tab = super::rb_tab().lock().unwrap();
            let e = tab.get_mut(ring_h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
            e.cons.take()
        };
        let mut ok = false;
        out_tab_mut(|tab| {
            if let Some(entry) = tab.get_mut(out_h as usize).and_then(|o| o.as_mut()) {
                if let Ok(mut sel) = entry.ring_sel.lock() {
                    if sel.is_none() {
                        if let Some(c) = cons_opt.take() { *sel = Some(c); ok = true; }
                    }
                }
            }
        });
        if ok { Ok(0) } else { Err(BasilError("Invalid output handle or ring consumer already connected".into())) }
    }
}

#[cfg(not(feature = "obj-audio"))]
// Device stubs
pub fn audio_open_in(_substr: &str) -> Result<i64> { set_err("AUDIO_OPEN_IN@: device I/O not compiled in"); Err(BasilError("AUDIO_OPEN_IN@: not available".into())) }
#[cfg(not(feature = "obj-audio"))]
pub fn audio_open_out(_substr: &str) -> Result<i64> { set_err("AUDIO_OPEN_OUT@: device I/O not compiled in"); Err(BasilError("AUDIO_OPEN_OUT@: not available".into())) }
#[cfg(not(feature = "obj-audio"))]
pub fn audio_start(_h: i64) -> Result<i64> { set_err("AUDIO_START%: not available"); Err(BasilError("AUDIO_START%: not available".into())) }
#[cfg(not(feature = "obj-audio"))]
pub fn audio_stop(_h: i64) -> Result<i64> { Ok(0) }
#[cfg(not(feature = "obj-audio"))]
pub fn audio_close(_h: i64) -> Result<i64> { Ok(0) }

#[cfg(feature = "obj-audio")]
pub fn audio_open_in(substr: &str) -> Result<i64> { devio::open_in(substr) }
#[cfg(feature = "obj-audio")]
pub fn audio_open_out(substr: &str) -> Result<i64> { devio::open_out(substr) }
#[cfg(feature = "obj-audio")]
pub fn audio_start(h: i64) -> Result<i64> { devio::start(h) }
#[cfg(feature = "obj-audio")]
pub fn audio_stop(h: i64) -> Result<i64> { devio::stop(h) }
#[cfg(feature = "obj-audio")]
pub fn audio_close(h: i64) -> Result<i64> { devio::close(h) }
#[cfg(feature = "obj-audio")]
pub fn audio_connect_in_to_ring(in_h: i64, ring_h: i64) -> Result<i64> { devio::connect_in_to_ring(in_h, ring_h) }
#[cfg(feature = "obj-audio")]
pub fn audio_connect_ring_to_out(ring_h: i64, out_h: i64) -> Result<i64> { devio::connect_ring_to_out(ring_h, out_h) }
#[cfg(feature = "obj-audio")]
pub fn audio_in_params(h: i64) -> Result<(u32,u16)> { devio::in_params(h) }
#[cfg(feature = "obj-audio")]
pub fn audio_out_params(h: i64) -> Result<(u32,u16)> { devio::out_params(h) }

/// Cleanup all audio-related resources (streams, rings, wav writers) and clear stop flag.
pub fn daw_cleanup_all() {
    daw_stop_clear();
    #[cfg(feature = "obj-audio")]
    {
        devio::cleanup_all();
    }
    // Clear ring buffers
    {
        let mut tab = rb_tab().lock().unwrap();
        tab.clear();
    }
    // Clear WAV writer accumulators
    {
        let mut tab = wav_tab().lock().unwrap();
        tab.clear();
    }
}

// Ring buffer handle table (lock-free rtrb SPSC)
use std::sync::{Mutex, OnceLock};
struct RbEntry { prod: Option<rtrb::Producer<f32>>, cons: Option<rtrb::Consumer<f32>> }
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
    let capacity = (frames as usize).max(1);
    let (prod, cons) = rtrb::RingBuffer::<f32>::new(capacity);
    let mut tab = rb_tab().lock().unwrap();
    let h = alloc_handle(&mut *tab, RbEntry{ prod: Some(prod), cons: Some(cons) });
    Ok(h)
}
pub fn ring_push(h: i64, data: &[f32]) -> Result<i64> {
    let mut tab = rb_tab().lock().unwrap();
    let entry = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
    let prod = entry.prod.as_mut().ok_or_else(|| BasilError("Ring producer not available (already connected to input)".into()))?;
    let mut n = 0i64;
    for &x in data.iter() {
        if prod.push(x).is_err() { break; }
        n += 1;
    }
    Ok(n)
}
pub fn ring_pop(h: i64, out: &mut [f32]) -> Result<i64> {
    let mut tab = rb_tab().lock().unwrap();
    let entry = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid ring handle".into()))?;
    let cons = entry.cons.as_mut().ok_or_else(|| BasilError("Ring consumer not available (already connected to output)".into()))?;
    let mut n = 0i64;
    for o in out.iter_mut() {
        if let Ok(x) = cons.pop() { *o = x; n += 1; } else { break; }
    }
    Ok(n)
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

// --- Low-level Synth handle API (obj-audio) ---
static SYNTH_TABLE: OnceLock<Mutex<Vec<Option<Synth>>>> = OnceLock::new();
fn synth_tab() -> &'static Mutex<Vec<Option<Synth>>> { SYNTH_TABLE.get_or_init(|| Mutex::new(Vec::new())) }

#[cfg(feature = "obj-audio")]
pub fn synth_new(rate: i64, poly: i64) -> Result<i64> {
    let mut tab = synth_tab().lock().unwrap();
    let rate_u = if rate <= 0 { 48000 } else { rate as u32 };
    let poly_u = if poly <= 0 { 8 } else { poly as usize };
    let h = alloc_handle(&mut *tab, Synth::new(rate_u, poly_u));
    Ok(h)
}
#[cfg(feature = "obj-audio")]
pub fn synth_note_on(h: i64, note: i64, vel: i64) -> Result<i64> {
    let mut tab = synth_tab().lock().unwrap();
    let s = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid synth handle".into()))?;
    s.note_on(note as u8, vel.max(0).min(127) as u8);
    Ok(0)
}
#[cfg(feature = "obj-audio")]
pub fn synth_note_off(h: i64, note: i64) -> Result<i64> {
    let mut tab = synth_tab().lock().unwrap();
    let s = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid synth handle".into()))?;
    s.note_off(note as u8);
    Ok(0)
}
#[cfg(feature = "obj-audio")]
pub fn synth_render(h: i64, out: &mut [f32]) -> Result<i64> {
    let mut tab = synth_tab().lock().unwrap();
    let s = tab.get_mut(h as usize).and_then(|o| o.as_mut()).ok_or_else(|| BasilError("Invalid synth handle".into()))?;
    s.render(out);
    Ok(out.len() as i64)
}
#[cfg(feature = "obj-audio")]
pub fn synth_delete(h: i64) -> Result<i64> {
    let mut tab = synth_tab().lock().unwrap();
    if let Some(slot) = tab.get_mut(h as usize) { *slot = None; Ok(0) } else { Err(BasilError("Invalid synth handle".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_push_pop_counts() {
        let h = ring_create(8).unwrap();
        assert_eq!(ring_push(h, &[1.0, 2.0, 3.0, 4.0]).unwrap(), 4);
        let mut out = [0.0f32; 3];
        let n = ring_pop(h, &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&out, &[1.0, 2.0, 3.0]);
        // After popping 3, one element remains; pushing five should accept all five (cap 8)
        assert_eq!(ring_push(h, &[5.0, 6.0, 7.0, 8.0, 9.0]).unwrap(), 5);
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
