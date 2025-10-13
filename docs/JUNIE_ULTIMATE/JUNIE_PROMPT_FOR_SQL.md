# 🧠 Junie Prompt — Implement `obj-sqlite` (rusqlite) with 2-D string array results

**Project:** Basil
**Module:** `obj-sqlite`
**Goal:** Provide a minimal SQLite integration for BASIC scripts.

* Open/close connections with integer handles
* Execute non-query SQL
* Run queries that **return a 2-D string array** (rows × columns)
* Interpreter must support whole-array assignment: `LET rows$() = SQLITE_QUERY2D$(...)` which **auto-redimensions** the target array to match the result set.

---

## 0) Cargo features & deps (basil-object/Cargo.toml)

Add optional dependency and feature gate:

```toml
[features]
# …
obj-sqlite = ["rusqlite"]

# Umbrella (include if present in your repo):
obj-all = [
  "obj-base64",
  "obj-zip",
  "obj-curl",
  "obj-json",
  "obj-csv",
  "obj-sqlite",
  "obj-bmx"
]

[dependencies]
rusqlite = { version = "0.31", features = ["bundled"], optional = true }
```

> Use `features = ["bundled"]` so SQLite builds without system libs. Adjust if you prefer linking to system SQLite.

---

## 1) Runtime support (VM/compiler) — whole-array assignment

**Add minimal runtime plumbing** so functions can return a **2-D string array value**, and assigning to `arr$()` resizes and copies:

* In your core `Value` enum, add:

  ```rust
  // Pseudocode; match your actual Value type
  enum Value {
      // …
      StrArray2D { rows: usize, cols: usize, data: Vec<String> }, // row-major
  }
  ```

* In the assignment path (e.g., `eval_let` or equivalent), detect:

    * **LHS** is a string array variable with empty parens (`rows$()` meaning “entire array”).
    * **RHS** is `Value::StrArray2D { rows, cols, data }`.
    * **Behavior**: redimension the LHS array to `(rows, cols)` and copy `data` row-major.
      This provides the “auto-resize to fit results” behavior.

* Parser: ensure `LET arr$() = Expr` parses as “whole-array assignment” (if not already supported).

* Add two tiny helpers (built-ins) for convenience:

  ```basil
  ARRAY_ROWS%(arr$())   ' returns number of rows
  ARRAY_COLS%(arr$())   ' returns number of cols
  ```

  These should work for any 2-D string array.

---

## 2) Module file (basil-object/src/sqlite.rs)

Implement a small handle table and minimal API.

### 2.1 Registry & handle store

```rust
use basil_runtime::prelude::*; // Registry, Value, RuntimeError, etc.
use once_cell::sync::Lazy;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::sync::Mutex;

static CONNS: Lazy<Mutex<ConnTable>> = Lazy::new(|| Mutex::new(ConnTable::default()));

#[derive(Default)]
struct ConnTable {
    next: i32,
    map: HashMap<i32, Connection>,
}

impl ConnTable {
    fn insert(&mut self, conn: Connection) -> i32 {
        self.next += 1;
        let id = self.next;
        self.map.insert(id, conn);
        id
    }
    fn get(&mut self, id: i32) -> Option<&mut Connection> {
        self.map.get_mut(&id)
    }
    fn remove(&mut self, id: i32) -> bool {
        self.map.remove(&id).is_some()
    }
}

pub fn register(reg: &mut Registry) {
    // Connections
    reg.func("SQLITE_OPEN%",  sqlite_open);   // path$ -> handle% (0 on failure)
    reg.proc("SQLITE_CLOSE",  sqlite_close);  // handle%

    // Exec (DDL/DML)
    reg.func("SQLITE_EXEC%",  sqlite_exec);   // handle%, sql$ -> affected_rows% (or -1 on error)

    // Query -> 2D string array (returned as Value::StrArray2D)
    reg.func("SQLITE_QUERY2D$", sqlite_query2d); // handle%, sql$ -> StrArray2D value

    // Convenience
    reg.func("SQLITE_LAST_INSERT_ID%", sqlite_last_insert_id); // handle% -> integer
}
```

