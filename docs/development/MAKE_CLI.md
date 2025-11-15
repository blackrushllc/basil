# 📦 Junie Prompt — Embed `includes/` and add `make` subcommand to **basilc**

**Repository:** Basil (Rust, multi-crate workspace)
**Target binary crate:** `basilc` (the CLI interpreter)
**Goal:**

1. At build time, read everything under `basilc/includes/` and embed it into the **`basilc` executable** as static bytes.
2. Add a CLI subcommand `make <target>` with behavior:

    * `basilc make examples` → write entire `includes/examples/` tree into `./examples/` (create/best-effort overwrite). **Do not run.**
    * `basilc make examples/hello.basil` → write *just that file* into `./examples/hello.basil` then **run it** (as if `basilc run examples/hello.basil`).
    * `basilc make upgrade` → write `./upgrade.basil` from root of `includes/` then **run it**.
    * Also accept any other file or directory present in `includes/` (e.g., `website/` template, `kitchensink.basil`, `tests.basil`, etc.).
    * Safety: disallow absolute paths and `..` in the `target` argument to avoid path traversal.

**Non-goals:** No shared crate extraction yet; duplicate this in Basic later.

---

## Constraints & Requirements

* **No network.** Pure `include_bytes!` + code-generated module from a **local** `includes/` folder at **`basilc/includes/`**.
* **Arbitrary files** allowed (`.basil`, `.html`, `.css`, images, etc.). Preserve bytes exactly.
* **Overwrite policy:** If destination exists, overwrite files. Create directories as needed.
* **Portability:** Handle Windows/Linux/macOS paths. Use forward slashes for embedded logical paths; normalize when writing to disk.
* **Discoverability:** Add `basilc make --list` (or `-l`) to print all embedded paths (files only) and all top-level directories. This is helpful UX and good for tests.
* **Run behavior:** If the user asked to make a **single file**, run it immediately **after** writing (same as `basilc run <path>`). If they asked to make a **directory**, just write files; do not run.
* **No impact** on existing subcommands (`run`, `lex`, `test`, etc.).
* **Binary growth:** Acceptable. We’re embedding a small “starter kit.”
* **Tests:** Add a couple of integration tests behind a feature `embed-test` that simulate writing into a temp dir (no actual run of BASIC VM there; just verify extraction mapping and safety rules). Unit tests may be limited because the module code is generated at build time.
* **Packaging:** Ensure `basilc/includes/**` is **tracked in git** (so Debian orig tarball includes them). Do **not** ignore them.

---

## High-Level Plan

1. **Generate an embedded index at compile time** in `basilc/build.rs`.

    * Walk `basilc/includes/` (recursive).
    * For each file, write a line into `OUT_DIR/embedded_includes.rs` that uses `include_bytes!("includes/<rel>")`.
    * Record: `path` (logical path like `examples/hello.basil`) and `contents` (`&'static [u8]`).
    * Normalize to forward slashes for paths in the table.

2. **Add a small runtime helper module** (e.g., `basilc/src/embedded.rs`) that `include!`s the generated file and exposes:

    * `pub struct EmbeddedFile { pub path: &'static str, pub contents: &'static [u8] }`
    * `pub static EMBEDDED_FILES: &[EmbeddedFile]`
    * Helpers: `find_file`, `has_dir`, `extract_dir`, `write_file_to`, `list_all`, etc.

3. **Wire a `make` subcommand** in the CLI layer and call these helpers.

    * Implement path safety checks.
    * Single-file → write & **run**.
    * Directory → write recursively & **do not run**.
    * `--list` prints available entries.

4. **Add tests** (tempdir based) for:

    * listing;
    * writing a single file;
    * writing a directory;
    * rejecting `..` and absolute paths.

---

## Concrete Edits

### A) New folder & tracking

* Ensure these paths exist and are versioned:

    * `basilc/includes/` (already created by user)
    * `basilc/build.rs` (new)
    * `basilc/src/embedded.rs` (new)

**Do NOT** add any rule that ignores `basilc/includes/**`.

### B) `basilc/build.rs`

Create this file:

