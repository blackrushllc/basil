Love it. Once **`bcc`** is green and producing real EXEs, here’s the tight next-step plan I’d run:

# Next steps (in order)

1. **Lock versions & publish runtime crates**

    * Pin `libbasilrt` and each `basil-obj-*` to `=x.y.z`.
    * Publish to crates.io so AOT builds work without cloning the repo.
    * Tag repo `v0.x` and create a CHANGELOG.

2. **Golden parity suite**

    * Choose ~20 representative `.basil` programs (loops, strings, arrays, objects, I/O, a DAW/MIDI sample, a TERM sample).
    * CI: run each with **`basilc`** and with compiled EXE (via **`bcc`**) and **diff** outputs.

3. **User-facing docs & samples**

    * Finalize `docs/AOT_COMPILER.md`, add a **Quickstart** in README.
    * Add `examples/aot/*` with one per feature (audio, midi, daw, term).
    * Provide a one-liner installer snippet for each OS.

4. **Release builds**

    * Produce signed binaries: `basilc` (VM, kitchen-sink features) and `bcc`.
    * Offer Linux `musl` statics, Windows MSVC, macOS universal (if feasible).
    * Attach to a GitHub Release with checksums.

5. **DX polish**

    * `bcc --explain-features app.basil` (print auto-detected + effective set + crates).
    * `bcc --emit-project out/` (air-gapped/offline path).
    * `basilc new <app>` to scaffold a Basil project folder with Kitchen-Sink style.

6. **Performance pass**

    * Benchmarks: VM vs AOT on hot loops, string concat, file I/O.
    * Low-hanging opts: const-folding, DCE already there; add simple inlining for tiny procs in the emitter.

7. **Debuggability**

    * Emit `// @basil: file:line:col` already; add `--debug-spans` to include richer comments.
    * Optional next: map runtime errors to Basil lines (thread-local “current span”).

8. **Feature ABI hardening**

    * Document stable symbol → feature mapping.
    * Add compile-time checks that a missing feature yields a crisp, early error (already planned) and ensure the message lists the exact flag to add.

9. **Packaging & templates**

    * Starter templates: “CLI App,” “TUI App (TERM),” “MIDI Monitor,” “DAW Recorder.”
    * Make `bcc --template tui` accept a tiny config to pre-enable features and emit a ready-to-build crate.

10. **Editor/IDE touchups**

* Ship the **TextMate grammar** + VS Code snippet pack and a Geany config bundle.
* (Later) lightweight LSP for syntax/hover using your `basil-frontend`.

11. **Roadmap to v0.2**

* Decide: add DWARF debug info later (optional), incremental build cache improvements, and a minimal profiler (`--time-ir`, `--time-build`).

12. **Announce & collect feedback**

* Post release notes, small gifs of `bcc aot hello.basil` → run.
* Create a “Known issues” section with platform quirks and fixes.

---














# **Release Checklist** and a **Parity Test Manifest** (with a tiny runner + CI job). They’re designed so Junie (and you) can paste them straight into the repo and go.

## Release checklist (first public AOT + VM release)

**File:** `docs/RELEASE_CHECKLIST.md`

