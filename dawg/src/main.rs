mod audio { pub mod io; pub mod record; pub mod play; pub mod monitor; }
mod midi  { pub mod list; pub mod capture; }
mod synth { pub mod simple; pub mod live; }

use anyhow::*;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name="cli-daw", version, about="Command-line DAW (MVP)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd
}

#[derive(Subcommand)]
enum Cmd {
    /// List audio devices
    Devices,
    /// List MIDI input ports
    MidiPorts,
    /// Record from an input device to a WAV file
    Record { #[arg(long)] input: String, #[arg(long)] out: String, #[arg(long)] seconds: Option<u64> },
    /// Play a WAV file to an output device
    Play   { #[arg(long)] output: String, #[arg(long)] file: String },
    /// Live monitor: input → output
    Monitor{ #[arg(long)] input: String, #[arg(long)] output: String },
    /// Capture MIDI to JSONL
    MidiCapture { #[arg(long)] port: String, #[arg(long)] out: String },
    /// Live synth: MIDI keyboard → built-in synth → audio output
    Synth { #[arg(long)] port: String, #[arg(long)] output: String, #[arg(long, default_value_t=16)] poly: usize },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Devices => audio::io::list_audio_devices()?,
        Cmd::MidiPorts => midi::list::list_midi_ports()?,
        Cmd::Record { input, out, seconds } => audio::record::record_wav(&input, &out, seconds)?,
        Cmd::Play { output, file } => audio::play::play_file(&output, &file)?,
        Cmd::Monitor { input, output } => audio::monitor::monitor(&input, &output)?,
        Cmd::MidiCapture { port, out } => midi::capture::capture_to_jsonl(&port, &out)?,
        Cmd::Synth { port, output, poly } => synth::live::run_live_synth(&port, &output, poly)?,
    }
    Ok(())
}
