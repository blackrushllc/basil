# Basil MIDI Object (obj-midi)

Overview: MIDI input helpers and event queueing accessible from Basil.

Status: Minimal, compilable MVP with stub implementations. Port enumeration and input opening succeed with dummy handles; no real events are generated in this build. Future versions should integrate `midir`.

APIs (Basil):
- MIDI_PORTS$[]
- MIDI_OPEN_IN@(portSubstr$)
- MIDI_POLL%(in@)
- MIDI_GET_EVENT$[](in@)  ; returns [status$, data1$, data2$]
- MIDI_CLOSE%(in@)

Notes:
- Event bytes are decimal strings (e.g., "144", "60", "100").
- Non-zero return codes indicate failure and set DAW_ERR$().

Build:
- Enable: cargo run -p basilc --features obj-midi -- run examples/midi_capture.basil

Troubleshooting:
- On Linux, consider ALSA vs JACK; on Windows/macOS, device sharing policies apply when real MIDI is enabled.
