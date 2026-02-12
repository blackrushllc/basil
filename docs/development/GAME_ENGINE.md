Here’s a pragmatic design plan for **obj-game** (Basil) and **G** (basic.js) that feels “GameMaker-ish” in workflow, but stays small enough to actually ship—and still powerful enough to build real games.

---

## North Star

**Teach + ship.**
Students can learn the real fundamentals (game loop, sprites, animation, collisions, scenes, input, audio, camera), and Basil users can build **runnable cross-platform 2D games** without you turning Basil into a giant engine.

Key principle: **same mental model** in both runtimes:

* Basil: `obj-game` (Feature Object)
* YoBASIC/basic.js: `G` (JS extension)
* Similar objects, similar method names, similar event/callback flow

---

## Scope (V1 “Micro Engine”)

### Must-have (V1)

1. **Window + Game Loop**

    * Fixed timestep update (optional) + variable render
    * `RUN()` owns the loop; the user writes callbacks (no manual polling)
2. **2D Rendering**

    * Sprites (texture regions), basic shapes, text
    * Layering / depth (`z`, or `layer`)
    * Camera (position, zoom)
3. **Assets**

    * Load textures + sprite sheets
    * Load bitmap font or simple system font (keep it simple V1)
4. **Input**

    * Keyboard + mouse
    * (Gamepad can be V2)
5. **Entities**

    * Lightweight ECS-ish (or just entity handles + components)
    * Create/destroy, tags, groups
6. **Animation**

    * Frame animation from sprite sheet
    * Per-entity animation player
7. **Collision**

    * AABB and circle colliders
    * Overlap tests + simple “move and slide” (optional)
8. **Audio**

    * Play SFX, loop music, volume

### Explicitly NOT in V1 (but planned)

* Box2D-grade physics, joints, ragdolls
* Full editor tooling
* Particles/shaders/post-processing
* Networking multiplayer
* 3D

---

## User-facing Programming Model (GameMaker-ish but Basil-native)

### The “Game” owns the loop; user provides callbacks

You get the best of both worlds:

* No app-level while-loop boilerplate for students
* You still teach the update/draw model cleanly

**Core callbacks (same concept in Basil + JS):**

* `OnInit`
* `OnUpdate(dt#)`
* `OnDraw`
* Optional: `OnKeyDown(key$)`, `OnMouseDown(btn%, x#, y#)`, etc.

### Entities + Components model (small, not a full engine)

GameMaker feels like “objects with behaviors.” We can emulate that with:

* Entities are IDs/handles
* Components are attached: `Transform`, `Sprite`, `Animator`, `Collider`, `Body`, `Text`

In Basil this maps well to your object ecosystem. In JS, it’s natural.

---

## API Shape (proposed)

### Basil (obj-game)

Expose a **root object** (or module) like:

* `GAME@` or `G@` (I’d go with `GAME@` to avoid confusion with JS “G”)
* Entity handles as `ENT@` objects

Example usage feel:

```basic
IMPORT "obj-game"

SUB OnInit()
  GAME@.Window(960, 540, "My Basil Game")
  GAME@.Assets.LoadTexture("player", "assets/player.png")
  
  player@ = GAME@.Entity.Create()
  player@.Transform.Set(100, 100)
  player@.Sprite.Set("player")
  player@.Collider.AABB(32, 32)
END SUB

SUB OnUpdate(dt#)
  vx# = 0 : vy# = 0
  IF GAME@.Input.KeyDown("Left")  THEN vx# = -200
  IF GAME@.Input.KeyDown("Right") THEN vx# =  200
  IF GAME@.Input.KeyDown("Up")    THEN vy# = -200
  IF GAME@.Input.KeyDown("Down")  THEN vy# =  200
  
  player@.Transform.Move(vx# * dt#, vy# * dt#)
END SUB

SUB OnDraw()
  GAME@.Draw.Clear()
  GAME@.Draw.Entity(player@)
END SUB

GAME@.Run(OnInit, OnUpdate, OnDraw)
```

### basic.js (G library)

Mirror the same shape:

