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

/// Reset/cleanup all DAW-related resources: stop flag, audio streams, rings, WAV writers, MIDI connections.
pub fn reset() {
    stop_clear();
    #[cfg(feature = "obj-audio")]
    { let _ = crate::audio::daw_cleanup_all(); }
    #[cfg(feature = "obj-midi")]
    { crate::midi::midi_cleanup_all(); }
}

// High-level helpers
#[cfg(feature = "obj-audio")]
pub fn audio_record(input_sub: &str, out_path: &str, seconds: i64) -> i64 {
    stop_clear();
    if seconds <= 0 { set_err("AUDIO_RECORD%: seconds must be > 0"); return 1; }
    // Open input
    let in_h = match crate::audio::audio_open_in(input_sub) {
        Ok(h) => h,
        Err(e) => { set_err(format!("AUDIO_RECORD%: {}", e)); return 1; }
    };
    // Determine input params
    let (rate, chans) = match crate::audio::audio_in_params(in_h) {
        Ok(rc) => rc,
        Err(_) => (crate::audio::audio_default_rate() as u32, crate::audio::audio_default_chans() as u16),
    };
    // Ring to bridge RT -> writer
    let cap_samples = (rate as usize).saturating_mul(chans as usize).saturating_mul(seconds as usize).saturating_mul(2);
    let ring_h = match crate::audio::ring_create(cap_samples as i64) { Ok(h)=>h, Err(e)=> { set_err(format!("AUDIO_RECORD%: {}", e)); let _=crate::audio::audio_close(in_h); return 1; } };
    if let Err(e) = crate::audio::audio_connect_in_to_ring(in_h, ring_h) { set_err(format!("AUDIO_RECORD%: {}", e)); let _=crate::audio::audio_close(in_h); return 1; }
    if let Err(e) = crate::audio::audio_start(in_h) { set_err(format!("AUDIO_RECORD%: {}", e)); let _=crate::audio::audio_close(in_h); return 1; }
    // WAV writer
    let wr_h = match crate::audio::wav_writer_open(out_path, rate as i64, chans as i64) { Ok(h)=>h, Err(e)=> { set_err(format!("AUDIO_RECORD%: {}", e)); let _=crate::audio::audio_stop(in_h); let _=crate::audio::audio_close(in_h); return 1; } };
    let mut written: usize = 0;
    let target: usize = (rate as usize) * (chans as usize) * (seconds as usize);
    let mut buf: Vec<f32> = vec![0.0; 4096 * (chans as usize).max(1)];
    while written < target {
        if should_stop() { break; }
        let n = match crate::audio::ring_pop(ring_h, &mut buf) { Ok(n)=> n as usize, Err(_)=> 0 };
        if n > 0 {
            if let Err(e) = crate::audio::wav_writer_write(wr_h, &buf[..n]) { set_err(format!("AUDIO_RECORD%: {}", e)); let _=crate::audio::wav_writer_close(wr_h); let _=crate::audio::audio_stop(in_h); let _=crate::audio::audio_close(in_h); return 1; }
            written += n;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let _ = crate::audio::wav_writer_close(wr_h);
    let _ = crate::audio::audio_stop(in_h);
    let _ = crate::audio::audio_close(in_h);
    0
}
#[cfg(not(feature = "obj-audio"))]
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

#[cfg(feature = "obj-audio")]
pub fn audio_monitor(input_sub: &str, output_sub: &str) -> i64 {
    stop_clear();
    // Open devices
    let in_h = match crate::audio::audio_open_in(input_sub) { Ok(h)=>h, Err(e)=> { set_err(format!("AUDIO_MONITOR%: {}", e)); return 1; } };
    let out_h = match crate::audio::audio_open_out(output_sub) { Ok(h)=>h, Err(e)=> { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_close(in_h); return 1; } };
    // Use output params for ring sizing
    let (rate, chans) = match crate::audio::audio_out_params(out_h) { Ok(rc)=>rc, Err(_)=> (crate::audio::audio_default_rate() as u32, crate::audio::audio_default_chans() as u16) };
    // ~100ms ring to reduce latency
    let mut cap_samples = (rate as usize).saturating_mul(chans as usize) / 10;
    if cap_samples < 128 * (chans as usize).max(1) { cap_samples = 128 * (chans as usize).max(1); }
    let ring_h = match crate::audio::ring_create(cap_samples as i64) { Ok(h)=>h, Err(e)=> { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_close(in_h); let _=crate::audio::audio_close(out_h); return 1; } };
    if let Err(e) = crate::audio::audio_connect_in_to_ring(in_h, ring_h) { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_close(in_h); let _=crate::audio::audio_close(out_h); return 1; }
    if let Err(e) = crate::audio::audio_connect_ring_to_out(ring_h, out_h) { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_close(in_h); let _=crate::audio::audio_close(out_h); return 1; }
    if let Err(e) = crate::audio::audio_start(in_h) { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_close(in_h); let _=crate::audio::audio_close(out_h); return 1; }
    if let Err(e) = crate::audio::audio_start(out_h) { set_err(format!("AUDIO_MONITOR%: {}", e)); let _=crate::audio::audio_stop(in_h); let _=crate::audio::audio_close(in_h); let _=crate::audio::audio_close(out_h); return 1; }
    // Block until stopped
    while !should_stop() { std::thread::sleep(std::time::Duration::from_millis(20)); }
    let _ = crate::audio::audio_stop(out_h);
    let _ = crate::audio::audio_stop(in_h);
    let _ = crate::audio::audio_close(out_h);
    let _ = crate::audio::audio_close(in_h);
    0
}
#[cfg(not(feature = "obj-audio"))]
pub fn audio_monitor(_input_sub: &str, _output_sub: &str) -> i64 {
    set_err("AUDIO_MONITOR%: device I/O not available in this build");
    1
}

#[cfg(feature = "obj-midi")]
pub fn midi_capture(port_sub: &str, out_jsonl: &str) -> i64 {
    stop_clear();
    // Open MIDI input
    let in_h = match crate::midi::midi_open_in(port_sub) { Ok(h)=>h, Err(e)=> { set_err(format!("MIDI_CAPTURE%: {}", e)); return 1; } };
    // Open log file
    let mut file = match std::fs::OpenOptions::new().create(true).truncate(true).write(true).open(out_jsonl) {
        Ok(f)=>f, Err(e)=> { set_err(format!("MIDI_CAPTURE%: cannot open file: {e}")); let _=crate::midi::midi_close(in_h); return 1; }
    };
    let t0 = std::time::Instant::now();
    loop {
        if should_stop() { break; }
        let n = match crate::midi::midi_poll(in_h) { Ok(n)=> n, Err(e)=> { set_err(format!("MIDI_CAPTURE%: {}", e)); let _=crate::midi::midi_close(in_h); return 1; } };
        if n > 0 {
            for _ in 0..n {
                match crate::midi::midi_get_event(in_h) {
                    Ok((s,d1,d2)) => {
                        let ms = t0.elapsed().as_millis();
                        let line = format!("{{\"ts\":{ms},\"status\":{s},\"data1\":{d1},\"data2\":{d2}}}\n");
                        if let Err(e) = std::io::Write::write_all(&mut file, line.as_bytes()) { set_err(format!("MIDI_CAPTURE%: write failed: {e}")); let _=crate::midi::midi_close(in_h); return 1; }
                    }
                    Err(_) => {}
                }
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let _ = crate::midi::midi_close(in_h);
    0
}
#[cfg(not(feature = "obj-midi"))]
pub fn midi_capture(_port_sub: &str, _out_jsonl: &str) -> i64 {
    set_err("MIDI_CAPTURE%: MIDI I/O not available in this build");
    1
}

#[cfg(all(feature = "obj-audio", feature = "obj-midi"))]
pub fn synth_live(midi_port_sub: &str, output_sub: &str, poly: i64) -> i64 {
    stop_clear();
    let poly = if poly <= 0 { 8 } else { poly as usize };
    // Open audio out
    let out_h = match crate::audio::audio_open_out(output_sub) { Ok(h)=>h, Err(e)=> { set_err(format!("SYNTH_LIVE%: {}", e)); return 1; } };
    let (rate, chans) = match crate::audio::audio_out_params(out_h) { Ok(rc)=>rc, Err(_)=> (crate::audio::audio_default_rate() as u32, crate::audio::audio_default_chans() as u16) };
    // ~100ms ring to reduce latency
    let mut cap_samples = (rate as usize).saturating_mul(chans as usize) / 10;
    if cap_samples < 128 * (chans as usize).max(1) { cap_samples = 128 * (chans as usize).max(1); }
    let ring_h = match crate::audio::ring_create(cap_samples as i64) { Ok(h)=>h, Err(e)=> { set_err(format!("SYNTH_LIVE%: {}", e)); let _=crate::audio::audio_close(out_h); return 1; } };
    if let Err(e) = crate::audio::audio_connect_ring_to_out(ring_h, out_h) { set_err(format!("SYNTH_LIVE%: {}", e)); let _=crate::audio::audio_close(out_h); return 1; }
    if let Err(e) = crate::audio::audio_start(out_h) { set_err(format!("SYNTH_LIVE%: {}", e)); let _=crate::audio::audio_close(out_h); return 1; }
    // Open MIDI in
    let in_h = match crate::midi::midi_open_in(midi_port_sub) { Ok(h)=>h, Err(e)=> { set_err(format!("SYNTH_LIVE%: {}", e)); let _=crate::audio::audio_stop(out_h); let _=crate::audio::audio_close(out_h); return 1; } };

    // Worker thread: poll MIDI and render synth blocks into ring
    let worker = std::thread::spawn(move || {
        let mut synth = crate::audio::Synth::new(rate, poly);
        let block: usize = 256;
        let mut mono: Vec<f32> = vec![0.0; block];
        let mut inter: Vec<f32> = vec![0.0; block * (chans as usize).max(1)];
        let master_gain: f32 = 0.2; // headroom to avoid clipping with multiple voices
        loop {
            if crate::daw::should_stop() { break; }
            // Drain MIDI events
            match crate::midi::midi_poll(in_h) { Ok(n)=>{
                for _ in 0..n {
                    if let Ok((status,d1,d2)) = crate::midi::midi_get_event(in_h) {
                        let st = status & 0xF0;
                        if st == 0x90 && d2 > 0 { synth.note_on(d1, d2); }
                        else if st == 0x80 || (st == 0x90 && d2 == 0) { synth.note_off(d1); }
                    }
                }
            }, Err(_)=>{} }
            // Render block
            synth.render(&mut mono);
            // Mono -> N channels interleave with master gain
            let chn = (chans as usize).max(1);
            for i in 0..block {
                let s = mono[i] * master_gain;
                for c in 0..chn { inter[i*chn + c] = s; }
            }
            let _ = crate::audio::ring_push(ring_h, &inter);
            // Let scheduler breathe without adding sizeable latency
            std::thread::yield_now();
        }
    });

    // Wait until stopped
    while !should_stop() { std::thread::sleep(std::time::Duration::from_millis(20)); }
    let _ = worker.join();
    let _ = crate::midi::midi_close(in_h);
    let _ = crate::audio::audio_stop(out_h);
    let _ = crate::audio::audio_close(out_h);
    0
}
#[cfg(not(all(feature = "obj-audio", feature = "obj-midi")))]
pub fn synth_live(_midi_port_sub: &str, _output_sub: &str, _poly: i64) -> i64 {
    set_err("SYNTH_LIVE%: audio/MIDI I/O not available in this build");
    1
}