### 2.2 Helpers

```rust
fn arg_str(args: &[Value], idx: usize, name: &str) -> Result<&str, RuntimeError> {
    args.get(idx).and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::new(&format!("{name} required")))
}

fn arg_int(args: &[Value], idx: usize, name: &str) -> Result<i32, RuntimeError> {
    args.get(idx).and_then(|v| v.as_int())
        .ok_or_else(|| RuntimeError::new(&format!("{name} required")))
}
```

### 2.3 Functions

```rust
fn sqlite_open(args: &[Value]) -> Result<Value, RuntimeError> {
    let path = arg_str(args, 0, "SQLITE_OPEN%: path$")?;
    match Connection::open(path) {
        Ok(conn) => {
            let mut tbl = CONNS.lock().unwrap();
            let id = tbl.insert(conn);
            Ok(Value::Int(id))
        }
        Err(_e) => Ok(Value::Int(0)), // per spec: 0 on failure
    }
}

fn sqlite_close(args: &[Value]) -> Result<Value, RuntimeError> {
    let handle = arg_int(args, 0, "SQLITE_CLOSE: handle%")?;
    let mut tbl = CONNS.lock().unwrap();
    let _ = tbl.remove(handle);
    Ok(Value::Empty)
}

fn sqlite_exec(args: &[Value]) -> Result<Value, RuntimeError> {
    let handle = arg_int(args, 0, "SQLITE_EXEC%: handle%")?;
    let sql = arg_str(args, 1, "SQLITE_EXEC%: sql$")?;
    let mut tbl = CONNS.lock().unwrap();
    let conn = tbl.get(handle).ok_or_else(|| RuntimeError::new("SQLITE_EXEC%: invalid handle"))?;
    match conn.execute(sql, []) {
        Ok(affected) => Ok(Value::Int(affected as i32)),
        Err(_e) => Ok(Value::Int(-1)),
    }
}

fn sqlite_query2d(args: &[Value]) -> Result<Value, RuntimeError> {
    let handle = arg_int(args, 0, "SQLITE_QUERY2D$: handle%")?;
    let sql = arg_str(args, 1, "SQLITE_QUERY2D$: sql$")?;

    let mut tbl = CONNS.lock().unwrap();
    let conn = tbl.get(handle).ok_or_else(|| RuntimeError::new("SQLITE_QUERY2D$: invalid handle"))?;

    let mut stmt = conn.prepare(sql)
        .map_err(|e| RuntimeError::new(&format!("SQLITE_QUERY2D$: prepare failed: {e}")))?;

    let col_cnt = stmt.column_count() as usize;

    let rows_iter = stmt.query_map([], |row| {
        let mut out: Vec<String> = Vec::with_capacity(col_cnt);
        for i in 0..col_cnt {
            // to string; NULL -> ""
            let v: rusqlite::types::Value = row.get::<usize, rusqlite::types::Value>(i)?;
            let s = match v {
                rusqlite::types::Value::Null => String::new(),
                rusqlite::types::Value::Integer(n) => n.to_string(),
                rusqlite::types::Value::Real(f) => {
                    // keep it readable
                    let mut s = f.to_string();
                    if s.ends_with(".0") { s.truncate(s.len()-2); }
                    s
                }
                rusqlite::types::Value::Text(t) => String::from_utf8_lossy(&t).to_string(),
                rusqlite::types::Value::Blob(b) => base64::engine::general_purpose::STANDARD.encode(b),
            };
            out.push(s);
        }
        Ok(out)
    }).map_err(|e| RuntimeError::new(&format!("SQLITE_QUERY2D$: query failed: {e}")))?;

    // Collect all rows, flatten row-major
    let mut data: Vec<String> = Vec::new();
    let mut row_cnt = 0usize;
    for r in rows_iter {
        let row = r.map_err(|e| RuntimeError::new(&format!("SQLITE_QUERY2D$: row read failed: {e}")))?;
        if row.len() != col_cnt {
            return Err(RuntimeError::new("SQLITE_QUERY2D$: inconsistent column count"));
        }
        data.extend(row);
        row_cnt += 1;
    }

    // Return special 2D string array value → assignment will auto-redim target
    Ok(Value::StrArray2D { rows: row_cnt, cols: col_cnt, data })
}

fn sqlite_last_insert_id(args: &[Value]) -> Result<Value, RuntimeError> {
    let handle = arg_int(args, 0, "SQLITE_LAST_INSERT_ID%: handle%")?;
    let mut tbl = CONNS.lock().unwrap();
    let conn = tbl.get(handle).ok_or_else(|| RuntimeError::new("SQLITE_LAST_INSERT_ID%: invalid handle"))?;
    let id = conn.last_insert_rowid();
    Ok(Value::Int(id as i32))
}
```

