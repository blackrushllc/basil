# How to ask ChatGPT to generate a prompt for you give to Junie Ultimate to build a new feature module for Basil

Just paste the following *raw* markdown into ChatGPT (or Junie, or any future model) whenever you want to continue Basil development — it’ll bring the AI up to speed with how to generate a new feature module:

---

# 🧭 Basil Feature Development Prep Prompt

I’m continuing development on **Basil**, our BASIC-style interpreter written in **Rust**, which compiles `.basil` source files into bytecode (`.basilx`) and executes them on a custom VM.

This prompt restores the entire Basil context for building **feature modules**, **language extensions**, and **runtime improvements**.

---

## 🧩 Overview

Basil is designed to be a **modern yet minimal BASIC** for the web and shell — structured, portable, and easily extensible through **Rust object modules**.

Each module:

* Lives in `basil-object/src/`
* Registers BASIC-accessible functions and procedures
* Compiles conditionally using Cargo `--features`
* Can be included in an umbrella feature (`obj-all`)

All modules follow this pattern:

```bash
cargo run -q -p basilc --features obj-curl -- run examples/curl_demo.basil
```

The interpreter:

* Automatically compiles `.basil` → `.basilx` bytecode files
* Skips recompilation if source unchanged
* Supports both **CLI** and **CGI** execution modes

---

## 🧱 Existing Features

| Module         | Crate               | Main Functions                                                                                                                    |
| -------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **obj-base64** | `base64`            | `BASE64_ENCODE$()`, `BASE64_DECODE$()`                                                                                            |
| **obj-zip**    | `zip`, `walkdir`    | `ZIP_COMPRESS_FILE`, `ZIP_COMPRESS_DIR`, `ZIP_EXTRACT_ALL`, `ZIP_LIST$()`                                                         |
| **obj-curl**   | `ureq`              | `HTTP_GET$()`, `HTTP_POST$()`                                                                                                     |
| **obj-json**   | `serde_json`        | `JSON_PARSE$()`, `JSON_STRINGIFY$()`                                                                                              |
| **obj-csv**    | `csv`, `serde_json` | `CSV_PARSE$()`, `CSV_WRITE$()`                                                                                                    |
| **obj-sqlite** | `rusqlite`          | `SQLITE_OPEN%()`, `SQLITE_CLOSE()`, `SQLITE_EXEC%()`, `SQLITE_QUERY2D$()`, `SQLITE_LAST_INSERT_ID%()` (returns 2-D string arrays) |

---

## ⚙️ Cargo Feature Model

All modules are declared in `basil-object/Cargo.toml` as **optional**:

```toml
[features]
obj-base64 = ["base64"]
obj-zip    = ["zip", "walkdir"]
obj-curl   = ["ureq"]
obj-json   = ["serde_json"]
obj-csv    = ["csv", "serde_json"]
obj-sqlite = ["rusqlite"]

# Umbrella feature to include everything
obj-all = [
  "obj-base64",
  "obj-zip",
  "obj-curl",
  "obj-json",
  "obj-csv",
  "obj-sqlite"
]

[dependencies]
base64    = { version = "0.22", optional = true }
zip       = { version = "0.6", optional = true, default-features = false, features = ["deflate"] }
walkdir   = { version = "2", optional = true }
ureq      = { version = "2.9", optional = true, features = ["json", "tls"] }
serde_json= { version = "1", optional = true }
csv       = { version = "1.3", optional = true }
rusqlite  = { version = "0.31", optional = true, features = ["bundled"] }
```

Cargo only includes code and dependencies for features that are explicitly enabled.

---

## 🧠 Interpreter Expectations

Basil supports 1-D and 2-D string arrays:

```basil
DIM A$(10)      ' 11 elements, 1-D
DIM B$(10,10)   ' 11x11, 2-D
```

Whole-array assignment:

```basil
DIM rows$(0,0)
LET rows$() = SQLITE_QUERY2D$(db%, "SELECT * FROM users")
```

### Runtime behavior:

* The interpreter **auto-redimensions** 2-D arrays when assigned a `Value::StrArray2D`.
* Arrays store strings row-major.
* `ARRAY_ROWS%(arr$())` and `ARRAY_COLS%(arr$())` return row/column counts.
* Future upgrades:

    * `TYPE..END TYPE` (record structures)
    * parameterized SQL queries
    * enhanced `FOREACH` over 2-D arrays
    * object persistence (`SAVE class@ TO file$`)

---

## 🧩 Adding a New Feature Module

When creating a new Basil feature (e.g. `obj-regex`, `obj-pdf`, `obj-crypto`), follow this pattern:

1. Add the feature and optional dependency in `basil-object/Cargo.toml`.
2. Create a file `basil-object/src/<name>.rs` implementing:

   ```rust
   pub fn register(reg: &mut Registry) {
       reg.func("FUNCNAME$", func_name);
       reg.proc("PROCNAME", proc_name);
   }
   ```
3. Register it in `basil-object/src/lib.rs`:

   ```rust
   #[cfg(feature = "obj-<name>")]
   mod <name>;

   pub fn register_objects(reg: &mut Registry) {
       #[cfg(feature = "obj-<name>")] <name>::register(reg);
       // ...
   }
   ```
4. Add a BASIC example in `/examples/<name>_demo.basil`.
5. Update the `obj-all` feature to include it.

---

## 🔍 Auto-Detection Note: 2-D String Array Support

Before adding any module that returns structured data (like `obj-sqlite`, `obj-csv`, `obj-regex` in the future),
**check if the interpreter supports the `Value::StrArray2D` type** and whole-array assignment logic:

```rust
enum Value {
    // ...
    StrArray2D { rows: usize, cols: usize, data: Vec<String> },
}
```

If it’s missing, add it first — along with:

* `ARRAY_ROWS%(arr$())`, `ARRAY_COLS%(arr$())`
* assignment support for `LET arr$() = Expr`
* automatic resizing behavior

That runtime capability is what allows Basil to treat SQL result sets and other table-like data as native BASIC arrays.

---

## 🪄 Example: How to Ask for a New Feature

If you (or any new developer) want to extend Basil, just describe what you need in plain English.

For example:

> “Please give me a Junie prompt to implement `obj-regex` for Basil.
> It should use the Rust `regex` crate and expose two BASIC functions:
> `REGEX_MATCH$(pattern$, text$)` — returns `"TRUE"` or `""`,
> and `REGEX_EXTRACT$(pattern$, text$)` — returns the first matching substring.
> Keep it consistent with other modules: feature flag `obj-regex`,
> registration in `basil-object/src/lib.rs`, and an example `examples/regex_demo.basil`.
> Add it to the `obj-all` umbrella feature.”

That’s it.
The assistant (or Junie) will then generate a ready-to-paste implementation prompt, following all the conventions above — feature flag, registration, example, and BASIC-friendly interface.

---

## ✅ Purpose of This File

Use this file anytime you (or another developer) want to:

* Resume Basil development even if context is lost
* Onboard a new AI assistant
* Define new features, syntax, or runtime behaviors
* Request “Junie prompts” for feature modules

Paste this whole thing into the chat first — it’ll reestablish everything we’ve done so far.
Then just ask naturally, e.g.:

> “Now let’s add `obj-crypto` with `MD5$(text$)` and `SHA256$(text$)` using `md5` and `sha2`.”

and you’ll get a complete, copy-ready Junie prompt matching the existing Basil ecosystem.

---


