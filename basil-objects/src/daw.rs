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
#[cfg(feature = "obj-audio")]
pub fn audio_play(output_sub: &str, file_path: &str) -> i64 {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc};
    use std::sync::atomic::{AtomicUsize, Ordering};

    stop_clear();

    // Load WAV file (interleaved) into f32 [-1,1]
    let mut reader = match hound::WavReader::open(file_path) {
        Ok(r) => r,
        Err(e) => { set_err(format!("AUDIO_PLAY%: failed to open WAV: {e}")); return 1; }
    };
    let spec = reader.spec();
    let file_rate = spec.sample_rate as u32;
    let file_chans = spec.channels as u16;
    let mut in_samples: Vec<f32> = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Int => {
            for s in reader.samples::<i16>() {
                match s {
                    Ok(v) => in_samples.push(v as f32 / 32768.0),
                    Err(e) => { set_err(format!("AUDIO_PLAY%: WAV read failed: {e}")); return 1; }
                }
            }
        }
        hound::SampleFormat::Float => {
            for s in reader.samples::<f32>() {
                match s {
                    Ok(v) => in_samples.push(v.max(-1.0).min(1.0)),
                    Err(e) => { set_err(format!("AUDIO_PLAY%: WAV read failed: {e}")); return 1; }
                }
            }
        }
    }

    // Pick output device by substring (case-insensitive); default if empty
    let host = cpal::default_host();
    let device = if output_sub.trim().is_empty() {
        match host.default_output_device() {
            Some(d) => d,
            None => { set_err("AUDIO_PLAY%: no default output device"); return 1; }
        }
    } else {
        let needle = output_sub.to_lowercase();
        let mut found = None;
        if let Ok(mut devs) = host.output_devices() {
            for d in devs.by_ref() {
                let name = d.name().unwrap_or_default();
                if name.to_lowercase().contains(&needle) { found = Some(d); break; }
            }
        }
        match found.or_else(|| host.default_output_device()) {
            Some(d) => d,
            None => { set_err(format!("AUDIO_PLAY%: no output device matches '{output_sub}'")); return 1; }
        }
    };

    // Use the device's default config
    let supported = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => { set_err(format!("AUDIO_PLAY%: failed to get device config: {e}")); return 1; }
    };
    let sample_format = supported.sample_format();
    let config = supported.config().clone();
    let dev_rate = config.sample_rate.0;
    let dev_chans = config.channels;

    // Precompute buffer resampled to device rate and mapped to device channels (linear interp)
    let in_ch = file_chans as usize;
    let out_ch = dev_chans as usize;
    let in_frames = if in_ch == 0 { 0 } else { in_samples.len() / in_ch };
    if in_frames == 0 { set_err("AUDIO_PLAY%: WAV has no samples"); return 1; }
    let out_frames = ((in_frames as f64) * (dev_rate as f64) / (file_rate as f64)).round() as usize;
    let mut out_samples = vec![0.0f32; out_frames * out_ch];
    let rate_ratio = (file_rate as f64) / (dev_rate as f64);
    for of in 0..out_frames {
        let src_pos = (of as f64) * rate_ratio; // in source frames
        let i0 = src_pos.floor() as isize;
        let i1 = i0 + 1;
        let frac = (src_pos - (i0 as f64)) as f32;
        for oc in 0..out_ch {
            let sc = if in_ch == 1 { 0 } else { oc % in_ch };
            let idx0 = (i0.max(0) as usize).min(in_frames.saturating_sub(1));
            let idx1 = (i1.max(0) as usize).min(in_frames.saturating_sub(1));
            let s0 = in_samples[idx0 * in_ch + sc];
            let s1 = in_samples[idx1 * in_ch + sc];
            out_samples[of * out_ch + oc] = s0 + (s1 - s0) * frac;
        }
    }

    let data = Arc::new(out_samples);
    let pos = Arc::new(AtomicUsize::new(0));

    let err_fn = |e| { eprintln!("cpal stream error: {e}"); };

    let stream_res = match sample_format {
        cpal::SampleFormat::F32 => {
            let data2 = data.clone();
            let pos2 = pos.clone();
            let cfg = config.clone();
            device.build_output_stream(&cfg, move |buf: &mut [f32], _| {
                let mut i = pos2.load(Ordering::Relaxed);
                for s in buf.iter_mut() {
                    if i < data2.len() { *s = data2[i]; i += 1; } else { *s = 0.0; }
                }
                pos2.store(i, Ordering::Relaxed);
            }, err_fn, None)
        }
        cpal::SampleFormat::I16 => {
            let data2 = data.clone();
            let pos2 = pos.clone();
            let cfg = config.clone();
            device.build_output_stream(&cfg, move |buf: &mut [i16], _| {
                let mut i = pos2.load(Ordering::Relaxed);
                for s in buf.iter_mut() {
                    if i < data2.len() { *s = (data2[i].max(-1.0).min(1.0) * i16::MAX as f32) as i16; i += 1; } else { *s = 0; }
                }
                pos2.store(i, Ordering::Relaxed);
            }, err_fn, None)
        }
        cpal::SampleFormat::U16 => {
            let data2 = data.clone();
            let pos2 = pos.clone();
            let cfg = config.clone();
            device.build_output_stream(&cfg, move |buf: &mut [u16], _| {
                let mut i = pos2.load(Ordering::Relaxed);
                for s in buf.iter_mut() {
                    if i < data2.len() {
                        let v = (data2[i].max(-1.0).min(1.0) * 0.5 + 0.5) * (u16::MAX as f32);
                        *s = v as u16;
                        i += 1;
                    } else { *s = u16::MIN; }
                }
                pos2.store(i, Ordering::Relaxed);
            }, err_fn, None)
        }
        _ => { set_err("AUDIO_PLAY%: unsupported device sample format"); return 1; }
    };

    let stream = match stream_res {
        Ok(s) => s,
        Err(e) => { set_err(format!("AUDIO_PLAY%: failed to build stream: {e}")); return 1; }
    };

    if let Err(e) = stream.play() { set_err(format!("AUDIO_PLAY%: failed to start stream: {e}")); return 1; }

    // Block until done or stop requested
    while pos.load(Ordering::Relaxed) < data.len() {
        if should_stop() { break; }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Stream drops here
    0
}

#[cfg(not(feature = "obj-audio"))]
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
