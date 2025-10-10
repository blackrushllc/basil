use anyhow::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::time::{Duration, Instant};
use std::result::Result::Ok;

pub fn record_wav(device_query: &str, out_path: &str, seconds: Option<u64>) -> Result<()> {
    let host = cpal::default_host();
    let mut dev = None;
    for d in host.input_devices()? {
        if d.name()?.to_lowercase().contains(&device_query.to_lowercase()) { dev = Some(d); break; }
    }
    let dev = dev.ok_or_else(|| anyhow!("No input device matching '{device_query}'"))?;
    let default_config = dev.default_input_config()?;
    let config: StreamConfig = default_config.config();

    let spec = WavSpec { channels: config.channels, sample_rate: config.sample_rate.0, bits_per_sample: 24, sample_format: hound::SampleFormat::Int };

    let writer = WavWriter::create(out_path, spec)?;
    let writer = Arc::new(Mutex::new(writer));

    let start = Instant::now();
    let max = seconds.map(Duration::from_secs);

    let writer_c = writer.clone();
    let err_fn = |e| eprintln!("input stream error: {e}");

    let stream = match default_config.sample_format() {
        SampleFormat::F32 => dev.build_input_stream(
            &config,
            move |data: &[f32], _| {
                if let Ok(mut w) = writer_c.lock() { for &s in data { let v = (s.clamp(-1.0, 1.0) * 8_388_607.0) as i32; let _ = w.write_sample(v); } }
            }, err_fn, None,
        )?,
        SampleFormat::I16 => dev.build_input_stream(
            &config,
            move |data: &[i16], _| { if let Ok(mut w) = writer_c.lock() { for &s in data { let _ = w.write_sample((s as i32) << 8); } } }, err_fn, None,
        )?,
        SampleFormat::U16 => dev.build_input_stream(
            &config,
            move |data: &[u16], _| { if let Ok(mut w) = writer_c.lock() { for &s in data { let centered = s as i32 - 32768; let _ = w.write_sample(centered << 8); } } }, err_fn, None,
        )?,
        _ => bail!("Unsupported sample format"),
    };

    stream.play()?;
    println!("Recording from '{}' → {}", dev.name()?, out_path);
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

    if let Some(limit) = max {
        while start.elapsed() < limit && !stop.load(Ordering::SeqCst) { std::thread::sleep(Duration::from_millis(50)); }
    } else {
        while !stop.load(Ordering::SeqCst) { std::thread::sleep(Duration::from_millis(200)); }
    }

    drop(stream);
    let _ = Arc::try_unwrap(writer).map(|m| m.into_inner().ok().map(|mut w| w.flush()));
    println!("Saved {}", out_path);
    Ok(())
}
