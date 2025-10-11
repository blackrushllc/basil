### Short answer
Make `bcc` emit projects that can build without this repo by default. Do that by switching the generated `Cargo.toml` to depend on published crates (crates.io) instead of path dependencies, and add an optional “vendor/offline” mode that copies a pre‑bundled set of runtime crates next to the emitted project and writes a `.cargo/config.toml` to use them. Then you can build successfully both:

- in a virgin directory (no Basil repo present), and
- inside the Basil repo terminal (without workspace conflicts), as long as `bcc` uses the local, self‑contained project it just emitted.

Below are the precise changes to make and what you need to publish or ship.

---

### What has to change in `bcc`/emitter to build standalone

1) Switch emitted runtime deps to crates.io (default)
- Today, `backend-rs::render_cargo_toml` writes:
  ```toml
  libbasilrt = { path = "../../../crates/libbasilrt", features = [ … ] }
  ```
- Change it to pinned crates.io dependencies and include selected feature‑object crates, also pinned:
  ```toml
  [dependencies]
  libbasilrt = { version = "=0.1.0", features = [ "term", "audio", "midi", "daw" ] }
  # Only include the ones actually needed by the program:
  # basil-obj-term  = "=0.1.0"
  # basil-obj-audio = "=0.1.0"
  # basil-obj-midi  = "=0.1.0"
  # basil-obj-daw   = "=0.1.0"
  ```
- Keep your existing early feature detection so you only add the object crates that are required.

2) Make builds self‑contained when run under the Basil repo
- When you run Cargo inside `E:\Projects\Yore\basil`, Cargo will detect the top‑level workspace. Even though the emitted crate is not listed as a member, it can still interact poorly with workspace context. To avoid any cross‑talk:
  - Always `current_dir()` the `cargo build` call to the emitted project directory (you already do this), and
  - Add `--manifest-path <emitted>/Cargo.toml` for extra safety if you ever run from a different CWD.
  - Optional but nice: set `CARGO_TARGET_DIR` inside `bcc` to `emitted/target` so it never writes into the workspace’s shared `target/`.

  Example in code (inside `bcc` before spawning Cargo):
  ```rust
  cmd.env("CARGO_TARGET_DIR", emitted.root.join("target"));
  cmd.arg("--locked"); // reproducible, uses Cargo.lock if present
  ```

3) Add an offline/vendor mode (no internet required)
- Provide an optional flag to `bcc`, e.g. `--vendor` (or `--offline`), that does two things:
  - Copies a pre‑packaged set of runtime crates (`libbasilrt` and any `basil-obj-*` crates you ship) into `emitted/vendor/`.
  - Writes `emitted/.cargo/config.toml` instructing Cargo to use those sources instead of crates.io:
    ```toml
    [source.crates-io]
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "vendor"
    ```
  - In this mode, the emitted `Cargo.toml` can still say `libbasilrt = "=0.1.0"`, because the source replacement makes Cargo resolve it from `vendor/`.

- Alternatively (simpler but heavier), in `--vendor` mode rewrite `Cargo.toml` to use local `path = "vendor/libbasilrt"` deps; this avoids writing `.cargo/config.toml`, but requires editing dependencies per feature crate. The source replacement approach scales better.

4) Make the mode explicit in the CLI
- New flags (names are suggestions):
  - `--dep-source crates-io` (default) or `--dep-source local:<dir>` or `--dep-source vendor`.
  - Shorthand helpers:
    - `--use-local-runtime <dir>` → emit `path = "…"` for `libbasilrt` and friends (developer workflow).
    - `--vendor <bundle.zip|dir>` → extract/copy vendor set into `emitted/vendor` and write `.cargo/config.toml` as above.
  - Keep `--emit-project` behavior the same; these flags only control what gets written.

5) Pin versions and profiles in the template
- You already emit `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`. Keep that.
- Pin exact versions for all runtime crates to ensure reproducible builds: `=0.1.0`.
- Add `cargo build --locked` so builds don’t drift once a `Cargo.lock` exists in the emitted project.

---

### What you need to publish to crates.io (for the default path)

To make “virgin directory, online build” work with only the `bcc` executable installed, publish these crates (with matching versions used by `bcc`):

- `libbasilrt` (the runtime API your emitted code calls)
- Each feature object crate you intend to support via AOT:
  - `basil-obj-term`
  - `basil-obj-audio`
  - `basil-obj-midi`
  - `basil-obj-daw`
  - …and any others you plan to expose.

Notes:
- Keep the APIs monomorphic and small to keep compile times quick.
- If some `basil-obj-*` are not yet ready to publish, either don’t auto‑select them in `@auto`, or only allow them when `--dep-source local:…` or `--vendor` is supplied.

Once these are published, an end user can do:
```powershell
bcc aot .\hello.basil  # bcc emits a tiny project using crates.io
# bcc then runs:
#   cargo build --release --locked
# That works in any folder on any machine with rustup + internet.
```

---

### What to distribute alongside the `bcc` executable if you want offline builds

You have two good options; you can support both.

