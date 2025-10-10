# 🌿 Basil MIDI Programming Guide

## 🌱 PART 1: High Level Basil Midi/Audio program examples

### **Basil** examples that use the `obj-daw` "helpers" only (so they “just work” when that feature is enabled).

---

### `examples/01_record_then_play.basil`

```basic
REM Record 5 seconds, then play it back;
PRINTLN "Recording 5s from USB input to take1.wav…";
rc% = AUDIO_RECORD%("usb", "take1.wav", 5);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

PRINTLN "Playing take1.wav on USB output…";
rc% = AUDIO_PLAY%("usb", "take1.wav");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$();

PRINTLN "Done.";
```

---

### `examples/02_live_monitor_until_key.basil`

```basic
REM Input → Output pass-through until a keypress (or DAW_STOP from another console);
PRINTLN "Live monitor: input 'usb' → output 'usb'. Press any key to stop.";
STARTED% = 0;

REM Run monitor in a background-ish way: start it, then poll for a key and request stop;
rc% = AUDIO_MONITOR%("usb", "usb");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

REM The helper is blocking on purpose; typical pattern is to trigger from another console:
REM cargo run … -- run examples/02_live_monitor_until_key.basil  (in one terminal)
REM and from another terminal call DAW_STOP() via a tiny script.
REM If you prefer single-process control, use low-level obj-audio instead.
```

> Tip: If you want same-process key handling, use the low-level ring/stream API (obj-audio). This file shows the simple helper semantics.

---

### `examples/03_midi_capture_to_jsonl.basil`

```basic
REM Log incoming MIDI to JSON Lines until you stop it from another console;
PRINTLN "Capturing MIDI from 'launchkey' to midilog.jsonl…";
rc% = MIDI_CAPTURE%("launchkey", "midilog.jsonl");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

REM Like monitor, this helper is blocking; call DAW_STOP() from another Basil snippet to end.
```

---

### `examples/04_live_synth.basil`

```basic
REM Play your MIDI keyboard through a built-in poly synth → selected output;
PRINTLN "Live synth: MIDI 'launchkey' → output 'usb' (poly=16).";
rc% = SYNTH_LIVE%("launchkey", "usb", 16);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$();
```

---

### `examples/05_stop_helpers_now.basil`

```basic
REM Send a global stop to any blocking helper (monitor, capture, live synth);
PRINTLN "Stopping all running DAW helpers…";
DAW_STOP();
PRINTLN "Stop requested.";
```

---

### `examples/06_quick_voice_memo.basil`

```basic
REM One-key voice memo: record N seconds and auto-play back;
DUR% = 8;
PRINT "Recording "; PRINT DUR%; PRINTLN "s to memo.wav…";
rc% = AUDIO_RECORD%("usb", "memo.wav", DUR%);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

PRINTLN "Playback memo.wav…";
rc% = AUDIO_PLAY%("usb", "memo.wav");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$();

PRINTLN "Done.";
```

---

### `examples/07_a_b_compare_take.basil`

```basic
REM Record two short takes and A/B them back-to-back;
PRINTLN "Take A (3s)…";
rc% = AUDIO_RECORD%("usb", "takeA.wav", 3);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

PRINTLN "Take B (3s)…";
rc% = AUDIO_RECORD%("usb", "takeB.wav", 3);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

PRINTLN "Playing A then B…";
rc% = AUDIO_PLAY%("usb", "takeA.wav"); IF rc% <> 0 THEN PRINTLN DAW_ERR$();
rc% = AUDIO_PLAY%("usb", "takeB.wav"); IF rc% <> 0 THEN PRINTLN DAW_ERR$();

PRINTLN "A/B complete.";
```

---

### `examples/08_midi_blink_recorder.basil`

```basic
REM Capture MIDI for ~10 seconds and stop it with a separate DAW_STOP call if desired;
PRINTLN "Start MIDI capture (10s window suggested). From another terminal you can stop early with DAW_STOP().";
rc% = MIDI_CAPTURE%("launchkey", "blink.jsonl");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$();
```

---

### `examples/09_make_a_sample_pack.basil`

```basic
REM Record three quick samples in a row (kick/snare/hat style), then play the pack;
PRINTLN "Sample Pack: kick.wav / snare.wav / hat.wav (each ~2s).";
rc% = AUDIO_RECORD%("usb", "kick.wav", 2);  IF rc% <> 0 THEN PRINTLN DAW_ERR$(); RETURN;
rc% = AUDIO_RECORD%("usb", "snare.wav", 2); IF rc% <> 0 THEN PRINTLN DAW_ERR$(); RETURN;
rc% = AUDIO_RECORD%("usb", "hat.wav", 1);   IF rc% <> 0 THEN PRINTLN DAW_ERR$(); RETURN;

PRINTLN "Audition pack:";
rc% = AUDIO_PLAY%("usb", "kick.wav");  IF rc% <> 0 THEN PRINTLN DAW_ERR$();
rc% = AUDIO_PLAY%("usb", "snare.wav"); IF rc% <> 0 THEN PRINTLN DAW_ERR$();
rc% = AUDIO_PLAY%("usb", "hat.wav");   IF rc% <> 0 THEN PRINTLN DAW_ERR$();
PRINTLN "Pack done.";
```

---

### `examples/10_quick_check.basil`

```basic
REM Smoke test: short record + immediate playback to confirm device routing;
PRINTLN "Quick check: 2s record → play.";
rc% = AUDIO_RECORD%("usb", "qc.wav", 2);
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;

rc% = AUDIO_PLAY%("usb", "qc.wav");
IF rc% <> 0 THEN PRINTLN "Error: "; PRINTLN DAW_ERR$(); RETURN;
PRINTLN "OK.";
```

---

## How to run (examples)

```bash
# assuming Basil CLI binary is basilc and obj-daw is enabled
cargo run -p basilc --features obj-daw -- run examples/01_record_then_play.basil
cargo run -p basilc --features obj-daw -- run examples/03_midi_capture_to_jsonl.basil
cargo run -p basilc --features obj-daw -- run examples/04_live_synth.basil

# in another terminal, to stop a blocking helper:
cargo run -p basilc --features obj-daw -- run examples/05_stop_helpers_now.basil
```

# 🌱 PART 2: Low-level obj-audio/obj-midi helpers



