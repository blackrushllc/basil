use anyhow::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use midir::{MidiInput, Ignore};
use ringbuf::HeapRb;
use crate::synth::simple::Synth;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;

#[derive(Clone, Copy)]
enum Ev { NoteOn(u8,u8), NoteOff(u8) }

pub fn run_live_synth(midi_port_q: &str, output_q: &str, poly: usize) -> Result<()> {
    // ---- Pick output device and config
    let host = cpal::default_host();
    let outdev = host.output_devices()?.find(|d|
        d.name().map(|n| n.to_lowercase().contains(&output_q.to_lowercase())).unwrap_or(false)
    ).ok_or_else(|| anyhow!("No output device matching '{output_q}'"))?;

    let out_cfg_any = outdev.default_output_config()?; // prefer device default
    let sr = out_cfg_any.sample_rate().0 as f32;
    let chans = out_cfg_any.channels() as usize; // typically 2
    let cfg: StreamConfig = out_cfg_any.config();

    // ---- Create synth + lock-free event queue
    let mut synth = Synth::new(sr, poly.max(1));
    let rb = HeapRb::<Ev>::new(2048);
    let (mut prod, mut cons) = rb.split();

    // ---- Hook MIDI input
    let mut inp = MidiInput::new("cli-daw-synth")?; inp.ignore(Ignore::None);
    let ports = inp.ports();
    let mut sel = None;
    for port in &ports {
        let name = inp.port_name(port)?;
        if name.to_lowercase().contains(&midi_port_q.to_lowercase()) { sel = Some(port.clone()); break; }
    }
    let port = sel.ok_or_else(|| anyhow!("MIDI port not found"))?;
    let port_name = inp.port_name(&port)?;

    let _conn = inp.connect(&port, "live-synth", move |_, msg, _| {
        if msg.is_empty() { return; }
        let status = msg[0] & 0xF0;
        match status {
            0x90 => { // note on
                let note = msg.get(1).copied().unwrap_or(0);
                let vel  = msg.get(2).copied().unwrap_or(0);
                if vel == 0 { let _ = prod.push(Ev::NoteOff(note)); } else { let _ = prod.push(Ev::NoteOn(note, vel)); }
            }
            0x80 => { let note = msg.get(1).copied().unwrap_or(0); let _ = prod.push(Ev::NoteOff(note)); }
            _ => {}
        }
    }, ())?;

    // ---- Build output stream (f32)
    let err_fn = |e| eprintln!("output stream error: {e}");

    let stream = match out_cfg_any.sample_format() {
        SampleFormat::F32 => outdev.build_output_stream(
            &cfg,
            move |out: &mut [f32], _| {
                // Drain pending MIDI events
                while let Some(ev) = cons.pop() {
                    match ev { Ev::NoteOn(n,v) => synth.note_on(n,v), Ev::NoteOff(n) => synth.note_off(n) }
                }
                // Render
                // out is interleaved; render a mono temp then spread to chans
                let frames = out.len() / chans;
                let mut tmp = vec![0.0f32; frames];
                synth.render_mono(&mut tmp);
                for (i, frame) in tmp.iter().enumerate() {
                    for ch in 0..chans { out[i*chans + ch] = *frame; }
                }
            }, err_fn, None
        )?,
        _ => bail!("Only f32 output supported for live synth in MVP"),
    };

    stream.play()?;
    println!("Live synth running: MIDI '{}' → Audio '{}' @ {} Hz ({} ch), poly={}",
        port_name, outdev.name()?, sr as u32, chans, poly);
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
