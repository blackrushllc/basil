### Linker error diagnosis (LNK1318: Unexpected PDB error; LIMIT (12))
This is a Windows/MSVC linker error related to Program Database (PDB) debug info generation. Common causes:
- Corrupted/locked PDB files in `target\debug` (e.g., held by a running process, AV software, or previous aborted build)
- Resource limits/bugs in `link.exe` with very large PDBs
- Conflicting dependencies causing the linker to ingest multiple Microsoft `windows` crate versions (your link command shows both `windows 0.52.x` and `0.53.x` being linked)
- Path/IO issues (long paths, permission issues, low disk space on the drive hosting `target` or `%TEMP%`)

The error you have includes both `windows_x86_64_msvc 0.52.6` and `0.53.1` libraries on the same link line, which is suboptimal and worth fixing, but the immediate failure is a PDB error. Let’s resolve it methodically.

---

### Step-by-step remediation
Try these in order; stop when the build succeeds.

1) Clean and rebuild (clears bad/locked PDBs and incremental state)
- Close any running basil/basilc processes and your IDE terminal sessions that may have loaded the binary.
- Ensure AV/Defender is not actively scanning your `target` directory if possible.
- Run:

```
cargo clean
cargo run -q -p basilc --features obj-all -- run .\examples\hello.basil
```

2) Reduce/disable debug info to shrink PDBs (quick workaround)
- One-off environment flag in PowerShell:

```
$env:RUSTFLAGS = "-C debuginfo=1"
# or to fully disable: "-C debuginfo=0"
$env:RUSTFLAGS = "-C debuginfo=0"

cargo clean
cargo run -q -p basilc --features obj-all -- run .\examples\hello.basil
```

- Permanent option in `Cargo.toml`:

```
[profile.dev]
debug = 1   # or 0 to disable
```

3) Disable incremental compilation (can avoid PDB contention)

```
$env:CARGO_INCREMENTAL = "0"

cargo clean
cargo run -q -p basilc --features obj-all -- run .\examples\hello.basil
```

4) Ensure toolchain and VS Build Tools are up-to-date
- Update Rust and dependencies:

```
rustup update
cargo update
```

- Update MSVC/Windows SDK via Visual Studio Installer (Build Tools 2022). Reboot if installers ran.

5) Align `windows` crate versions (reduce mixed 0.52/0.53 linkage)
- Inspect duplicates:

```
cargo tree -d | Select-String windows
```

- If you see both `windows 0.52` and `windows 0.53`, try pinning a consistent version in your workspace root `Cargo.toml` to unify the dependency tree. For example:

```
[workspace.dependencies]
windows = "=0.53.0"
windows-sys = "=0.52.0"  # only if you also use windows-sys and need a consistent version
```

- Alternatively, add a `[patch.crates-io]` override to coerce all crates to a single version you’ve tested.

6) Check environment and filesystem
- Verify there’s adequate free space on the drive hosting `E:\Projects\Yore\basil\target` and `%TEMP%`
- Ensure the project path isn’t triggering MAX_PATH issues (your paths look fine)
- Temporarily disable real-time AV scanning for the `target` folder during build

7) Last resort: use GNU toolchain or split debuginfo (experimental)
- Install and try the GNU toolchain to avoid MSVC’s PDBs:

```
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup override set stable-x86_64-pc-windows-gnu
cargo clean
cargo run -q -p basilc --features obj-all -- run .\examples\hello.basil
```

- Or try `split-debuginfo` (has limited effect on MSVC, more useful on other platforms):

```
$env:RUSTFLAGS = "-C split-debuginfo=packed -C debuginfo=1"
```

---

### Why the `windows` crate duplication matters
Your link line shows repeated entries for `windows 0.52.6` and one for `0.53.1`. While Rust generally tolerates multiple semver-major-minor versions in a build, it increases link complexity and PDB size, making MSVC failures more likely. Unifying versions:
- Can reduce debug info volume and symbol table size
- Makes future CI builds more deterministic

Use `cargo tree -d` to find which top-level crates are pinning each version, then bump or patch them so they agree (usually to the latest compatible minor).

---

### After you apply the changes
Re-run the command:

```
cargo run -q -p basilc --features obj-all -- run .\examples\hello.basil
```

If it still fails, please:
- Run with verbose linking to capture full linker args:

```
cargo clean
cargo build -p basilc --features obj-all --verbose
```

- Share the tail of the verbose output and any new errors. Also confirm which of the remediation steps you tried (debug info level, incremental off, toolchain/MSVC updates, dependency alignment).

---

### Summary
- Fix warnings by removing unnecessary `mut` and either using or annotating the unused function in `smtp.rs`.
- The link failure is a known MSVC PDB issue. Start with `cargo clean`, lower `debuginfo`, and disable incremental; update toolchain and unify `windows` crate versions if needed. These steps typically resolve `LNK1318 LIMIT (12)` on large Rust projects on Windows.