# Basil AOT compiler (bcc) — End‑to‑end compiling, online and offline

# Short version:

---

### Example command (online mode, clean folder)

```

cargo build -q -p bcc --release

```


## Copy `bcc.exe` and `hello.basil` into an empty folder, then run the command below.

```powershell

Copy-Item -Path ".\target\release\bcc.exe", ".\examples\hello.basil" -Destination "\projects\basil" 

```

```
.\bcc.exe aot .\hello.basil --dep-source local --local-runtime E:/Projects/Yore/basil
```


```powershell
.\bcc.exe aot .\hello.basil
```

- Copy `bcc.exe` and `hello.basil` into an empty folder, then run the command above.
- This uses the default online mode (crates.io dependencies). The final executable will be under the emitted project’s `target\release\hello.exe` path that `bcc` prints after building.

This guide shows both:

- How end‑users compile a .basil program to a native executable using bcc
- How you, as the distributor, package Basil (basilc + bcc) so users can build with or without internet access

Defaults: bcc assumes you have internet access and will fetch published runtime crates from crates.io. You can also run fully offline by supplying a vendor bundle.


# Gay version:

.\bcc.exe aot .\hello.basil --dep-source local --local-runtime E:/Projects/Yore/basil --emit-project generated\hello

Set-Location .\generated\hello

(or cd generated\hello)

cargo build --release

# Long version:

---

## Part 1 — End‑user: Compile your Basil program to a native executable

Prerequisites
- Install Rust via rustup (stable channel).
  - Windows (MSVC): install “Visual Studio Build Tools” (C++ workload).
  - macOS: Xcode Command Line Tools.
  - Linux: nothing extra for glibc; for fully static builds, add musl target.
- Verify:
  - rustc -V
  - cargo -V

Basic online build (default)
1) Compile your program from source to a native executable:
   
   bcc aot path\to\app.basil

2) bcc will:
   - Parse your .basil file
   - Auto-detect feature objects (@auto baseline) and validate
   - Emit a tiny Rust Cargo project
   - Run cargo build --release to produce a native .exe (Windows) or binary (macOS/Linux)

3) bcc prints the project path; the executable will be at:
   
   <emitted-project>\target\release\<name>.exe   # Windows
   <emitted-project>/target/release/<name>          # macOS/Linux

Useful options
- Program/binary name:
  
  bcc aot app.basil --name hello

- Target triple (cross-compile):
  
  rustup target add x86_64-unknown-linux-musl
  bcc aot app.basil --target x86_64-unknown-linux-musl

- Optimization and LTO:
  
  bcc aot app.basil --opt 3 --lto thin

- Emit project without building (for inspection or air-gapped prep):
  
  bcc aot app.basil --emit-project generated\hello

- Control runtime feature objects explicitly:
  
  bcc aot app.basil --features @auto        # default, auto + curated baseline
  bcc aot app.basil --features @all         # all stable features
  bcc aot app.basil --features obj-term     # explicit set (comma/space separated)

Fully offline build (vendor bundle)
If your machine has no internet access, use a vendor bundle provided by the distributor (see Part 2):

- With a vendor directory:
  
  bcc aot app.basil --dep-source vendor --vendor-dir C:\path\to\vendor

  This copies the vendor folder into the emitted project and writes .cargo/config.toml to point Cargo at it, then builds with cargo --offline.

- Or emit a project for offline build later:
  
  bcc aot app.basil --emit-project generated\hello --dep-source vendor --vendor-dir C:\path\to\vendor
  cd generated\hello
  cargo build --release --offline --locked

Developer/local build (inside the Basil repo)
If you’re working from the Basil source tree, you can make bcc use the in-repo runtime with path deps:

- From the repo root:
  
  bcc aot examples\hello.basil --dep-source local --local-runtime .

This is convenient for contributors and does not require internet. The build output stays under the emitted project’s target/ folder to avoid workspace interference.

