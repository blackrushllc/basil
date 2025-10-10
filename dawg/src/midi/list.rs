use anyhow::*;
use midir::MidiInput;

pub fn list_midi_ports() -> Result<()> {
    let input = MidiInput::new("cli-daw")?;
    let ports = input.ports();
    for port in &ports {
        let name = input.port_name(port)?;
        println!("• {}", name);
    }
    Ok(())
}