```basic
' in YoBASIC/basic.js
G.Window 960, 540, "My YoBASIC Game"

SUB OnInit()
  G.Assets.LoadTexture "player", "/assets/player.png"
  player = G.Entity.Create()
  G.Transform.Set player, 100, 100
  G.Sprite.Set player, "player"
END SUB

SUB OnUpdate(dt)
  ...
END SUB

SUB OnDraw()
  G.Draw.Clear
  G.Draw.Entity player
END SUB

G.Run OnInit, OnUpdate, OnDraw
```

**Important:** keep naming close even if call syntax differs.

---

## Architecture Plan

### 1) Shared “concept spec”

Before coding, create a small spec doc that both implementations follow:

* Entities: ID semantics, lifetime rules
* Coordinate system: pixels, origin top-left, +x right, +y down
* Units: velocities in px/sec; `dt` in seconds
* Render order: layer then z then creation index
* Asset keys: string IDs like `"player"`

This spec is what keeps Basil and basic.js aligned.

---

## Basil obj-game internals (Rust)

### Core crates (recommended)

Keep it boring and reliable:

* **winit**: window + event loop
* **wgpu**: 2D rendering backend (cross-platform)
* **image**: texture decode
* **glyphon** or similar for text (or defer text to V1.1 if it slows you down)
* **kira** or **rodio**: audio
* ECS: **hecs** (tiny) or your own lightweight entity store

You’re not building Bevy; you’re building a **micro-engine** that’s stable, small, and API-controlled.

### Engine layers

1. **Runtime Adapter**

    * Bridges Basil VM ↔ game loop
    * Calls Basil callbacks safely
    * Converts types (strings, floats, objects)
2. **Game Core**

    * `World` (entities + components)
    * `Assets` (textures, sounds, atlases)
    * `Input` state (pressed/down/released)
    * `Time` and fixed timestep option
3. **Renderer**

    * Batched sprite renderer (texture atlases later)
    * Draw queue (collected each frame)
4. **Audio**

    * SFX + music channels
5. **Collision**

    * Broad phase: simple grid or sweep lists (V1 can be O(n²) with guardrails + teach “keep it small”)
    * Narrow phase: AABB/circle overlap

### Basil integration details (important)

* `GAME@.Run(...)` likely blocks and owns the OS event loop.
* Basil exceptions inside callbacks must be handled:

    * If callback throws: stop loop, return error code, optionally show last error.
* Keep the engine single-threaded initially to avoid VM re-entrancy.
* If Basil already uses a global Tokio runtime: **do not** depend on Tokio in the render loop. If needed, run async tasks via your existing pattern but keep frame loop deterministic.

---

## basic.js “G” internals (JS)

### Rendering

* HTML5 **Canvas 2D** for V1 (fast enough, simplest for teaching)
* Later V2: optional WebGL backend

### Loop

* `requestAnimationFrame`
* Input collected via DOM listeners (keyboard/mouse)
* `G.Run()` triggers RAF loop and calls BASIC subs

### Entity store

* JS objects/maps:

    * `entities = new Map()`
    * components per entity `{transform, sprite, anim, collider, ...}`

### Asset loading

* `Image()` for textures
* `Audio()` / Web Audio for sounds (keep it simple first)

---

## The “GameMaker feel” without the bloat

GameMaker’s magic is:

* A standard game loop you don’t think about
* Objects have behaviors
* Sprites + collisions “just work”
* Scenes/rooms

We can capture that with just a few concepts:

### Scenes (Rooms) — include in V1

* `SCENE@` or `GAME@.Scene.Load("level1")`
* Scene is basically:

    * list of entities to spawn
    * background color, camera start, maybe tilemap reference

V1 implementation can be dead simple:

* Scene is a JSON file or Basil script-defined spawn function
* For teaching, Basil-defined spawn is great: it’s all code.

---

## Deliverables to implement “starting today”

### Step 0 — Create “obj-game spec” doc (1–2 pages)

* API list
* callback semantics
* coordinate/time units
* minimal component list

This is your guardrail so Junie doesn’t drift.

### Step 1 — Basil V1 skeleton

* `crates/obj-game/`

    * `mod.rs` feature object registration
    * `engine/` (world, assets, input, render, audio)
    * `api/` (methods exposed to Basil)