```rust
// basilc/build.rs

use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let includes_root = manifest_dir.join("includes");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("embedded_includes.rs");

    // If there is no includes/ folder, emit an empty table (no error).
    let mut entries: Vec<String> = Vec::new();
    if includes_root.exists() {
        collect_files(&includes_root, &includes_root, &mut entries).unwrap();
    }

    let mut f = File::create(&dest).unwrap();
    writeln!(
        f,
        r#"#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFile {{ pub path: &'static str, pub contents: &'static [u8] }}

pub static EMBEDDED_FILES: &[EmbeddedFile] = &["#
    )
    .unwrap();

    for logical in entries {
        // We generate lines like:
        // EmbeddedFile { path: "examples/hello.basil", contents: include_bytes!("includes/examples/hello.basil") },
        let include_path = format!("includes/{}", logical);
        writeln!(
            f,
            "    EmbeddedFile {{ path: {lp:?}, contents: include_bytes!({ip:?}) }},",
            lp = logical,
            ip = include_path
        )
        .unwrap();
    }

    writeln!(f, "];").unwrap();
    println!("cargo:rerun-if-changed=includes");
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let p = ent.path();
        let meta = ent.metadata()?;
        if meta.is_dir() {
            collect_files(root, &p, out)?;
        } else if meta.is_file() {
            let rel = p.strip_prefix(root).unwrap();
            // Normalize to forward slashes for a stable logical path
            let logical = rel.to_string_lossy().replace('\\', "/");
            out.push(logical);
        }
    }
    Ok(())
}
```

### C) `basilc/src/embedded.rs`

Create this module:

```rust
// basilc/src/embedded.rs

#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/embedded_includes.rs"));

use std::fs;
use std::path::{Path, PathBuf};

pub fn list_all_paths() -> impl Iterator<Item = &'static str> {
    EMBEDDED_FILES.iter().map(|f| f.path)
}

pub fn list_top_level_dirs() -> Vec<&'static str> {
    let mut dirs = Vec::new();
    for p in list_all_paths() {
        if let Some((first, _rest)) = p.split_once('/') {
            if !dirs.contains(&first) {
                dirs.push(first);
            }
        }
    }
    dirs
}

pub fn find_file(logical: &str) -> Option<&'static EmbeddedFile> {
    if let Some(f) = EMBEDDED_FILES.iter().find(|f| f.path == logical) {
        return Some(f);
    }
    // convenience: try "<name>.basil" for bare names like "upgrade"
    let fallback = format!("{logical}.basil");
    EMBEDDED_FILES.iter().find(|f| f.path == fallback)
}

pub fn has_dir(dir: &str) -> bool {
    let prefix = ensure_trailing_slash(dir);
    EMBEDDED_FILES.iter().any(|f| f.path.starts_with(&prefix))
}

pub fn write_single(logical: &str, dest_root: &Path) -> std::io::Result<PathBuf> {
    let file = find_file(logical).ok_or_else(|| not_found(logical))?;
    let out = resolved_output_path_for_file(logical, dest_root);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, file.contents)?;
    Ok(out)
}

pub fn extract_dir(dir: &str, dest_root: &Path) -> std::io::Result<()> {
    let prefix = ensure_trailing_slash(dir);
    let mut found_any = false;
    for f in EMBEDDED_FILES.iter() {
        if f.path.starts_with(&prefix) {
            found_any = true;
            let rel = &f.path[prefix.len()..]; // e.g. "hello.basil"
            let out = dest_root.join(dir).join(rel);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out, f.contents)?;
        }
    }
    if !found_any {
        return Err(not_found(dir));
    }
    Ok(())
}

fn not_found(name: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, format!("No embedded entry {name:?}"))
}

fn ensure_trailing_slash(s: &str) -> String {
    if s.ends_with('/') { s.to_string() } else { format!("{s}/") }
}

fn resolved_output_path_for_file(logical: &str, dest_root: &Path) -> PathBuf {
    // If logical contains a '/', treat as relative path under CWD.
    // If bare name like "upgrade", write "<cwd>/upgrade.basil".
    if logical.contains('/') || logical.ends_with(".basil") {
        dest_root.join(logical)
    } else {
        dest_root.join(format!("{logical}.basil"))
    }
}

pub fn is_unsafe_target(target: &str) -> bool {
    Path::new(target).is_absolute() || target.contains("..")
}
```

