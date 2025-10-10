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

Heck yeah—here’s a matching set of **low-level** Basil examples that use **`obj-audio`** and **`obj-midi`** directly (rings, streams, WAV I/O, MIDI polling, simple synth), with Kitchen-Sink style: semicolons; `BEGIN…END` blocks; `%/$/@` types; `PRINT/PRINTLN`; and clean exit on keypress.

> Assumptions:
> • Arrays of samples use **numeric `%[]`** interleaved frames.
> • `WAV_READ_ALL%[]` returns interleaved numeric samples.
> • Ring ops accept/return counts in samples (not frames).
> • Glue helpers exist: `AUDIO_CONNECT_IN_TO_RING%`, `AUDIO_CONNECT_RING_TO_OUT%`.
> If your final API names differ, tweak the calls—the structure remains the same.

---

### `examples/low/01_devices_and_defaults_low.basil`

```basic
REM List devices and defaults;
PRINTLN "== Outputs ==";
outs$[] = AUDIO_OUTPUTS$[];
FOR i% = 0 TO LEN(outs$[]) - 1 BEGIN
  PRINT "  "; PRINT i%; PRINT ": "; PRINTLN outs$[](i%);
END

PRINTLN "== Inputs ==";
ins$[] = AUDIO_INPUTS$[];
FOR i% = 0 TO LEN(ins$[]) - 1 BEGIN
  PRINT "  "; PRINT i%; PRINT ": "; PRINTLN ins$[](i%);
END

PRINT "Default rate: "; PRINTLN AUDIO_DEFAULT_RATE%();
PRINT "Default chans: "; PRINTLN AUDIO_DEFAULT_CHANS%();
```

---

### `examples/low/02_monitor_low.basil`

```basic
REM Low-level live monitor with local key exit;
in@  = AUDIO_OPEN_IN@("usb");
out@ = AUDIO_OPEN_OUT@("usb");
rb@  = AUDIO_RING_CREATE@(AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%() * 2); REM ~1s;

ok% = AUDIO_CONNECT_IN_TO_RING%(in@, rb@);
ok% = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
ok% = AUDIO_START%(in@);
ok% = AUDIO_START%(out@);

PRINTLN "Monitoring (usb -> usb). Press any key to stop.";
DO
  k$ = INKEY$();
LOOP UNTIL k$ <> "";

ok% = AUDIO_STOP%(out@); ok% = AUDIO_STOP%(in@);
ok% = AUDIO_CLOSE%(out@); ok% = AUDIO_CLOSE%(in@);
PRINTLN "Stopped.";
```

---

### `examples/low/03_record_low.basil`

```basic
REM Record N seconds using ring + WAV writer (non-blocking main loop);
DUR_S% = 5;
rate%  = AUDIO_DEFAULT_RATE%();
ch%    = AUDIO_DEFAULT_CHANS%();
in@    = AUDIO_OPEN_IN@("usb");
wr@    = WAV_WRITER_OPEN@("take_low.wav", rate%, ch%);
rb@    = AUDIO_RING_CREATE@(rate% * ch% * 4); REM ~2s headroom;

ok% = AUDIO_CONNECT_IN_TO_RING%(in@, rb@);
ok% = AUDIO_START%(in@);

blockSamples% = rate% * ch% / 10; REM ~100ms;
DIM buf%[](blockSamples%);

t0% = TIME%();
WHILE TIME%() - t0% < DUR_S% BEGIN
  n% = AUDIO_RING_POP%(rb@, buf%[]);
  IF n% > 0 THEN ok% = WAV_WRITER_WRITE%(wr@, buf%[]);
END

ok% = AUDIO_STOP%(in@);
ok% = WAV_WRITER_CLOSE%(wr@);
ok% = AUDIO_CLOSE%(in@);
PRINTLN "Saved take_low.wav";
```

---

### `examples/low/04_play_low.basil`

```basic
REM Play a WAV by pushing decoded samples to an output ring;
samples%[] = WAV_READ_ALL%[]("take_low.wav");
out@  = AUDIO_OPEN_OUT@("usb");
rb@   = AUDIO_RING_CREATE@(AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%() * 4);
ok%   = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
ok%   = AUDIO_START%(out@);

PRINTLN "Playing take_low.wav…";
i% = 0;
chunk% = AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%() / 10; REM ~100ms
DIM win%[](chunk%);

WHILE i% < LEN(samples%[]) BEGIN
  copy% = MIN(chunk%, LEN(samples%[]) - i%);
  FOR k% = 0 TO copy% - 1 BEGIN
    win%[](k%) = samples%[](i% + k%);
  END
  pushed% = AUDIO_RING_PUSH%(rb@, win%[]);
  i% = i% + copy%;
END

REM drain: give output time to finish
SLEEP 500;
ok% = AUDIO_STOP%(out@); ok% = AUDIO_CLOSE%(out@);
PRINTLN "Done.";
```

---

### `examples/low/05_midi_poll_print_low.basil`

