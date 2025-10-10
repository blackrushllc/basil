# MASTER PROMPT FOR JUNIE — Build Basil MIDI/Audio/DAW Feature Objects (Rust)

**Context (do not skip):**
We have a BASIC-style language named **Basil** with a Rust interpreter/VM. Basil uses:

* `%` for integers/numbers, `$` for strings, and `@` for object handles/refs.
* 1-D and 2-D arrays (string and numeric).
  We want a set of **Rust Feature Objects** (feature-flagged crates/modules) exposing **audio + MIDI + DAW** functions to Basil, usable from both Basil **CLI** and **CGI** modes. The goal is to replicate/replace our CLI DAW tasks (record, play, monitor, MIDI capture, live synth) **from within Basil**.

**Important requirements:**

* Use current crate versions and adapt to any breaking changes (APIs change often).
* Real-time (RT) audio callbacks must be allocation-free; do heavy work on non-RT threads.
* Cross-platform (Windows/macOS/Linux) using the system backends (CoreAudio, WASAPI, ALSA/Pulse/JACK).
* Threading & lifetimes must be clean; no panics for user errors.
* Provide full examples, tests, and docs.

---

## 0) Scope & deliverables

Create three opt-in feature objects (Cargo features), with clean registration hooks:

* `obj-audio` — audio devices, input/output streams, ring buffers, WAV read/write.
* `obj-midi`  — MIDI in (and out, if easy), event queueing.
* `obj-daw`   — high-level “one-liner” helpers: record/play/monitor/capture/synth; simple stop signal; error string.

### Deliverables checklist (must provide all):

1. **Rust code** for each feature object, guarded by Cargo features, compiled and tested.
2. **Symbol registration**: on VM init, register all Basil symbols for enabled features.
3. **Basil examples** in `examples/` demonstrating every API (see section 4).
4. **Unit tests** (Rust) for core pieces; **smoke tests** that exercise init/open/start/stop.
5. **Docs** in `docs/` (one overview doc + per-feature docs). Include API reference & examples.
6. **Build scripts/readme** updates: how to enable features & run the examples.
7. **Error handling**: per-thread last error string retrievable via `DAW_ERR$()` with consistent non-zero return codes on failure.
8. **Graceful stop**: global atomic stop flag toggled by `DAW_STOP()` for blocking helpers.

---

## 1) Project layout & features

Add/edit as needed (match our repo’s conventions):

```
/crates/basil-vm/                 # (existing) VM core
/crates/obj-audio/                # new feature object
/crates/obj-midi/                 # new feature object
/crates/obj-daw/                  # new feature object (depends on the others)
docs/
  audio.md
  midi.md
  daw.md
examples/
  audio_record.basil
  audio_play.basil
  audio_monitor.basil
  midi_capture.basil
  synth_live.basil
```

In workspace `Cargo.toml`, create features:

```toml
[features]
obj-audio = []
obj-midi  = []
obj-daw   = ["obj-audio", "obj-midi"]
```

---

## 2) Crates & versions (use latest; adapt to changes)

Prefer these, but you **must** verify current APIs and upgrade as needed:

* Audio I/O: `cpal`
* MIDI I/O: `midir`
* WAV write: `hound`
* Audio decode (WAV/PCM): `symphonia` (features: wav, pcm)
* Lock-free ring buffer: `ringbuf` (or `rtrb` if preferred)
* Signals/stop: `atomic`/`std::sync::atomic`
* Errors: `thiserror` or `anyhow` internally, but surface Basil-friendly codes + `DAW_ERR$()`

If any crate is deprecated, choose a maintained alternative and note it in docs.

---

## 3) Public API (what Basil users see)

### 3A) High-level, blocking helpers (from `obj-daw`)

Return `%` (0 = OK, non-0 = error). On failure, `DAW_ERR$()` returns a message.

* `AUDIO_RECORD%(inputSubstr$, outPath$, seconds%)`
* `AUDIO_PLAY%(outputSubstr$, filePath$)`
* `AUDIO_MONITOR%(inputSubstr$, outputSubstr$)`
* `MIDI_CAPTURE%(portSubstr$, outJsonlPath$)`
* `SYNTH_LIVE%(midiPortSubstr$, outputSubstr$, poly%)`
* `DAW_STOP()`  'Sets a global atomic stop flag that all blocking helpers check
* `DAW_ERR$()`  'Last error string (thread-local or global per helper; document behavior)