```
# Basil Release Checklist (VM: basilc, Compiler: bcc)

## 0) Preconditions
- [ ] CI green on development branch (VM + bcc parity).
- [ ] All examples build/run on Windows, macOS, Linux (including musl static).
- [ ] Versions in the workspace updated (Cargo.toml, libbasilrt, basil-obj-*).
- [ ] CHANGELOG.md updated with highlights & breaking changes (if any).

## 1) Versioning & tagging
- [ ] Bump versions and pin internal deps to exact (=x.y.z).
- [ ] Commit: "chore(release): v0.x.y"
- [ ] Tag: `git tag v0.x.y && git push --tags`

## 2) Publish crates (runtime first)
Order matters so `bcc` can resolve published versions:
- [ ] `libbasilrt`
- [ ] `basil-obj-audio`
- [ ] `basil-obj-midi`
- [ ] `basil-obj-daw`
- [ ] `basil-obj-term` (or whichever are “stable”)
- [ ] Any other `basil-obj-*`
(Use `cargo publish --dry-run` first, then `cargo publish`)

## 3) Build signed binaries
- [ ] basilc (kitchen-sink feature build)
- [ ] bcc
Targets:
- [ ] Linux (x86_64-unknown-linux-gnu)
- [ ] Linux static (x86_64-unknown-linux-musl)
- [ ] Windows (x86_64-pc-windows-msvc)
- [ ] macOS (aarch64-apple-darwin) and/or (x86_64-apple-darwin)

Create checksums:
- [ ] `shasum -a 256 <artifact> > <artifact>.sha256` (or `certutil` on Windows)

Sign (optional):
- [ ] GPG sign the checksums or binaries as per your policy.

## 4) Smoke tests on artifacts
For each OS:
- [ ] `./basilc --version` and `./bcc --version`
- [ ] `./basilc run examples/hello.basil`
- [ ] `./bcc aot examples/hello.basil && ./target-out/hello`

## 5) GitHub Release
- [ ] Draft Release `v0.x.y`
- [ ] Attach artifacts + `.sha256`
- [ ] Paste highlights: new `bcc`, how to AOT, features handling, Quickstart snippet
- [ ] Link: docs/AOT_COMPILER.md and docs/RELEASE_CHECKLIST.md
- [ ] Publish

## 6) Post-release
- [ ] Update README “Install” with binary links & `rustup` note for AOT.
- [ ] Open "v0.x+1 Roadmap" issue with next goals (debug symbols, templates, LSP).
- [ ] Announce (tweet, GH discussion, Reddit, Discord).
```

---

## Parity test manifest (VM vs AOT)

**Layout (add to repo):**

```
/tests/parity/
  manifest.toml
  run.sh
  cases/
    01_hello.basil           01_hello.out
    02_print_types.basil     02_print_types.out
    03_if_else.basil         03_if_else.out
    04_loops.basil           04_loops.out
    05_strings.basil         05_strings.out
    06_arrays_1d.basil       06_arrays_1d.out
    07_arrays_2d_str.basil   07_arrays_2d_str.out
    08_functions.basil       08_functions.out
    09_for_each.basil        09_for_each.out
    10_input_echo.basil      10_input_echo.out
    11_term_sample.basil     11_term_sample.out
    12_audio_stub.basil      12_audio_stub.out
    13_midi_stub.basil       13_midi_stub.out
    14_daw_stub.basil        14_daw_stub.out
    15_errors.basil          15_errors.out
    16_objects.basil         16_objects.out
    17_obj_array.basil       17_obj_array.out
    18_math_edge.basil       18_math_edge.out
    19_concat_perf.basil     19_concat_perf.out
    20_control_mix.basil     20_control_mix.out
```

**`manifest.toml`** (keeps features deterministic per case)

```toml
# tests/parity/manifest.toml
# Each case references features the compiled program must include.
# The VM (basilc) runs with your kitchen-sink build.

[defaults]
timeout_ms = 15000
features = ["term"]  # baseline stable set for AOT unless overridden

[[case]]
name = "hello"
file = "cases/01_hello.basil"

[[case]]
name = "print_types"
file = "cases/02_print_types.basil"

[[case]]
name = "term_sample"
file = "cases/11_term_sample.basil"
features = ["term"]

[[case]]
name = "audio_stub"
file = "cases/12_audio_stub.basil"
features = ["audio"]

[[case]]
name = "midi_stub"
file = "cases/13_midi_stub.basil"
features = ["midi"]

[[case]]
name = "daw_stub"
file = "cases/14_daw_stub.basil"
features = ["daw","audio","midi"]

# ...and so on for all listed cases
```