Troubleshooting
- “linker not found” on Windows: install MSVC Build Tools (C++), restart terminal.
- Missing feature object: add #USE FOO to your .basil or pass --features obj-foo.
- Building inside a Rust workspace: bcc already builds by manifest-path in the emitted directory and isolates CARGO_TARGET_DIR to that project.

---

## Part 2 — Distributor: Package Basil so users can build online or offline

You can distribute just two executables and, optionally, a vendor bundle:
- basilc.exe — Basil VM/interpreter (runs .basil and .basilx)
- bcc.exe — AOT compiler (Basil → tiny Rust → Cargo → native)
- Optional: vendor/ — an offline bundle of all crates needed to build AOT output

Two distribution paths
1) Online (simplest): publish runtime crates to crates.io
   - Publish these crates with pinned versions (example =0.1.0):
     - libbasilrt
     - basil-obj-term, basil-obj-audio, basil-obj-midi, basil-obj-daw (and any others you support)
   - Ship basilc and bcc only. End-users with rustup can run bcc aot app.basil and Cargo will fetch runtime crates from crates.io.

2) Offline (no internet): ship a vendor bundle
   - On a connected machine, prepare a vendor snapshot using Cargo’s standard mechanism.
   - Create a new Cargo project that depends on libbasilrt = "=0.1.0" and the object crates you plan to support. Then run:
     
     cargo vendor --versioned-dirs --respect-source-config > vendor-config.txt
     
     This produces a vendor/ directory with all needed crates (including transitive dependencies). Verify libbasilrt and basil-obj-* appear under vendor/.
   - Package the vendor/ folder alongside bcc (e.g., in Basil\vendor). End-users run:
     
     bcc aot app.basil --dep-source vendor --vendor-dir C:\Basil\vendor
   - bcc copies vendor/ into the emitted project, writes .cargo/config.toml that redirects crates.io to the vendored-sources, and invokes cargo build --offline --locked.

Recommended installer layout (Windows example)
- Basil\
  - basilc.exe
  - bcc.exe
  - vendor\  (optional; present only for offline installers)
  - docs\ quickstart.txt

Version pinning and reproducibility
- bcc emits Cargo.toml with:
  - libbasilrt = "=X.Y.Z" and object crates pinned when used
  - [profile.release]: opt-level = 3, lto = "thin", codegen-units = 1, panic = "abort"
- bcc builds with cargo --locked so builds don’t drift once Cargo.lock is created.

What if crates aren’t on crates.io yet?
- For internal testing before publishing, use local mode in the repo:
  
  bcc aot examples\hello.basil --dep-source local --local-runtime .

- For external users without internet, provide the vendor/ folder as shown above.

Support matrix and notes
- Targets depend on the user’s local toolchain. For static Linux, add and use x86_64-unknown-linux-musl.
- Windows: MSVC toolchain recommended; GNU works but may require extra setup.
- macOS: codesigning/notarization are up to your distribution pipeline; bcc emits standard Rust projects.

FAQ
- Q: Where is the final executable?
  A: Under the emitted project at target/(triple)/release/<name>. On Windows without explicit --target, it’s target\release\<name>.exe.
- Q: Can users inspect the generated Rust?
  A: Yes; bcc emits src/main.rs (simple, monomorphic code) and Cargo.toml into a cache or the directory you pass with --emit-project.
- Q: Does bcc support feature auto-detection?
  A: Yes. It scans #USE and symbol prefixes (AUDIO_, MIDI_, DAW_, TERM_). You can also override with --features.

Quick reference: CLI flags (AOT path)
- --name <prog>            Set package/binary name
- --features @auto|@all|obj-foo,obj-bar
- --target <triple>        Cross-compile target
- --opt <0|1|2|3>          Opt level (default 3)
- --lto off|thin|fat       LTO mode (default thin)
- --emit-project <dir>     Emit project only
- --dep-source crates-io|local|vendor
- --local-runtime <dir>    Root with crates/libbasilrt (for local)
- --vendor-dir <dir>       Path to vendor bundle (for vendor/offline)

That’s it. With these modes, a single bcc command compiles Basil programs to native executables in both connected and air‑gapped environments.