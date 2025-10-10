use anyhow::*;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn list_audio_devices() -> Result<()> {
    let host = cpal::default_host();

    println!("== Output devices ==");
    for dev in host.output_devices()? { println!("• {}", dev.name()?); }

    println!("\n== Input devices ==");
    for dev in host.input_devices()? { println!("• {}", dev.name()?); }

    Ok(())
}

#[allow(dead_code)]
pub fn find_input_by_substr(q: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    for d in host.input_devices()? { if d.name()?.to_lowercase().contains(&q.to_lowercase()) { return Ok(d); } }
    bail!("No input device matching '{q}'")
}

#[allow(dead_code)]
pub fn find_output_by_substr(q: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    for d in host.output_devices()? { if d.name()?.to_lowercase().contains(&q.to_lowercase()) { return Ok(d); } }
    bail!("No output device matching '{q}'")
}