**Device/port selection:** substring match on the human-readable name (case-insensitive), first match wins.

### 3B) Low-level, composable API

**Device discovery**

* `AUDIO_OUTPUTS$[]` → 1-D array of output device names
* `AUDIO_INPUTS$[]`  → 1-D array of input device names
* `AUDIO_DEFAULT_RATE%()` → sample rate (e.g., 48000)
* `AUDIO_DEFAULT_CHANS%()` → channels (e.g., 2)

**Streams**

* `AUDIO_OPEN_IN@(deviceSubstr$)`  → input stream handle
* `AUDIO_OPEN_OUT@(deviceSubstr$)` → output stream handle
* `AUDIO_START%(handle@)`
* `AUDIO_STOP%(handle@)`
* `AUDIO_CLOSE%(handle@)`

**Ring buffers** (bridge RT ↔ Basil thread)

* `AUDIO_RING_CREATE@(frames%)` → ring handle (float frames, interleaved if stereo)
* `AUDIO_RING_PUSH%(ring@, frames![])`    'push interleaved f32 frames
* `AUDIO_RING_POP%(ring@, OUT frames![])` 'pop up to len(frames) samples; returns count

> If Basil lacks a distinct float array `![]`, use numeric arrays `%[]` with [-1.0..1.0] scaled by 32767. Provide conversion helpers. Otherwise prefer `f32`.

**WAV I/O**

* `WAV_WRITER_OPEN@(path$, rate%, chans%)`
* `WAV_WRITER_WRITE%(writer@, frames![])`   'interleaved
* `WAV_WRITER_CLOSE%(writer@)`
* `WAV_READ_ALL![](path$)` → returns interleaved f32 array (or add a streaming reader later)

**MIDI**

* `MIDI_PORTS$[]` → input port names
* `MIDI_OPEN_IN@(portSubstr$)`
* `MIDI_POLL%(in@)` → number of queued events
* `MIDI_GET_EVENT$[](in@)` → 3-byte event as strings (`status`, `data1`, `data2`); document encoding
* `MIDI_CLOSE%(in@)`
* (Optional) `MIDI_OPEN_OUT@`, `MIDI_SEND%(out@, status%, data1%, data2%)`

**Synth (baseline poly sine; easy to extend)**

* `SYNTH_NEW@(rate%, poly%)`
* `SYNTH_NOTE_ON%(synth@, note%, vel%)`
* `SYNTH_NOTE_OFF%(synth@, note%)`
* `SYNTH_RENDER%(synth@, OUT frames![])`  'fills mono buffer
* `SYNTH_DELETE%(synth@)`

**Glue**

* `AUDIO_CONNECT_IN_TO_RING%(in@, ring@)`     'RT callback pushes into ring
* `AUDIO_CONNECT_RING_TO_OUT%(ring@, out@)`   'RT callback pulls from ring

---

## 4) Examples (put these in `/examples` and ensure they run)

1. **Record 10s from USB input**

```basic
rc% = AUDIO_RECORD%("usb", "take1.wav", 10)
IF rc% <> 0 THEN PRINT "Error: "; DAW_ERR$()
```

2. **Play WAV to USB output**

```basic
rc% = AUDIO_PLAY%("usb", "take1.wav")
```

3. **Live monitor input → output (Esc or DAW_STOP)**

```basic
rc% = AUDIO_MONITOR%("scarlett", "scarlett")
```

4. **Capture MIDI to JSON Lines**

```basic
rc% = MIDI_CAPTURE%("keystation", "midilog.jsonl")
```

5. **Live synth (MIDI keyboard → output)**

```basic
rc% = SYNTH_LIVE%("launchkey", "usb", 16)
```

6. **Manual graph with low-level API**

```basic
in@  = AUDIO_OPEN_IN@("usb")
out@ = AUDIO_OPEN_OUT@("usb")
rb@  = AUDIO_RING_CREATE@(48000*2)
ok%  = AUDIO_CONNECT_IN_TO_RING%(in@, rb@)
ok%  = AUDIO_CONNECT_RING_TO_OUT%(rb@, out@)
ok%  = AUDIO_START%(in@) : ok% = AUDIO_START%(out@)
PRINT "Monitoring... Press any key."
k$ = INKEY$
ok% = AUDIO_STOP%(out@) : ok% = AUDIO_STOP%(in@)
ok% = AUDIO_CLOSE%(out@) : ok% = AUDIO_CLOSE%(in@)
```

