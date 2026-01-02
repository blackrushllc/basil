JUNIE PROMPT (PhpStorm) — basic.js Extension Library: G (V1 foundation)

You are working inside the YoBASIC / basic.js project. Implement a new extension library named `G` that provides a minimal 2D micro game engine foundation for teaching. This is the JS counterpart to Basil’s obj-game. The goal is a YoBASIC program that opens a canvas window/panel, loads an image, moves it with arrow keys, and draws each frame.

Non-negotiables:
- The BASIC program must NOT require a polling loop. `G.Run(initSub, updateSub, drawSub)` owns the loop via requestAnimationFrame.
- Must integrate with YoBASIC’s IDE/windowing UI (if present). If the “desktop/windows 95-ish” UI exists, render the canvas inside a window; otherwise fall back to a simple full-width canvas container.
- Use “object-dot” notation style consistent with other JS libs (e.g., UI lib).
- Keep API aligned in spirit with Basil obj-game.

V1 Success Criteria:
1) A demo BASIC program runs in YoBASIC and creates a game surface (canvas) and loop.
2) It loads `/assets/player.png` (or similar) and draws it.
3) Arrow keys move sprite smoothly using dt.
4) The browser close/stop (or IDE stop) halts loop cleanly.

Deliverables:
A) New JS library files (follow existing conventions):
- e.g. `ui/yobasic-g.js` or `g/yobasic-g.js`
- Supporting modules optional: `g_assets.js`, `g_input.js`, `g_draw.js`, `g_runtime.js`
  B) Public BASIC-facing API (minimum set, mirror Basil):
- `G.Window w%, h%, title$` (creates canvas surface; title used if windowed UI exists)
- `G.Assets.LoadTexture key$, path$` (async image load; provide a ready check or auto-wait in Run)
- `G.Input.KeyDown(key$) -> ok%`
- `G.Draw.Clear` (optional args)
- `G.Draw.Sprite key$, x#, y#`
- `G.Run initSub, updateSub, drawSub` (init once; update(dt) and draw() each frame)
- Optional: `G.Quit`
  C) Demo program + integration:
- Add a demo BASIC program accessible from the IDE (examples menu or a docs example), named like `examples/g_move_sprite.bas`.
- Add minimal docs page in the tutorial iframe area or a markdown doc in repo to describe usage.

Implementation Details / Guidance:
- Use Canvas 2D for V1.
- Timing: compute dt in seconds from performance.now().
- Input:
  - Register keydown/keyup listeners on the document or the canvas container.
  - Track “down” state; `KeyDown("Left")` etc.
  - Support at least: Left/Right/Up/Down/Space/A..Z.
- Asset loading:
  - Use `new Image()` and set src.
  - Maintain a map of textures by key.
  - In `Run`, if init loads textures, allow the game to start drawing only when assets are ready OR provide `G.Assets.Ready()` and show an auto “Loading…” overlay until ready (simple is fine).
- Rendering:
  - Clear with ctx.clearRect or fillRect.
  - Draw sprite with ctx.drawImage.
- Clean shutdown:
  - Provide `G.Quit()` to cancelAnimationFrame and detach listeners if needed.
  - If YoBASIC has a “Stop” button/hook, wire into it so it calls G.Quit automatically.

Alignment with Basil:
- Keep naming and behavior as close as possible, but follow JS/YoBASIC calling conventions.
- Document any intentional differences.

Step Plan:
1) Add library skeleton and ensure it can be loaded by basic.js runtime.
2) Implement surface creation (canvas) and optional window embedding if UI desktop exists.
3) Implement input manager.
4) Implement assets manager.
5) Implement draw API.
6) Implement Run loop with callbacks into BASIC interpreter.
7) Add demo program and docs.

Acceptance checks:
- Provide the exact steps to run the demo in the web IDE.
- Confirm it works without needing the BASIC program to contain its own loop.

Now implement this V1 foundation fully.
