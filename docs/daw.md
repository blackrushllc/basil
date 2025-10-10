# Basil DAW Helpers (obj-daw)

High-level one-liners to record, play, monitor audio, capture MIDI, and run a simple live synth.

Status: This build wires the API and stop/error semantics but stubs actual device I/O. Use it to prototype scripts; future updates will integrate CPAL/Midir.

APIs (Basil):
- AUDIO_RECORD%(inputSubstr$, outPath$, seconds%)
- AUDIO_PLAY%(outputSubstr$, filePath$)
- AUDIO_MONITOR%(inputSubstr$, outputSubstr$)
- MIDI_CAPTURE%(portSubstr$, outJsonlPath$)
- SYNTH_LIVE%(midiPortSubstr$, outputSubstr$, poly%)
- DAW_STOP()   ; sets a global stop flag (checked by helpers)
- DAW_ERR$()   ; returns last error string for this thread

Semantics:
- All % helpers return 0 on success, non-zero on failure and set DAW_ERR$().
- DAW_STOP() is polled by long-running helpers and should stop within ~250ms.

Troubleshooting:
- If helpers return 1 with an error like "device I/O not available", rebuild with real backends enabled in the future (CPAL/Midir). On Linux consider JACK vs ALSA; on Windows, WASAPI exclusive/shared modes and sample rate.

Run examples:
- cargo run -p basilc --features obj-daw -- run examples/audio_record.basil
- cargo run -p basilc --features obj-daw -- run examples/audio_play.basil
- cargo run -p basilc --features obj-daw -- run examples/audio_monitor.basil
- cargo run -p basilc --features obj-daw -- run examples/midi_capture.basil
- cargo run -p basilc --features obj-daw -- run examples/synth_live.basil
