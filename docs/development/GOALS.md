# Goals for Basil 🌿, a modern systems scripting language

🌱 This is what I want to be able to build with Basil, a BASIC-flavored language designed for web and backend development. The focus is on **readability**, **developer experience**, and **extensibility** via multiple stable FFI paths.

# 1) 🌱 A tiny web service (routes + middleware)

```basil
MODULE app

IMPORT web/http        ' server, Request, Response, Context
IMPORT std/json        ' Json, encode, decode
IMPORT db/sqlite       ' Sql.open, Query
IMPORT auth/jwt        ' verify, sign
IMPORT time            ' now()

' ---- config ---------------------------------------------------------------
CONST PORT = 8080
LET db = AWAIT Sql.open("file:data.db?mode=rwc")

' ---- middleware: request logging -----------------------------------------
FUNC useLogger(next AS Handler) AS Handler
    RETURN FUNC (ctx AS Context) AS Response
        LET started = time.now()
        LET res = AWAIT next(ctx)
        PRINT "[", started, "] ", ctx.req.method, " ", ctx.req.path, " -> ", res.status
        RETURN res
    END
END

' ---- route handlers -------------------------------------------------------
FUNC getHealth(ctx AS Context) AS Response
    RETURN 200 JSON { status: "ok", now: time.now() }
END

TYPE Todo
    id   AS INT
    text AS STRING
    done AS BOOL = FALSE
END

FUNC listTodos(ctx AS Context) AS Response
    LET rows = AWAIT db.query("SELECT id, text, done FROM todos ORDER BY id DESC")
    RETURN 200 JSON rows
END

FUNC createTodo(ctx AS Context) AS Response
    LET body AS Todo = AWAIT ctx.req.json(Todo)
    LET id = AWAIT db.execOne(
        "INSERT INTO todos (text, done) VALUES (?, ?) RETURNING id",
        body.text, body.done
    )
    RETURN 201 JSON { id: id }
END

FUNC toggleTodo(ctx AS Context) AS Response
    LET id = INT(ctx.params["id"])
    LET updated = AWAIT db.exec(
        "UPDATE todos SET done = NOT done WHERE id = ?", id)
    IF updated = 0 THEN RETURN 404 JSON { error: "not found" }
    RETURN 204
END

' ---- server bootstrap -----------------------------------------------------
FUNC main()
    LET app = http.new()
    app.use(useLogger)

    app.get("/health", getHealth)
    app.get("/todos", listTodos)
    app.post("/todos", createTodo)
    app.post("/todos/:id/toggle", toggleTodo)

    AWAIT app.listen(PORT)
END
```

### Notes

* **BASIC vibes**: keywords, simple types, `FUNC/END`, but everything is **expression-friendly** and **async/await** is first-class.
* **Typed (optional)**: you can omit types for dynamic feel, or annotate for tooling/perf.
* **HTTP surface** is Rack/WSGI/PSR-like: handlers take a `Context`, return a `Response`.

# 2) 🌱 Async DB (SQLite → later Postgres)

```basil
IMPORT db/sqlite

FUNC migrate()
    AWAIT db.exec("
        CREATE TABLE IF NOT EXISTS todos(
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            done BOOLEAN NOT NULL DEFAULT 0
        );
    ")
END
```

* DB API is **awaitable**.
* Query results decode into **records** automatically if a target type is provided.

# 3) 🌱 WASI plug-in boundary (safe, portable extensions)

**Goal:** Allow third-party features to be added without native code on your host—perfect for SaaS and “upload your plugin” scenarios.

### 3.1 🌱 Define a WIT interface (component model)

```wit
package basil:plugins@1.0.0;

world text-utils {
  import host-log: func(msg: string)

  export to-upper: func(s: string) -> string
  export checksum: func(bytes: list<u8>) -> u32
}
```

* The plugin exports two functions; it can **import `host-log`** to print via the host’s logger.

### 3.2 🌱 Using it from Basil (host side)

