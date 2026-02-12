JUNIE PROMPT (RustRover) — Basil Feature Object: obj-game (V1 foundation)

You are working inside the Basil Rust workspace. Implement a new Feature Object crate named `obj-game` that provides a minimal 2D micro game engine foundation. This is V1 (vertical slice). The goal is to run a Basil example that opens a window, loads a texture, moves a sprite with arrow keys, and draws it each frame.

Non-negotiables / Basil caveats:
- Basil is a BASIC-like interpreter/compiler with type suffixes ($ string, % integer, none float, @ object).
- Feature Objects are optional, compiled-in Rust libraries exposed as BASIC “objects/mods”.
- Errors must map to Basil exceptions (TRY/CATCH) or a consistent error surface (return codes + TERM_ERR$()-like pattern if you already have one for mods).
- Keep API small and ergonomic; avoid exposing Rust complexity.
- Include at least one runnable example under /examples and minimal docs in /docs (or wherever other feature objects document themselves).
- Prefer rustls if networking appears (it won’t in this task).
- Keep engine single-threaded initially; do not require Tokio for the render loop.

V1 Success Criteria (must pass):
1) `examples/obj_game_move_sprite.basil` (or similar) runs and opens a window.
2) The program loads `assets/player.png` (you can add a placeholder asset in the repo or use an existing small image if present).
3) Arrow keys move the sprite smoothly (velocity * dt).
4) Each frame clears screen and draws the sprite at its position.
5) Closing window exits cleanly; errors are understandable.

Tech constraints / choices:
- Use `winit` for window/event loop.
- Use `wgpu` for rendering (simple textured quad pipeline; no batching needed yet).
- Use `image` crate to decode PNG.
- You may defer text rendering and audio until later; not needed in V1.
- Use a lightweight entity representation for now (even “one sprite object” is OK) but lay groundwork for entities/components later.

Deliverables:
A) New crate `crates/obj-game/` (or the workspace’s standard location for feature objects) with:
- `lib.rs` / `mod.rs` that registers the feature object into Basil’s object/mod system.
- An internal engine module: window+loop, input state, assets, renderer.
  B) Public BASIC-facing API (minimum set):
- `GAME@.Window(w%, h%, title$)`
- `GAME@.Assets.LoadTexture(key$, path$)`
- `GAME@.Input.KeyDown(key$) -> ok%`
- `GAME@.Draw.Clear()` (or Clear(r#,g#,b#,a#) optional)
- `GAME@.Draw.Sprite(key$, x#, y#)`
- `GAME@.Run(initFn, updateFn, drawFn)` where init/update/draw are Basil SUB/FUNCTION references
    - `updateFn(dt#)` receives dt in seconds as float
    - `drawFn()` no args
- (Optional) `GAME@.Quit()`
  C) Example + docs:
- Example Basil program demonstrating: create window, load texture, set x/y, update using arrow keys, draw sprite.
- Short docs describing setup, API, and how the callback loop works.

Implementation Notes / Guidance:
- Callback invocation: Basil allows user-defined functions; you must call into Basil’s runtime safely from the loop.
- If the callback throws (Basil exception), stop the loop and surface error.
- Input mapping: accept `"Left"`, `"Right"`, `"Up"`, `"Down"`, `"Space"`, plus `"A"`..`"Z"` as strings; keep mapping minimal and document it.
- Asset paths: relative to current working directory; document expected layout.

Repo Integration:
- Follow existing patterns used by other feature objects (obj-sqlite, obj-curl, etc.) for:
    - crate naming, feature flags, module registration, and doc placement.
- Add necessary workspace Cargo.toml entries and feature flags.
- Ensure `cargo build` succeeds with obj-game enabled (and doesn’t break without it if optional).

Step Plan (execute in order, committing logically):
1) Create crate + workspace wiring + empty registration.
2) Implement engine skeleton with winit event loop + wgpu init.
3) Implement texture loader (PNG) and sprite drawing pipeline.
4) Implement input state tracking (pressed/down) from winit events.
5) Implement callback runner: Run(init, update, draw) — init once, update+draw each frame, dt seconds.
6) Add Basil example and minimal docs.

Acceptance checks:
- Provide the exact command(s) to run the example in this repo.
- Include notes on platform support (Windows/Linux/macOS) and any gotchas.
- include feature obj-game when compiled with --feature obj-all
Now implement this V1 foundation fully.