1) Vendor bundle (recommended for offline installers)
- Ship `bcc.exe` with a `vendor/` directory that contains a `cargo vendor` snapshot of:
  - `libbasilrt` at the exact pinned version
  - all `basil-obj-*` crates you support
  - all of their transitive dependencies
- At runtime, `bcc --vendor` copies that `vendor/` into the emitted project and writes the `.cargo/config.toml` shown above. Then it runs `cargo build --offline` (optional but ideal).

  Pros: no path rewrites, future‑proof if you expand object crates; standard Cargo offline pattern.

2) Local runtime source tree
- Ship `bcc.exe` with a `runtime-src/` folder containing the exact sources of `libbasilrt` and `basil-obj-*` (only your crates, not third‑party deps). In `--use-local-runtime <path>` mode, `bcc` writes `path = "…"` entries for those crates. Cargo still needs internet to fetch third‑party deps unless you also vendor them.

  Pros: simpler to produce; good for developer builds. Cons: not fully offline unless paired with `cargo vendor`.

Either way, also distribute:
- A small `README-AOT.txt` with prerequisites (rustup and MSVC build tools on Windows), quickstart, and troubleshooting.
- Optional: a tiny bootstrap that checks for rustup and offers to install it.

---

### Example: emitted `Cargo.toml` (crates.io mode)

```toml
[package]
name = "basil_prog_hello"
version = "0.1.0"
edition = "2021"

[dependencies]
libbasilrt = { version = "=0.1.0", features = ["term"] }
# Only when used:
# basil-obj-term  = "=0.1.0"
# basil-obj-audio = "=0.1.0"
# basil-obj-midi  = "=0.1.0"
# basil-obj-daw   = "=0.1.0"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
```

And for vendor mode, the only addition is the `.cargo/config.toml` file:

```toml
# emitted/.cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

---

### Ergonomics and UX tweaks in `bcc`

- Print exactly where the project was emitted, and the command you’re about to run, e.g.:
  ```
  Emitted project: C:\Users\You\AppData\Local\Temp\.basil\targets\2ab394e91f2a1f0f
  Running: cargo build --release --locked --manifest-path C:\…\Cargo.toml
  ```
- If inside a Rust workspace (like the Basil repo), always pass `--manifest-path` to avoid accidental workspace context.
- Add `--print-cargo-cmd` for debugging.
- Respect `--target` by forwarding to `cargo build --target <triple>` (you already do that).

---

### Putting it all together: behaviors you’ll get

- Virgin directory, online:
  - User runs `bcc aot file.basil` → `Cargo.toml` uses crates.io → Cargo fetches `libbasilrt` and needed object crates → build succeeds.

- Virgin directory, offline:
  - User runs `bcc aot file.basil --vendor` (or `--offline`) → `bcc` drops `vendor/` + `.cargo/config.toml` → `cargo build --offline` → build succeeds.

- Inside Basil repo terminal:
  - User runs `bcc aot file.basil` → `bcc` emits project under `.basil/targets/<hash>` (or `--emit-project …`) and runs `cargo build --manifest-path <that>/Cargo.toml` with `CARGO_TARGET_DIR` set to the emitted folder. No workspace interference.

---

### Minimal implementation checklist

- backend-rs
  - [ ] Add a `DepsMode` enum: `CratesIo { version: String } | LocalPath { root: PathBuf } | Vendor`.
  - [ ] Change `render_cargo_toml` to render appropriately for each mode.
  - [ ] Add writer for `.cargo/config.toml` when `DepsMode::Vendor`.

- bcc CLI
  - [ ] Flags: `--dep-source crates-io|local:<path>|vendor`, default `crates-io`.
  - [ ] Flag: `--vendor <bundle.zip|dir>` to point to your prebuilt vendor set (optional if you ship it alongside).
  - [ ] Always run `cargo build --release --locked --manifest-path <emitted>/Cargo.toml` and set `CARGO_TARGET_DIR` to `<emitted>/target`.

- Release engineering
  - [ ] Publish `libbasilrt` and initial `basil-obj-*` crates to crates.io and tag versions.
  - [ ] Build a `cargo vendor` tarball of those plus transitive deps for offline installers.
  - [ ] Package `bcc` with either: (a) nothing extra (crates.io mode), or (b) vendor bundle for offline.

---

### FAQ

- Do we still need the path dependency variant?
  - Yes, keep it for contributors working in the Basil repo (`--dep-source local:<repo-root>`). But don’t make it the default.

- Will this fix the “current package believes it’s in a workspace when it’s not” error?
  - Yes, because the default path will be crates.io dependencies, and `bcc` will build the emitted project on its own, pointing Cargo precisely at that manifest. Even when run inside the repo, Cargo will operate exclusively on the emitted project.

- What about `basilc` distribution?
  - `basilc` is standalone. For `bcc`, users need Rust (rustup + MSVC on Windows). Mention this in your downloader/installer and docs.

---

### TL;DR
- Change the emitter to use pinned crates.io deps by default, plus an optional vendor/offline mode.
- Publish `libbasilrt` and the `basil-obj-*` crates.
- In `bcc`, always build the emitted project directly by manifest path and keep its `target/` local.
- If you also ship a vendor bundle, `bcc --vendor` will build successfully even without internet, in any folder, without this repo.