### D) Wire the CLI `make` subcommand

Find the CLI parsing in `basilc` (where `run`, `lex`, `test` are defined). Add:

* `basilc make <target>`
* `basilc make --list` (optional flag to just print embedded items and exit)

Pseudocode (adjust to your arg parser):

```rust
// basilc/src/main.rs (or wherever CLI dispatch happens)

mod embedded;

use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // ... existing CLI parsing ...

    match cmd {
        Command::Make { target, list } => {
            if list {
                print_embedded_inventory();
                return Ok(());
            }
            handle_make(&target)?;
        }
        // ... other subcommands ...
    }
    Ok(())
}

fn print_embedded_inventory() {
    println!("Embedded files:");
    for p in embedded::list_all_paths() {
        println!("  {}", p);
    }
    let dirs = embedded::list_top_level_dirs();
    if !dirs.is_empty() {
        println!("\nTop-level dirs: {}", dirs.join(", "));
    }
}

fn handle_make(target: &str) -> anyhow::Result<()> {
    if embedded::is_unsafe_target(target) {
        anyhow::bail!("Refusing unsafe target: {target}");
    }

    let cwd = env::current_dir()?;

    let is_dir = embedded::has_dir(target);
    let file = embedded::find_file(target);

    if is_dir && file.is_none() {
        embedded::extract_dir(target, &cwd)?;
        println!("Wrote directory: {target}/");
        return Ok(());
    }

    if file.is_some() {
        let out = embedded::write_single(target, &cwd)?;
        println!("Wrote file: {}", out.display());

        // If it's a single file, run it.
        run_script(&out)?;
        return Ok(());
    }

    if is_dir {
        embedded::extract_dir(target, &cwd)?;
        println!("Wrote directory: {target}/");
        return Ok(());
    }

    // Nothing matched
    anyhow::bail!("No embedded file or dir named {target:?}. Try `basilc make --list`.")
}

// Stub: hook this into your existing interpreter path for "run"
fn run_script(path: &PathBuf) -> anyhow::Result<()> {
    // Reuse your existing 'run' flow
    // Example:
    //   let src = std::fs::read_to_string(path)?;
    //   basil::execute(&src)?;
    crate::cli::run_path(path) // or whatever your existing function is
}
```

*(Adjust names/namespaces to your codebase. If you use `clap`, define a `Make` subcommand struct with fields `target: String`, `list: bool`.)*

### E) Tests (optional but helpful)

Add a small test module gated by `#[cfg(feature = "embed-test")]` that:

* Creates a tempdir,
* Calls the helpers to write a known single file (`upgrade.basil`) and a known directory (`examples/`),
* Asserts that files exist and bytes match `EMBEDDED_FILES`,
* Asserts that `is_unsafe_target("/abs")` and `is_unsafe_target("../etc")` return true.

You can include a tiny fixture under `basilc/includes/examples/hello.basil` for predictable assertions.

### F) Docs & `--help`

Add help text for the new subcommand:

```
basilc make <target>
    Write an embedded file or directory from the built-in includes/ tree into the current directory.
    If <target> is a single file, it's written AND executed.
    If <target> is a directory, it's written recursively and not executed.

Examples:
  basilc make examples
  basilc make examples/hello.basil
  basilc make upgrade
  basilc make --list
```

---

## Acceptance Criteria

* `cargo build --release` succeeds.
* `basilc make --list` prints the embedded items from `basilc/includes`.
* `basilc make examples` creates `./examples/` with the same structure and bytes as `basilc/includes/examples`.
* `basilc make examples/hello.basil` writes `./examples/hello.basil` and **runs** it using the existing interpreter flow.
* `basilc make upgrade` writes `./upgrade.basil` and **runs** it.
* Safety: `basilc make /etc/passwd` and `basilc make ../foo` are rejected with a clear error.
* Works cross-platform (paths normalized; tests don’t rely on OS-specific separators).
* No changes to other subcommands.

---

## Notes for Packaging (FYI)

* Ensure `basilc/includes/**` is committed to git so it ends up in Debian orig tarball and your vendored offline builds.
* No special changes needed to Debian rules; this is just extra code/bytes in the binary.


