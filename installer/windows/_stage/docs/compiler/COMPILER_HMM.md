### What’s happening
You’re seeing two different issues depending on where you run/build the generated project:

1) When building inside the Basil repo tree
- Error: `current package believes it's in a workspace when it's not`
- Why: Cargo detected the workspace at `E:\Projects\Yore\basil\Cargo.toml`, but the generated crate under `generated\bigtest.exe` is not listed in that workspace’s `members`. By default, running Cargo from within a workspace directory makes Cargo try to treat sub-crates as workspace members.

2) After copying the generated folder outside the repo
- Error: Cargo can’t find `libbasilrt` path dependency and tries to read `E:\crates\libbasilrt\Cargo.toml`.
- Why: The generated `Cargo.toml` currently points to Basil’s in-repo runtime with a path dependency (for local dev):
  
  ```toml
  libbasilrt = { path = "../../../crates/libbasilrt", features = [ ... ] }
  ```
  
  After you move the folder elsewhere, that relative path becomes invalid. Cargo then fails to resolve the dependency.

---

### Quick fixes (choose one)

#### Option A — Build it in place (inside the Basil repo) without workspace errors
- Use `--manifest-path` so Cargo builds the generated crate as a standalone project:
  
  ```powershell
  # From the repo root or any directory
  cargo run --release --manifest-path E:\Projects\Yore\basil\generated\bigtest.exe\Cargo.toml
  # or just build
  cargo build --release --manifest-path E:\Projects\Yore\basil\generated\bigtest.exe\Cargo.toml
  ```
  
- Tip: To keep targets separate from your workspace’s `target/`, set a per-project target dir:
  
  ```powershell
  Set-Location E:\Projects\Yore\basil\generated\bigtest.exe
  $env:CARGO_TARGET_DIR = "$PWD\target"
  cargo run --release
  ```

- RustRover: attach `generated\bigtest.exe\Cargo.toml` as a separate Cargo project and run it (details below).

#### Option B — Temporarily add the generated folder to the workspace
- Edit `E:\Projects\Yore\basil\Cargo.toml` and add `"generated/bigtest.exe"` to `[workspace].members`.
- Then you can `cargo run -p bigtest.exe` directly.
- Caveat: This is fine for local experiments but clutters the workspace; remove the member entry when done.

#### Option C — Make the generated project fully standalone (outside the repo)
- Edit the generated `Cargo.toml` to use a crates.io dependency for `libbasilrt` instead of a path:
  
  ```toml
  [dependencies]
  libbasilrt = { version = "=0.1.0", features = [ "term" ] }  # adjust features as needed
  ```
  
- Remove the old path-based line. Then run:
  
  ```powershell
  Set-Location E:\Projects\generated\bigtest.exe
  cargo run --release
  ```
  
- Important: This requires `libbasilrt` to be published on crates.io at the pinned version. If it isn’t published yet, use an absolute path to the repo instead:
  
  ```toml
  [dependencies]
  libbasilrt = { path = "E:/Projects/Yore/basil/crates/libbasilrt", features = [ "term" ] }
  ```
  
  Note: Use forward slashes in Cargo paths on Windows.

---

### How to run from RustRover cleanly

- Option A: Attach the generated crate as an additional Cargo project
  1) In the Project tool window, right‑click `generated\bigtest.exe\Cargo.toml` → "Attach Cargo Project".
  2) Open the Cargo tool window → choose the attached project → `run (release)`.
  3) This avoids the workspace-member error because RustRover runs Cargo with the manifest pointed at that `Cargo.toml`.

- Option B: Create a Cargo Run configuration
  1) Run → Edit Configurations… → + → Cargo.
  2) Command: `run`
  3) Additional arguments: `--release`
  4) Working directory: `E:\Projects\Yore\basil\generated\bigtest.exe`
  5) Apply & Run.

- If you moved the folder outside the repo, ensure `Cargo.toml` uses a valid `libbasilrt` dependency (crates.io version or absolute path as shown above), otherwise RustRover will fail the same way Cargo does.

---

### Why `bcc --emit-project` currently points to a path dependency
For fast local iteration, the emitted `Cargo.toml` uses a path to `crates/libbasilrt` in your repo. This means:
- It compiles out of the box when kept inside the repo tree (with `--manifest-path` or after adding to workspace members).
- If you move it elsewhere, you must either:
  - Rewrite the dependency to an absolute path to your repo, or
  - Use the crates.io version once `libbasilrt` (and any `basil-obj-*` crates you need) are published.

Once the runtime crates are published, `bcc` can be switched to emit `libbasilrt = { version = "=x.y.z" }` by default for portable, standalone builds.

---

### Summary
- Inside the repo: build with `--manifest-path` or attach the generated crate in RustRover.
- Outside the repo: change `libbasilrt` from a path dependency to either a crates.io version or an absolute path to your repo.
- RustRover: attach the generated `Cargo.toml` as its own Cargo project or set a Run configuration whose working directory is the generated folder.

If you paste your current `generated\bigtest.exe\Cargo.toml`, I can show you the exact one-line change needed for your setup.