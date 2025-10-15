Build the **obj-game** mod for Basil that makes 2D games dead-simple now, and leaves a clean path to 3D/Web later.

# What we’ll build (phased)

**Phase 1 – 2D “starter kit” (MVP)**

* Window + render loop (vsync, fixed/variable timestep).
* Draw: images, spritesheets/animations, rectangles/circles/lines, text.
* Input: keyboard, mouse, basic gamepad.
* Audio: SFX and music (play/loop/stop, volume).
* Time: delta-time, FPS counter, timers.
* Assets: load PNG/JPG/TTF/OGG/WAV; simple asset paths.
* Scene: very light “entities + tags” helpers and AABB collisions.
* Packaging: cross-platform (Win/Linux/macOS) native; no external runtime install.

**Phase 2 – “batteries included”**

* Tilemaps (CSV/Tiled), parallax cameras, particles, sprite batching.
* Physics (Rapier2D togglable), simple rigid bodies & overlaps.
* Input rebinding + gamepad rumble.
* Text layout (bitmap + TTF), SDF text optional.
* Asset packer + hot-reload (opt-in).
* Simple UI widgets (buttons, sliders) for menus/debug.

**Phase 3 – Platform & ecosystem**

* Web export (WASM via winit + wgpu/pixels).
* Controller glyphs, localization, save-files, screenshots.
* Editor hooks: paused step, entity inspector.

# Rust tech choices (pragmatic + light deps)

* **winit** (window + input) + **pixels** (blitter) for a simple, zero-overhead 2D pipeline, or **wgpu** later.
* **image** (PNG/JPG), **ab_glyph** (TTF text), **rodio** (audio), **gilrs** (gamepad), **rapier2d** (optional physics).
* Keeps us pure-Rust (no SDL2 dev libs headaches), works on Win/Linux/macOS and compiles to WASM later.

# Basil-level API (clean, BASIC-flavored)

Names are short, discoverable, and consistent with your style.

### Lifecycle & window

* `GAME.INIT width%, height%, title$`
* `GAME.TARGET_FPS% = 60` (optional; 0 = uncapped)
* `GAME.LOOP label$`  (calls the label each frame; exits when window closed)
* `GAME.CLOSE`

### Time

* `GAME.DELTA!` (seconds as float)
* `GAME.FPS%`

### Draw (stateful immediate mode)

* `DRAW.CLEAR r%,g%,b%`
* `DRAW.RECT x!,y!,w!,h!, r%,g%,b% [ ,fill% ]`
* `DRAW.CIRCLE x!,y!,radius!, r%,g%,b% [ ,fill% ]`
* `DRAW.LINE x1!,y1!,x2!,y2!, r%,g%,b%`
* `DRAW.IMAGE img%, x!,y!`
* `DRAW.SPRITE spr%, x!,y! [ ,frame% ]`
* `DRAW.TEXT font%, x!,y!, text$`
* `DRAW.CAMERA x!,y!` (applies to draws after set)
* `DRAW.PRESENT` (called automatically at end of loop unless disabled)

### Assets

* `ASSET.IMAGE_LOAD%(path$)` → handle
* `ASSET.SPRITESHEET_LOAD%(path$, frame_w%, frame_h%)` → handle
* `ASSET.FONT_LOAD%(path$, size%)` → handle
* `ASSET.SOUND_LOAD%(path$)` / `ASSET.MUSIC_LOAD%(path$)`

### Audio

* `SOUND.PLAY snd% [ ,volume! ,pan! ]`
* `MUSIC.PLAY mus% [ ,loop% ]`
* `MUSIC.STOP`
* `AUDIO.VOLUME master!` (0–1)

### Input

* `KEY.DOWN%(code%)`, `KEY.PRESSED%(code%)`, `KEY.RELEASED%(code%)`
* `MOUSE.X!`, `MOUSE.Y!`, `MOUSE.BUTTON%(1|2|3)`
* `PAD.AXIS!(id%, axis%)`, `PAD.BUTTON%(id%, btn%)`

### Entities & collisions (helpers, not full ECS)

