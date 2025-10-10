# Basil Audio Object (obj-audio)

Overview: Audio device discovery, simple ring buffers bridging RT ↔ Basil thread, and WAV I/O helpers.

Status: Minimal, compilable MVP. Device I/O is intentionally stubbed in this build to keep CI green. WAV read/write and ring buffers are implemented. In future, integrate `cpal` for real devices.

APIs (Basil):
- AUDIO_OUTPUTS$[]
- AUDIO_INPUTS$[]
- AUDIO_DEFAULT_RATE%()
- AUDIO_DEFAULT_CHANS%()
- AUDIO_OPEN_IN@(deviceSubstr$)        ; stub → returns -1 and sets DAW_ERR$
- AUDIO_OPEN_OUT@(deviceSubstr$)       ; stub → returns -1 and sets DAW_ERR$
- AUDIO_START%(handle@)                ; stub → returns 1 and sets DAW_ERR$
- AUDIO_STOP%(handle@)
- AUDIO_CLOSE%(handle@)
- AUDIO_RING_CREATE@(frames%)
- AUDIO_RING_PUSH%(ring@, frames![])
- AUDIO_RING_POP%(ring@, OUT frames![])
- WAV_WRITER_OPEN@(path$, rate%, chans%)
- WAV_WRITER_WRITE%(writer@, frames![])
- WAV_WRITER_CLOSE%(writer@)
- WAV_READ_ALL![](path$)

Notes:
- Sample format is f32 interleaved for ring/WAV functions. Basil numeric arrays are used to carry these samples.
- Device selection uses case-insensitive substring match (stubbed for now).
- Errors set DAW_ERR$() and return a non-zero code for % functions.

Build:
- Enable: cargo run -p basilc --features obj-audio -- run examples/audio_play.basil

Troubleshooting:
- On Windows, exclusive/shared mode and sample-rate mismatches matter when device I/O is enabled. For now, device calls are stubbed.