> Note: above uses `base64` for BLOB→string fallback; if you don’t already depend on it, you can either add `base64` optional dep or replace with `"[BLOB]"`.

---

## 3) Register module (basil-object/src/lib.rs)

```rust
#[cfg(feature = "obj-sqlite")]
mod sqlite;

pub fn register_objects(reg: &mut Registry) {
    #[cfg(feature = "obj-sqlite")] { sqlite::register(reg); }
    // …existing registrations…
}
```

---

## 4) Built-in helpers (VM) — rows/cols

Add two simple built-ins, usable in loops:

* `ARRAY_ROWS%(arr$())` → returns row count for a 2-D string array
* `ARRAY_COLS%(arr$())` → returns column count

(Implement by inspecting your array header / dimension metadata.)

---

## 5) BASIC usage examples

### 5.1 Query table → 2-D array

```basil
PRINTLN "SQLite demo"

db% = SQLITE_OPEN%("demo.db")
IF db% = 0 THEN
  PRINTLN "Failed to open DB"
  END
ENDIF

' Ensure table exists
rows% = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)")
PRINTLN "DDL ok, rows%=", rows%

' Insert a row
rows% = SQLITE_EXEC%(db%, "INSERT INTO users(name) VALUES ('Erik')")
id% = SQLITE_LAST_INSERT_ID%(db%)
PRINTLN "Inserted id=", id%

' Prepare 2D array and fill with query results.
' We require the variable to exist as 2D, so start with (0,0) — it will be auto-redimensioned.
DIM rows$(0,0)
LET rows$() = SQLITE_QUERY2D$(db%, "SELECT id, name FROM users ORDER BY id")

rmax% = ARRAY_ROWS%(rows$())
cmax% = ARRAY_COLS%(rows$())

FOR r% = 0 TO rmax% - 1
  PRINTLN rows$(r%, 0), " - ", rows%(r%, 1)
NEXT

SQLITE_CLOSE(db%)
```

### 5.2 Empty results

```basil
DIM r$(0,0)
LET r$() = SQLITE_QUERY2D$(db%, "SELECT * FROM users WHERE 1=0")
PRINTLN "rows=", ARRAY_ROWS%(r$()), " cols=", ARRAY_COLS%(r$())   ' expect 0, N
```

---

## 6) Notes & v1 constraints

* **No parameters** in v1. We’ll add parameterized queries later (`SQLITE_QUERY2D_PARAM$`).
* All values are converted to **strings**. `NULL` → `""`.
  Numbers stringify; BLOBs base64 (or placeholder) — keep it deterministic.
* The **caller must declare the target** 2-D string array variable (like `DIM rows$(0,0)`) before whole-array assignment. The interpreter takes care of resizing.
* `FOREACH` behavior with 2-D arrays can come later; loops can use indices with `ARRAY_ROWS%/COLS%`.

---

## 7) Build & run

```bash
# SQLite only
cargo run -q -p basilc --features obj-sqlite -- run examples/sqlite_demo.basil

# Umbrella (everything)
cargo run -q -p basilc --features obj-all -- run examples/sqlite_demo.basil
```