* `ENT.NEW%()` / `ENT.DEL id%`
* `ENT.SET_POS id%, x!, y!` / `ENT.GET_POSX! id%` / `ENT.GET_POSY! id%`
* `ENT.SET_SPRITE id%, spr%`
* `ENT.TAG id%, tag$` / `ENT.HAS_TAG%(id%, tag$)`
* `COLLIDE.AABB%(x!,y!,w!,h!, x2!,y2!,w2!,h2!)`

> Internally, these helpers just manage a simple array of structs; users can roll their own if they need more control later.

# Minimal Basil example (Phase 1)

```basil
' examples/pong.basil
CONST W% = 800, H% = 480
CALL GAME.INIT(W%, H%, "Pong!")

ballX! = W%/2 : ballY! = H%/2 : vx! = 220 : vy! = 160
p1Y! = H%/2-40 : p2Y! = H%/2-40

font% = ASSET.FONT_LOAD%("assets/PressStart2P.ttf", 18)
beep% = ASSET.SOUND_LOAD%("assets/beep.wav")

score1% = 0 : score2% = 0

CALL GAME.LOOP("Tick")
END

LABEL Tick
  dt! = GAME.DELTA!

  ' input
  IF KEY.DOWN%(87) THEN p1Y! -= 260*dt!  ' W
  IF KEY.DOWN%(83) THEN p1Y! += 260*dt!  ' S
  IF KEY.DOWN%(38) THEN p2Y! -= 260*dt!  ' Up
  IF KEY.DOWN%(40) THEN p2Y! += 260*dt!  ' Down

  ' update ball
  ballX! += vx!*dt! : ballY! += vy!*dt!

  ' walls
  IF ballY! < 0 OR ballY! > H!-10 THEN
     vy! = -vy! : SOUND.PLAY beep%
  END IF

  ' paddles
  IF COLLIDE.AABB%(ballX!,ballY!,10,10, 20,p1Y!,12,80) OR _
     COLLIDE.AABB%(ballX!,ballY!,10,10, W%-32,p2Y!,12,80) THEN
     vx! = -vx! : SOUND.PLAY beep%
  END IF

  ' scoring
  IF ballX! < -20 THEN score2% += 1 : ballX! = W%/2 : ballY! = H%/2
  IF ballX! > W%+20 THEN score1% += 1 : ballX! = W%/2 : ballY! = H%/2

  ' draw
  CALL DRAW.CLEAR(18,18,24)
  CALL DRAW.RECT(20,p1Y!,12,80, 240,240,240, 1)
  CALL DRAW.RECT(W%-32,p2Y!,12,80, 240,240,240, 1)
  CALL DRAW.RECT(ballX!,ballY!,10,10, 250,180,90, 1)
  CALL DRAW.TEXT(font%, W%/2-80, 20, "#{score1%} : #{score2%}")
RETURN
```

# Crate wiring (Rust side)

```
/basil/
  /mods/
    obj-game/
      src/lib.rs
      Cargo.toml
```

* `obj-game` exposes a `register(runtime: &mut BasilRuntime)` that binds the functions above to Basil names.
* **Backend** module owns the winit event loop and a thread-safe `GameState` (input state, frame buffers, audio sink).
* We run a **single render thread**; Basil code runs on the main VM thread. `GAME.LOOP label$` pumps the event loop, calls back into VM each frame (re-entrantly) with dt.

Key third-party crates in `Cargo.toml`:

```toml
winit = "0.30"
pixels = "0.14"
image = "0.25"
ab_glyph = "0.2"
rodio = { version = "0.18", features=["vorbis"] }
gilrs = "0.10"
rapier2d = { version="0.26", optional=true }
```

Feature flags:

* `physics` to enable Rapier hookups
* `wasm` to use `wasm-bindgen` + web audio fallback (Phase 3)

# Dev ergonomics

* Ship **examples**: `pong.basil`, `shmup.basil`, `tiles.basil`, `particles.basil`.
* Provide **starter assets** (public-domain font/sounds).
* Add to build docs: `cargo run -q -p basilc --features obj-game -- run examples/pong.basil`

# Nice extras (fast wins)

* **GAME.SHOW_FPS% = 1**: draws a tiny FPS overlay.
* **TIMERS**: `TIME.AFTER ms%, label$` and `TIME.EVERY ms%, label$` (implemented in the loop; stored in a small vector).
* **CAPTURE**: `GAME.SCREENSHOT path$` (PNG dump).

