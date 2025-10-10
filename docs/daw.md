# Basil DAW Helpers (obj-daw)

High-level one-liners to record, play, monitor audio, capture MIDI, and run a simple live synth.

Status: AUDIO_PLAY%, AUDIO_RECORD%, AUDIO_MONITOR%, MIDI_CAPTURE%, and SYNTH_LIVE% are implemented when built with the relevant features (obj-daw; enabling obj-audio and/or obj-midi). DAW_STOP() and DAW_ERR$() work across helpers. Internally we use CPAL for audio and midir for MIDI input.

Notes:
- Real-time safety: current callbacks use a ring buffer guarded by a mutex (MVP). For most use cases this is fine; future work will swap in a lock-free SPSC.
- Sample rate/channels: helpers use the device default configs; mono synth output is fanned out to all output channels.
- Stop: helpers poll DAW_STOP() and return within ~20–50ms of a stop signal.

APIs (Basil):
- AUDIO_RECORD%(inputSubstr$, outPath$, seconds%)
- AUDIO_PLAY%(outputSubstr$, filePath$)
- AUDIO_MONITOR%(inputSubstr$, outputSubstr$)
- MIDI_CAPTURE%(portSubstr$, outJsonlPath$)
- SYNTH_LIVE%(midiPortSubstr$, outputSubstr$, poly%)
- DAW_STOP()   ; sets a global stop flag (checked by helpers)
- DAW_ERR$()   ; returns last error string for this thread
- DAW_RESET     ; frees DAW resources (audio streams, MIDI connections, rings, WAV writers) for the current process/thread

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