**Example case contents** (keep outputs deterministic—no timestamps/random):

```
-- 01_hello.basil
PRINTLN "Hello, Basil!";
```

```
-- 01_hello.out
Hello, Basil!
```

```
-- 11_term_sample.basil
PRINTLN "TERM OK";  ' avoid cursor ops; just ensure the module links
```

```
-- 11_term_sample.out
TERM OK
```

*(For AUDIO/MIDI/DAW stubs, call a no-op or a dry-run function that prints a fixed line, e.g., `PRINTLN "AUDIO OK";` via your facade—keeps CI hermetic.)*

**`run.sh`** (Unix runner; add a `.cmd` if you want Windows native too)

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/../.. && pwd)"
PARITY_DIR="$ROOT/tests/parity"
CASES="$PARITY_DIR/cases"
MANI="$PARITY_DIR/manifest.toml"

BASILC="${BASILC:-$ROOT/target/release/basilc}"
BCC_BIN="${BCC_BIN:-$ROOT/target/release/bcc}"

fail=0

# jq-like parsing with awk (simple): grabs values from manifest
parse_cases() {
  awk -v RS= -v ORS= '
    /[[case]]/ {
      name=""; file=""; features="";
      n=split($0, lines, "\n");
      for (i=1;i<=n;i++){
        if (match(lines[i], /name *= *"(.*)"/, m)) name=m[1];
        if (match(lines[i], /file *= *"(.*)"/, m)) file=m[1];
        if (match(lines[i], /features *= *\[(.*)\]/, m)) features=m[1];
      }
      gsub(/ /, "", features);
      print name "|" file "|" features "\n";
    }
  ' "$MANI"
}

while IFS='|' read -r name file features; do
  [[ -z "$name" ]] && continue
  in="$PARITY_DIR/$file"
  out="${in%.*}.out"
  echo "==> $name"

  # 1) VM output
  vm_out="$(mktemp)"; trap 'rm -f "$vm_out"' EXIT
  "$BASILC" run "$in" > "$vm_out"

  # 2) AOT compile + run
  aot_out="$(mktemp)"; trap 'rm -f "$aot_out"' EXIT
  feat_flag=""
  if [[ -n "$features" ]]; then
    # Convert like: "audio","midi" -> obj-audio,obj-midi
    feats="$(echo "$features" | sed 's/"/obj-/g; s/,/,obj-/g')"
    feat_flag="--features $feats"
  fi
  "$BCC_BIN" aot "$in" $feat_flag --name "parity_$name" --quiet
  # bcc should print/know the output path; assuming default ./target-out/<name>
  exe="./target-out/parity_$name"
  [[ -f "$exe" || -f "$exe.exe" ]] || { echo "AOT exe missing for $name"; fail=1; continue; }
  [[ -f "$exe" ]] || exe="$exe.exe"
  "$exe" > "$aot_out"

  # 3) Compare against golden expected first, then cross-compare VM vs AOT
  if ! diff -u "$out" "$vm_out"; then
    echo "Golden mismatch (VM) in $name"; fail=1
  fi
  if ! diff -u "$vm_out" "$aot_out"; then
    echo "VM vs AOT mismatch in $name"; fail=1
  fi
done < <(parse_cases)

exit $fail
```

> Make executable: `chmod +x tests/parity/run.sh`

---

## CI job (add to your GitHub Actions)

**File:** `.github/workflows/parity.yml`

```yaml
name: parity
on:
  push:
  pull_request:

jobs:
  parity:
    strategy:
      matrix:
        os: [ubuntu-latest]
        rust: [stable]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with: { toolchain: ${{ matrix.rust }} }

      - name: Build VM and compiler
        run: |
          cargo build --release -p basilc
          cargo build --release -p bcc

      - name: Run parity tests
        run: tests/parity/run.sh
```

*(You can add a second job later for Windows/macOS and another for `x86_64-unknown-linux-musl` static builds.)*

---