```basil
IMPORT wasi/load

LET plug = AWAIT wasi.loadComponent("plugins/text_utils.wasm", world="text-utils")

FUNC demoWasi()
    plug.call("host-log", "hello from host")     ' import called by plugin
    LET out  = plug.call("to-upper", "hello")
    LET sum  = plug.call("checksum", BYTES("abc"))
    PRINT out, " ", sum
END
```

* Under the hood this is **WASI Preview 2 / Component Model**; values cross the boundary as **strings/bytes** (no raw pointers).

# 4) 🌱 Native C-ABI for high-performance extensions

Keep it tiny and stable. Think **“Lua-like”** API with handles and explicit memory management. Below is a sketch:

### 4.1 🌱 C header (`basil.h`)

```c
#ifndef BASIL_H
#define BASIL_H

#include <stdint.h>
#include <stddef.h>

#ifdef _WIN32
  #ifdef BASIL_EXPORTS
    #define BASIL_API __declspec(dllexport)
  #else
    #define BASIL_API __declspec(dllimport)
  #endif
#else
  #define BASIL_API __attribute__((visibility("default")))
#endif

typedef struct basil_vm*        basil_vm_t;
typedef struct basil_value*     basil_val_t;
typedef struct basil_string*    basil_str_t;
typedef struct basil_buffer*    basil_buf_t;

/* Error codes */
typedef enum {
  BASIL_OK = 0,
  BASIL_ERR_OOM,
  BASIL_ERR_TYPE,
  BASIL_ERR_PANIC
} basil_err_t;

/* Value creation */
BASIL_API basil_val_t basil_make_int(basil_vm_t vm, int64_t v);
BASIL_API basil_val_t basil_make_bool(basil_vm_t vm, int v);
BASIL_API basil_val_t basil_make_string(basil_vm_t vm, const char* s, size_t n);
BASIL_API basil_val_t basil_make_bytes(basil_vm_t vm, const uint8_t* p, size_t n);

/* Introspection */
BASIL_API int64_t     basil_as_int(basil_vm_t vm, basil_val_t v, basil_err_t* err);
BASIL_API const char* basil_as_string(basil_vm_t vm, basil_val_t v, size_t* n, basil_err_t* err);

/* Foreign functions register */
typedef basil_err_t (*basil_cfunc)(basil_vm_t vm, int argc, basil_val_t* argv, basil_val_t* ret);

BASIL_API basil_err_t basil_register(basil_vm_t vm,
                                     const char* module,
                                     const char* name,
                                     basil_cfunc fn);

/* Memory & lifetime */
BASIL_API void basil_retain(basil_vm_t vm, basil_val_t v);
BASIL_API void basil_release(basil_vm_t vm, basil_val_t v);

#endif /* BASIL_H */
```

### 4.2 🌱 A native module (C) exposing `crc32(data: bytes) -> int`

```c
#include "basil.h"
#include <zlib.h>

static basil_err_t fn_crc32(basil_vm_t vm, int argc, basil_val_t* argv, basil_val_t* ret) {
    if (argc != 1) return BASIL_ERR_TYPE;
    size_t n=0;
    basil_err_t err=BASIL_OK;
    const char* p = (const char*) basil_as_string(vm, argv[0], &n, &err);
    if (err) return err;
    uLong c = crc32(0L, Z_NULL, 0);
    c = crc32(c, (const Bytef*)p, (uInt)n);
    *ret = basil_make_int(vm, (int64_t)c);
    return BASIL_OK;
}

BASIL_API basil_err_t basil_init(basil_vm_t vm) {
    return basil_register(vm, "hash", "crc32", fn_crc32);
}
```

### 4.3 🌱 Using it from Basil

```basil
IMPORT native "hash"    ' loads shared lib: hash.dll / libhash.so

LET c = hash.crc32("hello")
PRINT c
```

**Why this Application Binary Interface (ABI) works well**

* 🌱 **Single stable C entrypoint** (`basil_init`) registering functions.
* 🌱 **Handles/retain/release** avoid raw pointers crossing languages.
* 🌱 No GC surprise: the host controls object lifetimes; extensions can hold refs with `retain`.

# 5) 🌱 Node/JS & Python bridges (ecosystem leverage)

