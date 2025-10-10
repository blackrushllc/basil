# DAWG (cli-daw)

Command-line Digital Audio Workstation MVP in Rust.

![logo](dawg.png)

# Examples:

```bash
# build release
cargo build --release

# run release
cargo run --release

# live synth (play a MIDI keyboard → output to PC speakers)
./target/debug/cli-daw.exe synth --port "LKMK3 MIDI" --output "LC27T55 (NVIDIA High Definition Audio)"

 # show synth help
 ./target/debug/cli-daw.exe help synth

# show help
./target/debug/cli-daw.exe --help

# show version
./target/debug/cli-daw.exe --version

# show devices
./target/debug/cli-daw.exe devices

# show MIDI ports
./target/debug/cli-daw.exe midi-ports

# record 10s from a USB input (use a substring from `devices`)
./target/debug/cli-daw.exe record --input "usb" --out take1.wav --seconds 10

# play a WAV to your USB output
./target/debug/cli-daw.exe play --output "usb" --file take1.wav

# live monitor (pass-through)
./target/debug/cli-daw.exe monitor --input "usb" --output "usb"

# capture MIDI to JSON Lines
./target/debug/cli-daw.exe midi-capture --port "keystation" --out midilog.jsonl

```


## Features
- List audio (CPAL) and MIDI (midir) devices
- Record from an input device to WAV
- Play a WAV file to an output device
- Monitor input → output (pass-through)
- Capture MIDI to JSONL
- **Live synth**: play your MIDI keyboard via built-in poly synth to a selected audio output

## Build & run
```bash
cargo run --release -- devices
cargo run --release -- midi-ports

# Record 10s from a USB input (use a substring from `devices`)
cargo run --release -- record --input "usb" --out take1.wav --seconds 10

# Play a WAV to your USB output
cargo run --release -- play --output "usb" --file take1.wav

# Live monitor input → output
cargo run --release -- monitor --input "usb" --output "usb"

# Capture MIDI to JSON Lines
cargo run --release -- midi-capture --port "keystation" --out midilog.jsonl

# Live synth: MIDI keyboard → output
cargo run --release -- synth --port "keystation" --output "usb" --poly 16
```