7. **Record with ring + WAV writer**

```basic
in@ = AUDIO_OPEN_IN@("usb")
wr@ = WAV_WRITER_OPEN@("take1.wav", AUDIO_DEFAULT_RATE%(), AUDIO_DEFAULT_CHANS%())
rb@ = AUDIO_RING_CREATE@(48000*4)
ok% = AUDIO_CONNECT_IN_TO_RING%(in@, rb@)
ok% = AUDIO_START%(in@)

DIM buf![](4800*2)
t% = TIME%()
WHILE TIME%() - t% < 10
  n% = AUDIO_RING_POP%(rb@, buf![])
  IF n% > 0 THEN ok% = WAV_WRITER_WRITE%(wr@, buf![])
WEND

ok% = AUDIO_STOP%(in@)
ok% = WAV_WRITER_CLOSE%(wr@)
```

---

## 5) Implementation notes (how to build it)

* **Registration:** For each feature crate, expose `pub fn register(vm: &mut Vm)` that binds Basil names to Rust call shims. At top-level init, conditionally call `register()` per enabled feature.
* **RT audio callbacks:**

  * No heap allocs or locks in the callback.
  * Use SPSC ring buffers to move audio between RT and Basil/control threads.
  * If `cpal` provides non-`f32` samples, convert at boundary (one pass).
* **Stop mechanism:** A global `AtomicBool` set by `DAW_STOP()`; long-running helpers check it every block and return cleanly.
* **Error reporting:** Store last error string (thread-local or per subsystem). On any `%` error return, also set the error string retrievable via `DAW_ERR$()`.
* **Thread cleanup:** Ensure streams are stopped/closed on interpreter shutdown; drop connections cleanly.
* **MIDI queueing:** Non-blocking push from midir callback into a lock-free queue; Basil pulls via `MIDI_POLL%`/`MIDI_GET_EVENT$[]`.
* **Sample format:** Prefer interleaved `f32`. Document channel behavior; output glue should fan out mono synth to all output channels.
* **Resampling:** Not required for MVP; assume device default rate. Leave TODO for `rubato` integration.

---

## 6) Tests

* **Unit tests:**

  * Ring buffer push/pop counts; synth note on/off + render length; WAV writer frame counts.
* **Smoke tests (behind `--ignored` or feature-gated):**

  * Device enumeration returns ≥ 1 device on CI only where available; otherwise skip gracefully.
  * Open/close streams without panic.
  * MIDI open/close path without device present should fail with a clear error and non-zero code.
* **Doc tests:** Short snippets compiling for non-RT parts.

---

## 7) Docs

Write:

* `docs/audio.md`: devices, streams, rings, WAV I/O (API table + examples).
* `docs/midi.md`: ports, polling, event format, caveats.
* `docs/daw.md`: one-liner helpers, stop/error semantics, latency notes, platform tips.
  Include a **“Troubleshooting”** section (exclusive mode on Windows, sample-rate mismatches, Linux JACK vs ALSA).

---

## 8) Build & run instructions

In **README** and each doc, include:

```bash
# List devices / ports via Basil examples
cargo run -p basilc --features obj-daw -- run examples/audio_record.basil
cargo run -p basilc --features obj-daw -- run examples/audio_play.basil
cargo run -p basilc --features obj-daw -- run examples/audio_monitor.basil
cargo run -p basilc --features obj-daw -- run examples/midi_capture.basil
cargo run -p basilc --features obj-daw -- run examples/synth_live.basil
```

---

## 9) Acceptance criteria

* Building the workspace with `--features obj-daw` succeeds on current stable Rust.
* Each example runs without panics on a typical machine with a class-compliant USB interface & a USB MIDI keyboard:

  * records 10s to `take1.wav`,
  * plays back a WAV file,
  * live monitors input→output,
  * logs MIDI to JSONL,
  * plays a polyphonic live synth via MIDI input to the chosen audio output.
* `DAW_STOP()` cleanly stops any blocking helper within ≤ 250ms block latency.
* Errors always set `DAW_ERR$()` with a meaningful string.

---

### Implementation freedom

If any crate’s API differs, **adapt** while preserving the Basil API. Prefer the simplest stable approach that meets RT constraints. Document any deviations.

**Please proceed to implement everything above.** When you finish, output:

* the patch (files created/edited),
* build results,
* a brief test run transcript,
* and any “gotchas” you discovered (platform specifics or crate quirks).

---

That’s the full spec.
