use anyhow::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;

pub fn monitor(in_q: &str, out_q: &str) -> Result<()> {
    let host = cpal::default_host();
    let indev = host.input_devices()?.find(|d| d.name().map(|n| n.to_lowercase().contains(&in_q.to_lowercase())).unwrap_or(false))
        .ok_or_else(|| anyhow!("No input match"))?;
    let outdev = host.output_devices()?.find(|d| d.name().map(|n| n.to_lowercase().contains(&out_q.to_lowercase())).unwrap_or(false))
        .ok_or_else(|| anyhow!("No output match"))?;

    let in_cfg = indev.default_input_config()?.config();
    let out_cfg = outdev.default_output_config()?.config();

    // Simple ring buffer bridge (assumes f32 devices; OK for many USB class devices)
    let rb = HeapRb::<f32>::new((out_cfg.sample_rate.0 as usize) * 4);
    let (mut prod, mut cons) = rb.split();

    let in_stream = indev.build_input_stream(&in_cfg, move |data: &[f32], _| { let _ = prod.push_slice(data); }, |e| eprintln!("input err: {e}"), None)?;
    let out_stream = outdev.build_output_stream(&out_cfg, move |out: &mut [f32], _| {
        let n = cons.pop_slice(out);
        if n < out.len() { for s in &mut out[n..] { *s = 0.0; } }
    }, |e| eprintln!("output err: {e}"), None)?;

    in_stream.play()?; out_stream.play()?;
    println!("Monitoring '{}' → '{}'", indev.name()?, outdev.name()?);
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
