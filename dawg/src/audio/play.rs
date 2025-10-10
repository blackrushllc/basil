use anyhow::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::default::get_probe;
use std::fs::File;
use std::result::Result::Ok;

pub fn play_file(device_query: &str, path: &str) -> Result<()> {
    // decode fully to interleaved f32 (compact for MVP)
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = get_probe().format(&Default::default(), mss, &FormatOptions::default(), &Default::default())?;
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(|| anyhow!("no default audio track"))?;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut pcm: Vec<f32> = Vec::new();
    loop {
        match format.next_packet() {
            Ok(packet) => {
                let decoded = decoder.decode(&packet)?;
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        let mut sbuf = SampleBuffer::<f32>::new(buf.capacity() as u64, *buf.spec());
                        sbuf.copy_interleaved_ref(AudioBufferRef::F32(buf));
                        pcm.extend_from_slice(sbuf.samples());
                    }
                    other => {
                        let spec = *other.spec();
                        let mut sbuf = SampleBuffer::<f32>::new(other.capacity() as u64, spec);
                        sbuf.copy_interleaved_ref(other);
                        pcm.extend_from_slice(sbuf.samples());
                    }
                }
            }
            Err(_) => break,
        }
    }

    let host = cpal::default_host();
    let dev = host
        .output_devices()?.find(|d| d.name().map(|n| n.to_lowercase().contains(&device_query.to_lowercase())).unwrap_or(false))
        .ok_or_else(|| anyhow!("No output device matching '{device_query}'"))?;

    let default_config = dev.default_output_config()?;
    let mut pos = 0usize;
    let config: StreamConfig = default_config.config();
    let total_ms = ((pcm.len() as f64) / (config.channels as f64) / (config.sample_rate.0 as f64)) * 1000.0;
    let err_fn = |e| eprintln!("output stream error: {e}");

    let stream = match default_config.sample_format() {
        SampleFormat::F32 => dev.build_output_stream(
            &config, move |out: &mut [f32], _| { for s in out.iter_mut() { *s = if pos < pcm.len() { let v = pcm[pos]; pos += 1; v } else { 0.0 }; } }, err_fn, None,
        )?,
        _ => bail!("Only f32 output supported in MVP"),
    };

    stream.play()?;
    println!("Playing '{}' on '{}'", path, dev.name()?);
    std::thread::sleep(std::time::Duration::from_millis(total_ms.ceil() as u64 + 200));
    Ok(())
}