```basic
REM Poll MIDI input and print events until keypress;
m@ = MIDI_OPEN_IN@("launchkey");

PRINTLN "MIDI events (press any key to quit)…";
DO
  WHILE MIDI_POLL%(m@) > 0 BEGIN
    ev$[] = MIDI_GET_EVENT$[](m@);  REM ["status","data1","data2"]
    PRINT "MIDI: "; PRINT ev$; PRINT ", "; PRINT ev$; PRINT ", "; PRINTLN ev$;
  END
  k$ = INKEY$();
LOOP UNTIL k$ <> "";

MIDI_CLOSE%(m@);
PRINTLN "Bye.";
```

---

### `examples/low/06_synth_live_low.basil`

```basic
REM Live poly synth driven by MIDI, rendered in Basil loop to output ring;
rate% = AUDIO_DEFAULT_RATE%();
poly% = 16;

m@   = MIDI_OPEN_IN@("launchkey");
out@ = AUDIO_OPEN_OUT@("usb");
rb@  = AUDIO_RING_CREATE@(rate% * AUDIO_DEFAULT_CHANS%() * 2);
ok%  = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
ok%  = AUDIO_START%(out@);

s@ = SYNTH_NEW@(rate%, poly%);

blockFrames% = rate% / 50; REM 20ms
ch%          = AUDIO_DEFAULT_CHANS%();
DIM mono%[](blockFrames%);
DIM inter%[](blockFrames% * ch%);

PRINTLN "Synth: play your keys (press any key to quit).";
DO
  WHILE MIDI_POLL%(m@) > 0 BEGIN
    ev$[] = MIDI_GET_EVENT$[](m@);
    status% = VAL(ev$);
    d1% = VAL(ev$); d2% = VAL(ev$);
    IF (status% AND &HF0) == &H90 THEN
      IF d2% == 0 THEN SYNTH_NOTE_OFF%(s@, d1%); ELSE SYNTH_NOTE_ON%(s@, d1%, d2%);
    ELSEIF (status% AND &HF0) == &H80 THEN
      SYNTH_NOTE_OFF%(s@, d1%);
    ENDIF
  END

  REM Render mono block and fan out to all channels;
  SYNTH_RENDER%(s@, mono%[]);
  idx% = 0;
  FOR i% = 0 TO blockFrames% - 1 BEGIN
    FOR c% = 0 TO ch% - 1 BEGIN
      inter%[](idx%) = mono%[](i%);
      idx% = idx% + 1;
    END
  END

  ok% = AUDIO_RING_PUSH%(rb@, inter%[]);
  k$ = INKEY$();
LOOP UNTIL k$ <> "";

SYNTH_DELETE%(s@);
ok% = AUDIO_STOP%(out@);
MIDI_CLOSE%(m@);
AUDIO_CLOSE%(out@);
PRINTLN "Stopped.";
```

---

### `examples/low/07_ring_health_low.basil`

```basic
REM Monitor ring under/overflows while relaying input->output, print stats each second;
in@  = AUDIO_OPEN_IN@("usb");
out@ = AUDIO_OPEN_OUT@("usb");
rb@  = AUDIO_RING_CREATE@(AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%() * 2);

ok% = AUDIO_CONNECT_IN_TO_RING%(in@, rb@);
ok% = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
ok% = AUDIO_START%(in@);
ok% = AUDIO_START%(out@);

PRINTLN "Ring health (press any key to stop)…";
lastT% = TIME%(); pushes% = 0; pops% = 0;

DIM tmp%[](AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%() / 10);

DO
  REM simulate main-thread assist by popping/pushing a bit (optional depending on your glue);
  n% = AUDIO_RING_POP%(rb@, tmp%[]);
  IF n% > 0 THEN pops% = pops% + 1;
  pushed% = AUDIO_RING_PUSH%(rb@, tmp%[]);
  IF pushed% > 0 THEN pushes% = pushes% + 1;

  IF TIME%() - lastT% >= 1 THEN
    PRINT "sec: pushes="; PRINT pushes%; PRINT ", pops="; PRINTLN pops%;
    pushes% = 0; pops% = 0; lastT% = TIME%();
  ENDIF

  k$ = INKEY$();
LOOP UNTIL k$ <> "";

ok% = AUDIO_STOP%(out@); ok% = AUDIO_STOP%(in@);
AUDIO_CLOSE%(out@); AUDIO_CLOSE%(in@);
PRINTLN "Done.";
```

---

### `examples/low/08_device_picker_low.basil`