# Junie prompt (drop-in)

Use this as your IDE prompt to scaffold Phase 1:

```
Implement a new Basil module crate named `obj-game` and integrate it with the Basil VM.

Goals (Phase 1):
- Pure-Rust 2D backend using winit + pixels.
- Immediate-mode drawing API, simple asset manager, basic input, simple audio.

API to export to Basil (exact names/signatures):
- GAME.INIT width%, height%, title$
- GAME.TARGET_FPS% (r/w int; 0 = uncapped)
- GAME.LOOP label$
- GAME.CLOSE
- GAME.DELTA! -> float seconds
- GAME.FPS% -> int
- DRAW.CLEAR r%,g%,b%
- DRAW.RECT x!,y!,w!,h!, r%,g%,b% [,fill%]
- DRAW.CIRCLE x!,y!,radius!, r%,g%,b% [,fill%]
- DRAW.LINE x1!,y1!,x2!,y2!, r%,g%,b%
- DRAW.IMAGE img%, x!,y!
- DRAW.SPRITE spr%, x!,y! [,frame%]
- DRAW.TEXT font%, x!,y!, text$
- DRAW.CAMERA x!,y!
- DRAW.PRESENT (no-op for now; present at end of frame)
- ASSET.IMAGE_LOAD%(path$) -> int handle
- ASSET.SPRITESHEET_LOAD%(path$, frame_w%, frame_h%) -> int handle
- ASSET.FONT_LOAD%(path$, size%) -> int handle
- ASSET.SOUND_LOAD%(path$) -> int handle
- ASSET.MUSIC_LOAD%(path$) -> int handle
- SOUND.PLAY snd% [,volume! ,pan!]
- MUSIC.PLAY mus% [,loop%]
- MUSIC.STOP
- AUDIO.VOLUME master!
- KEY.DOWN%(code%), KEY.PRESSED%(code%), KEY.RELEASED%(code%)
- MOUSE.X!, MOUSE.Y!, MOUSE.BUTTON%(btn%)
- ENT.NEW%(), ENT.DEL id%
- ENT.SET_POS id%, x!, y!, ENT.GET_POSX! id%, ENT.GET_POSY! id%
- ENT.SET_SPRITE id%, spr%
- ENT.TAG id%, tag$, ENT.HAS_TAG%(id%, tag$)
- COLLIDE.AABB%(x!,y!,w!,h!, x2!,y2!,w2!,h2!)

Implementation notes:
- Create crate `mods/obj-game` with `register(runtime: &mut BasilRuntime)` binding all functions.
- Build a `GameHost` that owns winit window, pixels surface, input state, asset stores (images/fonts/audio), and a frame timer.
- Input: collect per-frame pressed/released via edge detection.
- Text: use ab_glyph; cache glyphs; draw to pixel buffer.
- Images: decode with image crate; upload to a simple texture atlas or blit directly into the pixel buffer for MVP.
- Audio: rodio sink for SFX; separate sink for music (looping).
- Entities: minimal Vec of structs `{x,y,sprite,tag bitset or small string}`; provide helpers only.
- Export example programs under `mods/obj-game/examples/*.basil` and ensure `cargo run -p basilc --features obj-game -- run mods/obj-game/examples/pong.basil` works.
- Add a README in the crate with API docs and a minimal getting-started guide.

Non-goals (Phase 1):
- Physics, particles, tilemaps, shaders, WASM export.

Deliverables:
- Compiling crate, bound into Basil.
- 3 examples: pong.basil, spritesheet.basil (walk cycle), input_test.basil.
- Basic docs and keys table (common VK codes) in README.
```

# Next steps (quick checklist)

1. Create `mods/obj-game` crate and wire `register(...)`.
2. Hard-code a window + loop; expose `GAME.LOOP`.
3. Implement input state + KEY/MOUSE funcs.
4. Implement `DRAW.CLEAR/RECT/LINE/TEXT`.
5. Add image/spritesheet loading + `DRAW.IMAGE/SPRITE`.
6. Add `rodio` for SFX/MUSIC.
7. Ship `examples/pong.basil` and a small `/assets` folder.