* Smoke test example under `/examples/game_min/`

    * opens window
    * draws sprite
    * moves with arrow keys

### Step 2 — basic.js G skeleton

* `/basicjs/lib/g/` (or your preferred libs folder)

    * `g.js` main
    * `g_assets.js`, `g_input.js`, `g_world.js`, `g_draw.js`
* YoBASIC demo page or desktop page integration:

    * “Run Game” uses the same IDE run button
    * Opens canvas in a window inside your windowing UI

### Step 3 — Shared examples (match behavior)

Write the same 5 small sample games in both:

1. Move a sprite with keyboard
2. Pong
3. Breakout
4. Top-down “collect coins”
5. Simple platformer-ish movement (no full physics, just gravity + AABB floor)

---

## V1 API Checklist (keep this tight)

### Game / Loop

* `Window(w%, h%, title$)`
* `Run(initFn, updateFn, drawFn)`
* `Quit()`
* `Time.FPS%()` / `Time.Delta#()`

### Assets

* `Assets.LoadTexture(key$, path$)`
* `Assets.LoadSound(key$, path$)`
* `Assets.LoadAtlas(key$, imagePath$, jsonPath$)` (optional V1.1)

### Entities

* `Entity.Create() -> ent@`
* `Entity.Destroy(ent@)`
* `Entity.Tag(ent@,"Player")`
* `Entity.FindByTag("Player") -> ent@ or list`

### Transform

* `Transform.Set(ent@, x#, y#)`
* `Transform.Move(ent@, dx#, dy#)`
* `Transform.PosX#(ent@)` / `PosY#(ent@)`

### Sprite/Draw

* `Sprite.Set(ent@, textureKey$)`
* `Sprite.Region(ent@, x%, y%, w%, h%)` (for sheets)
* `Draw.Clear([r,g,b,a])`
* `Draw.Entity(ent@)`
* `Draw.Sprite(textureKey$, x#, y#)`
* `Draw.Text(text$, x#, y#)` (optional V1.1)

### Input

* `Input.KeyDown("Left")`
* `Input.KeyPressed("Space")`
* `Input.MouseX#()`, `MouseY#()`
* `Input.MouseDown(btn%)`

### Collision

* `Collider.AABB(ent@, w#, h#)`
* `Collider.Circle(ent@, r#)`
* `Collision.Overlaps(a@, b@) -> ok%`
* `Collision.QueryAABB(x#, y#, w#, h#) -> list`

### Audio

* `Audio.PlaySfx(key$)`
* `Audio.PlayMusic(key$, loop%)`
* `Audio.Volume(v#)`

---

## Risks & how to avoid getting stuck

1. **Rendering rabbit hole (wgpu complexity)**

    * Start with a single textured quad pipeline + batching later.
    * V1: “sprites and rectangles” only.
2. **Text rendering**

    * If it slows you down, ship V1 without text and add V1.1 quickly.
3. **Collision performance**

    * V1 can be simple with warning docs (“for 100s of entities, not 10,000”).
    * V2 add spatial hash grid.
4. **Keeping Basil + JS aligned**

    * Maintain the shared spec + mirrored examples.
    * Treat differences as bugs unless explicitly documented.

---

## “Today” implementation plan (day-1 kickoff)

### Basil (obj-game)

* Create crate + feature-gated registration
* Implement:

    * window + loop
    * input state
    * texture loader
    * draw sprite at x,y
* Expose minimal API:

    * `Window`, `Run`, `Assets.LoadTexture`, `Draw.Sprite`, `Input.KeyDown`

### basic.js (G)

* Implement:

    * canvas creation inside your windowing UI
    * RAF loop calling BASIC subs
    * keyboard state
    * image loader + drawImage
* Match the same minimal example program

Once both can run **the same “move a sprite” demo**, everything else becomes incremental.

---

If you want, I can turn the above into two **Junie Ultimate implementation prompts** (RustRover for obj-game, PhpStorm for G) that reference your Basil caveats (type suffixes, object model, exception mapping, examples/docs requirements) and your preferred repo layout.