```basic
REM Pick the first output containing a substring, then play a short beep via synth;
outs$[] = AUDIO_OUTPUTS$[];
match$ = "usb";
pick$ = "";
FOR i% = 0 TO LEN(outs$[]) - 1 BEGIN
  IF INSTR(LOWER$(outs$[](i%)), match$) > 0 THEN pick$ = outs$[](i%);
END
IF pick$ == "" THEN PRINTLN "No match for 'usb' in outputs."; RETURN;

out@ = AUDIO_OPEN_OUT@(pick$);
rb@  = AUDIO_RING_CREATE@(AUDIO_DEFAULT_RATE%() * AUDIO_DEFAULT_CHANS%());
ok%  = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
ok%  = AUDIO_START%(out@);

rate% = AUDIO_DEFAULT_RATE%();
s@ = SYNTH_NEW@(rate%, 8);
SYNTH_NOTE_ON%(s@, 69, 100);   REM A4
DIM mono%[](rate% / 5);        REM 200ms
DIM inter%[](LEN(mono%[]) * AUDIO_DEFAULT_CHANS%());

FOR n% = 0 TO 4 BEGIN
  SYNTH_RENDER%(s@, mono%[]);
  idx% = 0;
  FOR i% = 0 TO LEN(mono%[]) - 1 BEGIN
    FOR c% = 0 TO AUDIO_DEFAULT_CHANS%() - 1 BEGIN
      inter%[](idx%) = mono%[](i%); idx% = idx% + 1;
    END
  END
  ok% = AUDIO_RING_PUSH%(rb@, inter%[]);
END
SYNTH_NOTE_OFF%(s@, 69);

SLEEP 300;
ok% = AUDIO_STOP%(out@);
AUDIO_CLOSE%(out@);
SYNTH_DELETE%(s@);
PRINTLN "Beeped on: "; PRINTLN pick$;
```

---

### `examples/low/09_record_two_takes_ab_low.basil`

```basic
REM Two takes via low-level path, then play both via low-level playback;
rate% = AUDIO_DEFAULT_RATE%(); ch% = AUDIO_DEFAULT_CHANS%();

REM ---- record A
PRINTLN "Take A (2s)…";
inA@ = AUDIO_OPEN_IN@("usb"); rbA@ = AUDIO_RING_CREATE@(rate% * ch% * 4);
ok% = AUDIO_CONNECT_IN_TO_RING%(inA@, rbA@); ok% = AUDIO_START%(inA@);
wrA@ = WAV_WRITER_OPEN@("A_low.wav", rate%, ch%);

DIM buf%[](rate% * ch% / 10);
t0% = TIME%();
WHILE TIME%() - t0% < 2 BEGIN
  n% = AUDIO_RING_POP%(rbA@, buf%[]);
  IF n% > 0 THEN ok% = WAV_WRITER_WRITE%(wrA@, buf%[]);
END
AUDIO_STOP%(inA@); WAV_WRITER_CLOSE%(wrA@); AUDIO_CLOSE%(inA@);

REM ---- record B
PRINTLN "Take B (2s)…";
inB@ = AUDIO_OPEN_IN@("usb"); rbB@ = AUDIO_RING_CREATE@(rate% * ch% * 4);
ok% = AUDIO_CONNECT_IN_TO_RING%(inB@, rbB@); ok% = AUDIO_START%(inB@);
wrB@ = WAV_WRITER_OPEN@("B_low.wav", rate%, ch%);

t1% = TIME%();
WHILE TIME%() - t1% < 2 BEGIN
  n% = AUDIO_RING_POP%(rbB@, buf%[]);
  IF n% > 0 THEN ok% = WAV_WRITER_WRITE%(wrB@, buf%[]);
END
AUDIO_STOP%(inB@); WAV_WRITER_CLOSE%(wrB@); AUDIO_CLOSE%(inB@);

REM ---- play A then B
PRINTLN "Play A then B…";
CALL PlayFileLow$("A_low.wav");
CALL PlayFileLow$("B_low.wav");
PRINTLN "Done.";

SUB PlayFileLow$(p$)
  samples%[] = WAV_READ_ALL%[](p$);
  out@ = AUDIO_OPEN_OUT@("usb");
  rb@  = AUDIO_RING_CREATE@(rate% * ch% * 2);
  ok%  = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@);
  ok%  = AUDIO_START%(out@);
  i% = 0; chunk% = rate% * ch% / 10;
  DIM win%[](chunk%);
  WHILE i% < LEN(samples%[]) BEGIN
    copy% = MIN(chunk%, LEN(samples%[]) - i%);
    FOR k% = 0 TO copy% - 1 BEGIN win%[](k%) = samples%[](i% + k%); END
    ok% = AUDIO_RING_PUSH%(rb@, win%[]);
    i% = i% + copy%;
  END
  SLEEP 300;
  AUDIO_STOP%(out@); AUDIO_CLOSE%(out@);
END SUB
```

---

## How to run (low-level)

```bash
# Enable low-level features; adapt names if your cargo features differ
cargo run -p basilc --features obj-audio,obj-midi -- run examples/low/02_monitor_low.basil
cargo run -p basilc --features obj-audio,obj-midi -- run examples/low/03_record_low.basil
cargo run -p basilc --features obj-audio,obj-midi -- run examples/low/06_synth_live_low.basil
```

Want me to bundle these into your repo (folders + files) and hand you a zip?