Keep these *optional* and **process-isolated** by default (fast IPC), with an in-process option for trusted code.

**Node bridge (in-process option via N-API)**

```basil
IMPORT node ' enables require()

LET crypto = node.require("crypto")
LET h = crypto.createHash("sha256").update("hi").digest("hex")
PRINT h
```

**Python bridge (default: sidecar process via HPy/limited-ABI)**

```basil
IMPORT py

LET np = py.module("numpy")
PRINT np.sqrt(9)     ' -> 3.0
```

* For production, you’d point these to a **sidecar** started by the runtime and communicate over **UDS/Named Pipes** with **CBOR**.

# 6) 🌱 Project layout & manifest

```
myapp/
  src/app.basil
  src/routes.basil
  migrations/001_init.sql
  basil.toml
```

```toml
# basil.toml
package = "myapp"
version = "0.1.0"
edition = "2026"

[dependencies]
"db/sqlite" = "1"
"web/http"  = "1"

[target."wasm32-wasi"]
capabilities = ["fs:read", "http:client"]

[plugins]
# load WASI components at runtime
"text-utils" = { path = "plugins/text_utils.wasm", world = "text-utils" }
```

# 7) 🌱 Minimal “hello-CRUD” scaffold (what `basil new web` could generate)

* **HTTP server** with middleware
* **SQLite** migration + DAO
* **.env** support
* **tests** with `basil test`
* **formatter** with `basil fmt`
* **LSP** for VS Code/JetBrains

# 8) ✨ A few syntax goodies 🤔🤔🤔:

* **Pattern matching**

```basil
MATCH res
    CASE 200..299 THEN PRINT "ok"
    CASE 404      THEN PRINT "not found"
    CASE _        THEN PRINT "err ", res
END
```

* **Pipelines for web ergonomics**

```basil
LET html = data |> renderTemplate("todos.html", _)
```

* **Gradual typing**

```basil
LET n = 0               ' dynamic
LET m AS INT = 0        ' static
```

---

## Why this combo works 🤔

* **Rust host** + **bytecode VM** for iteration, then **AOT to C** and **WASM** for reach.
* **Two stable extension paths**: **C-ABI** (speed, native libs) and **WASI** (safety, portability).
* **Bridges** to Node/Python unlock huge ecosystems without forcing you to re-implement the world.
* A **WSGI/Rack-like HTTP contract** makes it easy to build multiple servers/frameworks on one surface.

* **BASIC-like syntax** is approachable and readable, but with modern features (expressions, async/await, optional types).
* **Focus on DX**: batteries-included tooling (formatter, LSP, test runner), clear error messages, sensible defaults.
* **Web-first standard library**: HTTP client/server, JSON, SQLite, dotenv, logging, crypto.
* **Project templates** to get you started quickly.
* **Performance plan**: start with bytecode VM, add AOT to C for dev builds, then native (LLVM) for hot paths.
* **Realistic 12-month roadmap** to ship a usable 1.0.
* **Documentation & examples**: clear guides, cookbooks, API docs.
* **Ecosystem growth**: package registry, community libraries, frameworks.
* **Long-term vision**: gradual typing, algebraic data types, pattern matching, async/await, macros.
* **Performance tuning**: profiling tools, benchmarks, optimizations.
* **Security focus**: capability-based I/O, sandboxing, secure defaults.
* **Cross-platform**: Windows, macOS, Linux, WASM (WASI).
* **Embedding**: easy to embed in other applications (C API, WASM).


* **Open governance**: stable editions, semantic versioning, security model.
* **Community-driven**: RFCs for major changes, open design discussions.

 
* **Internationalization**: Unicode support, localization libraries.
* **Accessibility**: screen reader support, high-contrast themes.
* **Community engagement**: forums, chat, events, contributor recognition.
* **Sustainability**: funding models, sponsorships, grants.

 
* **Legal clarity**: clear licensing, contributor agreements, trademark policies.
* **Regular releases**: predictable cadence, changelogs, migration guides.
* **User feedback loops**: surveys, usability testing, feature requests.


* **Punny CLI** to keep things fun 🌿 (see PUNS.md)
