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

> Jeepers I tried everything, rebooting, turning off AV, deleting the files, cargo clear, cargo update, setting the
> values on Cargo.toml. Even set RUSTFLAGS=-C link-arg=/DEBUG:NONE had no effect. I copied the entire proect folder to my
> C: drive and we get the same results. I updated my Visual Studio build tools too. Same results. I switched branches to
> my stable branch from before we added a new module, which includes SMTP and SFTP functionality, and it's fine. The
> difference is we added a new feature called for SMTP and SFTP in a folder called basil-object-net. Hmmm ... What is it
> about having this additional code that could be causing this error?

Short answer: your new SMTP/SFTP module pulled in a *lot* more deps (e.g., `lettre`, `native-tls/schannel`, `libssh2-sys`, `libz-sys`, `windows*`, etc.). That explodes debug symbol output in **dev** builds and tickles an MSVC PDB bug—hence LNK1318. Release works because there’s far less (or different) debug info.

Two big, actionable points for you:

## 1) Your `/DEBUG:NONE` test likely didn’t run

You’re in **PowerShell**, but you used CMD syntax:

```
set RUSTFLAGS=...
```

In PowerShell you must use:

```powershell
$env:RUSTFLAGS = '-C link-arg=/DEBUG:NONE'
cargo clean -p basilc
cargo build -p basilc
```

Or, to *completely* turn off debuginfo (stronger):

```powershell
$env:RUSTFLAGS = '-C debuginfo=0 -C link-arg=/DEBUG:NONE'
cargo clean -p basilc
cargo build -p basilc
```

If that succeeds, you’ve confirmed it’s purely PDB generation.

## 2) Reduce/avoid PDB pressure in dev and tame the graph

Add this to the repo to make dev builds sane on Windows (and still keep symbols if you want them):

```toml
# .cargo/config.toml (create if missing)
[profile.dev]
debug = 1            # smaller PDBs (0 = none, 1 = light, 2 = default)
split-debuginfo = "off"
incremental = false  # fewer weird PDB interactions
```

Then:

```powershell
cargo clean -p basilc
cargo build -p basilc
```

### Also worth doing (one-time cleanups)

* **Unify the `windows`/`windows-sys` versions.** Your link line shows both 0.52.x and 0.53.x getting pulled, which bloats and confuses symbol generation.

  ```powershell
  cargo tree -i windows
  cargo tree -i windows-sys
  # choose a single version, e.g. 0.53.x:
  cargo update -p windows --precise 0.53.0
  cargo update -p windows-sys --precise 0.52.0   # or bump both to compatible releases
  ```

  If something resists, temporarily pin with:

  ```toml
  # Cargo.toml (workspace root)
  [patch.crates-io]
  windows = "=0.53.0"
  windows-sys = "=0.52.0"
  ```

  (Goal: stop multiple major/minor `windows*` crates from coexisting.)

* **Prefer rustls over schannel/native-tls** for SMTP if you can (fewer Windows-specific libs involved). For `lettre`, pick the `rustls` TLS feature rather than `native-tls`:

  ```toml
  lettre = { version = "...", default-features = false, features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }
  ```

  This removes `schannel` from the graph and often shrinks PDBs.

* **libssh2-sys**: use the vendored build (default) and keep it up to date. Nothing special to do, but be aware it adds C objects + symbols in debug.

If, after all that, dev still trips LNK1318 on your box, you have two pragmatic options for daily work:

* Keep `[profile.dev] debug = 0` on Windows (no PDBs in dev) and use `cargo run --release` when you need full speed.
* Or create a custom profile that disables debuginfo only for `basilc`:

  ```toml
  [profile.dev.package.basilc]
  debug = 0
  ```

**Why the new module triggers it:** SMTP/SFTP added TLS + crypto + Windows bindings + C sys-crates. That multiplies type info and object counts in debug. MSVC’s PDB writer occasionally hits internal limits in that scenario, which is exactly what “LIMIT (12)” is hinting at.


If you want, paste the output of:

```powershell
cargo tree -e features -p basilc
cargo tree -i windows
cargo tree -i windows-sys
```

and I’ll show you precisely which features/versions to flip to slim the graph.

> Yeah it's definitely that branch. I cloned a fresh copy of the repo on my C: drive and it does the same thing. I
> updated the Cargo.toml like you suggested and the problem went away. It never seemed to be an issue with Linux

Sweet — that lines up perfectly with a Windows-only PDB/linker quirk. Linux doesn’t emit MSVC PDBs, so it never trips the same limit. Your new SMTP/SFTP module pulled in a lot of Windows-flavored deps (TLS, libssh2, windows*/schannel, etc.), which made the **debug** build on MSVC spit out way more symbols than before; dialing back debug info (or slimming features/versions) fixes it.

Here’s how I’d lock this in so it never bites you again:

## 1) Make a Windows-only dev profile

Keep symbols small only on Windows debug builds; leave other platforms unchanged:

```toml
# .cargo/config.toml (workspace root)
[target.'cfg(windows)'.profile.dev]
debug = 1          # lighter PDBs (0 = none, 1 = light, 2 = default)
incremental = false
```

If you ever need to go nuclear on a stubborn machine, temporarily do:

```powershell
$env:RUSTFLAGS = '-C debuginfo=0 -C link-arg=/DEBUG:NONE'
```

(For PowerShell, not `set`.)

## 2) Prefer rustls over schannel on Windows

This trims Windows linkage and symbol bloat:

```toml
# basil-objects-net/Cargo.toml (or wherever SMTP lives)
lettre = { version = "0.11", default-features = false, features = [
  "smtp-transport",            # what you need
  "tokio1-rustls-tls",         # use rustls, not native-tls/schannel
  "builder"
] }
```

## 3) Avoid mixed `windows` crate versions

Your failing link line showed both `windows 0.52.x` and `0.53.x`. Pin to one:

```toml
# Cargo.toml (workspace root)
[patch.crates-io]
windows = "=0.53.0"
windows-sys = "=0.52.0"  # or align to what your deps expect
```

Then:

```powershell
cargo update -p windows --precise 0.53.0
cargo update -p windows-sys --precise 0.52.0
```

(Fewer, unified Windows bindings → smaller PDBs and fewer surprises.)

## 4) Keep CI happy and document it

* Add a note in `SETUP_WINDOWS.md` (or README) explaining the Windows dev profile and the rustls choice.
* In CI, keep a Windows debug build to catch regressions, but use the trimmed settings above.
* If you split crates later, consider isolating the SMTP/SFTP bits so `basilc` doesn’t always pull them in during *debug* development.

### Why Linux was fine

* Linux uses DWARF debug info and `ld`/`lld` linkers — no MSVC PDB writer involved.
* The extra deps don’t stress the same internal limits there.

If you want, paste your current `Cargo.toml` feature blocks for the net module and I’ll propose the cleanest feature set for Windows vs. non-Windows.
