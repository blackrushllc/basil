love this idea. here’s a crisp, end-to-end plan for an **obj-gearman** Feature Object that gives Basil first-class client + worker support for Gearman clusters, with clean Basil APIs, async Rust under the hood, and nice DX.

# Goals

* Let Basil programs **submit jobs** to Gearman and **run workers** that handle jobs.
* Keep the Basil surface tiny and BASIC-ish, but powerful: sync results, background jobs, streaming, status, retries, and graceful shutdown.
* No runtime deps beyond your Gearman server(s); pure TCP protocol client in Rust (or feature-gated libgearman FFI if we want).

---

# Basil API (user-facing)

### config

* default config file: `.basil-gearman.toml` (also read env overrides)

```toml
# .basil-gearman.toml
servers   = ["127.0.0.1:4730", "10.0.0.12:4730"]
client_id = "basil-app-%HOSTNAME%"
timeout_ms = 15000
retries   = 2
keepalive = true
```

env overrides (example): `GEARMAN_SERVERS`, `GEARMAN_CLIENT_ID`, `GEARMAN_TIMEOUT_MS`, `GEARMAN_RETRIES`.

### constants

* `GEARMAN_OK% = 0`, `GEARMAN_ERR% = -1`, plus specific error codes (timeout, io, protocol, not_found, worker_failed, etc.).

### client functions

* `GEARMAN.SUBMIT$(function$, payload$ [, unique$])` → returns result payload as string (binary-safe via Basil strings).
* `GEARMAN.SUBMIT_BG%(function$, payload$ [, unique$])` → returns 1 on accepted; job handle in `GEARMAN.LAST_HANDLE$`.
* `GEARMAN.STATUS$(handle$)` → returns JSON string like `{"known":true,"running":false,"numerator":0,"denominator":0}`.
* `GEARMAN.CANCEL%(handle$)` → attempt to cancel (best-effort).
* `GEARMAN.ECHO$(data$)` → round-trip ping through server(s).
* `GEARMAN.SERVERS$()` → returns the active server list (after shuffles / failover).

### worker API

* `GEARMAN.WORKER_START%(id$ [, concurrency%])` → start worker loop in background threads (Tokio tasks); returns 1 when active.
* `GEARMAN.WORKER_ADD%(function$, handler$)` → register a BASIC subroutine name as a handler for that function.
* `GEARMAN.WORKER_STOP%()` → request graceful stop; returns 1 when the loop has quiesced.
* `GEARMAN.WORKER_ALIVE%()` → 1 if running.
* `GEARMAN.WORKER_SET_STATUS%(numerator%, denominator%)` → progress updates.
* `GEARMAN.WORKER_YIELD$()` → optional: pop next job and return its JSON envelope for manual handling (advanced).

### events / streaming (optional but nice)

* `GEARMAN.SUBMIT_STREAM%(function$, payload$, callback$)` → invoke `callback$` on each WORK_DATA / WORK_WARNING packet; return code when finished; final result available via `GEARMAN.LAST_RESULT$`.

### errors

* All funcs set `GEARMAN.ERR$` with a short message and `GEARMAN.ERRCODE%`. On fatal, raise `GEARMAN_ERROR`.

---

# Example: client (sync)

```basil
INCLUDE "obj-gearman"

PRINT "Submitting image-resize..."
res$ = GEARMAN.SUBMIT$("img.resize", "{""url"":""https://.../cat.jpg"",""w"":640}")
IF GEARMAN.ERRCODE% <> GEARMAN_OK% THEN
  PRINT "Gearman error: "; GEARMAN.ERR$
  STOP
END IF
PRINT "Result: "; LEFT$(res$, 80); "..."
```

# Example: background job + status polling

```basil
ok% = GEARMAN.SUBMIT_BG$("report.build", "{""user_id"":42}", "user-42-weekly")
IF ok% = 0 THEN
  PRINT "Submit failed: "; GEARMAN.ERR$
  STOP
END IF
handle$ = GEARMAN.LAST_HANDLE$
DO
  SLEEP 500
  st$ = GEARMAN.STATUS$(handle$)
  PRINT "Status: "; st$
LOOP UNTIL INSTR(st$, """running"":false") > 0
```

# Example: worker with BASIC handler

```basil
INCLUDE "obj-gearman"

SUB do_resize(job$) ' job$ is raw payload (e.g., JSON)
  ' parse JSON (assume JSON.PARSE$ exists) and do work...
  GEARMAN.WORKER_SET_STATUS%(1, 3)
  ' ... process ...
  GEARMAN.WORKER_SET_STATUS%(2, 3)
  ' return a string payload:
  RETURN "{""ok"":true,""file"":""/out/cat-640.jpg""}"
END SUB

ok% = GEARMAN.WORKER_ADD%("img.resize", "do_resize")
ok% = GEARMAN.WORKER_START%("basil-worker-#{HOST$()}", 4)

' Wait for Ctrl-C or some app condition...
DO : SLEEP 1000 : LOOP
```

---

# Rust design (inside `obj-gearman` crate/feature)

**Crate shape**

```
basil/
  crates/
    obj-gearman/
      src/
        lib.rs
        client.rs
        worker.rs
        protocol.rs      # minimal Gearman protocol (REQ/RES packets)
        config.rs
      Cargo.toml
```

**Runtime model**

* Async (Tokio). One `GearmanClientPool` with N connections → round-robin; auto-reconnect; jittered backoff.
* Workers: each function gets an async task group pulling from `GRAB_JOB` / `PRE_SLEEP` loops; use `SET_CLIENT_ID`, `CAN_DO`, `SET_CLIENT_ID`, `SET_STATUS`, `WORK_DATA`, `WORK_WARNING`, `WORK_COMPLETE`, `WORK_FAIL`, etc.
* Protocol: implement the **binary** Gearman protocol (REQ/RES headers, magic, type codes) to avoid libgearman dependency. Keep it small and tested.
* Optional feature `ffi-libgearman` to bind libgearman if users prefer system lib.

**Key types**

```rust
pub struct GearmanConfig { servers: Vec<SocketAddr>, client_id: String, timeout: Duration, retries: u32, keepalive: bool }

pub struct GearmanClientPool { /* connections, metrics, rng */ }
pub struct JobHandle(String);
pub enum SubmitMode { Foreground, Background, Stream }

pub struct WorkerRuntime { /* function map, tasks, stop signal */ }
type BasilHandler = Arc<dyn Fn(String) -> BoxFuture<'static, Result<String, BasilError>> + Send + Sync>;
```

**Basil FFI surface**

* Each Basil function wraps a call into the async runtime via the VM’s async bridge (we already do this for other net modules).
* Map `Result<T, GearmanError>` → Basil return or raise `BasilException::Feature("GEARMAN_ERROR", msg)` with `ERR$`/`ERRCODE%` side-channels set.

**Progress + streaming**

* For `SUBMIT_STREAM%`, register a per-request callback handle stored in a VM registry (small `u64` id). Worker `WORK_DATA`/`WORK_WARNING` frames re-enter the VM to call the BASIC callback with chunk strings.

**Graceful shutdown**

* `WORKER_STOP%()` flips an atomic; tasks finish current job, call `PRE_SLEEP`, and join. Expose `WORKER_ALIVE%()` for health.

**Retries & backoff**

* Client ops retry on connect/reset/timeouts (idempotent BG submits, careful with FG). BG submit returns first success; FG may fan-out across servers according to policy (configurable).

---

# Protocol coverage (MVP)

* Client: `SUBMIT_JOB`, `SUBMIT_JOB_BG`, `SUBMIT_JOB_HIGH[_BG]`, `ECHO_REQ`, `GET_STATUS`
* Worker: `SET_CLIENT_ID`, `CAN_DO`, `GRAB_JOB`, `WORK_DATA`, `WORK_WARNING`, `WORK_STATUS`, `WORK_COMPLETE`, `WORK_FAIL`, `PRE_SLEEP`
* Authentication: (Gearman usually none). If your infra terminates TLS elsewhere, we’re fine. Optionally support `stunnel`/TLS tunnel if needed.

---

# Testing & examples

### smoke tests (Basil)

1. **Ping**

```basil
PRINT GEARMAN.ECHO$("hello")
```

2. **Round-trip**

* Start a minimal Basil worker for `echo.upper` and a client submit. Assert `"HELLO"`.

3. **Background + status**

* Submit BG job; poll `STATUS$` until done; assert fields.

### Rust unit tests

* Protocol encode/decode tests (goldens).
* Connection loss + reconnect.
* Worker loop `PRE_SLEEP` wakeups.

### docker dev harness

* `docker compose` with `gearmand:latest` and two replicas to test multi-server lists.
* Makefile tasks: `make gearman-up`, `make integ`.

---

# DX polish

### feature flag

* Build with:
  `cargo run -p basilc --features "obj-gearman"`

### docs

* `/docs/OBJ_GEARMAN.md`: quickstart, config, examples, error table, operational notes.

### errors table (snippet)

* `1 TIMEOUT`
* `2 IO`
* `3 PROTOCOL`
* `4 NOT_FOUND`
* `5 WORKER_FAILED`
* `6 CANCEL_UNSUPPORTED`
* … map from Rust enums.

---

# Performance notes

* Use **length-prefixed zero-copy buffers** for payloads; avoid UTF-8 coercion (Basil strings can be binary); only JSON examples are for convenience.
* Pipeline requests where allowed; keep one in-flight per connection for FG, multiple for BG if protocol-safe.
* Expose `concurrency%` in `WORKER_START%`.

---

# Roadmap (beyond MVP)

* Priority queues: `SUBMIT_JOB_HIGH[_BG]`.
* Namespaces/queues per environment (`function$ = "app1:img.resize"`).
* Worker heartbeats & metrics (optionally push stats to Basil’s `obj-metrics`).
* TLS (via proxy or feature-gated native TLS).
* Backpressure & bounded queues for worker load-shedding.

---

# “Junie prompt” (paste-ready)

Create a new Basil Feature Object **obj-gearman** that adds Gearman client and worker support.

Requirements:

* Rust crate `obj-gearman` with modules: `config.rs`, `protocol.rs`, `client.rs`, `worker.rs`, `lib.rs`.
* Implement the Gearman **binary protocol** (no external C deps). Provide a feature flag `ffi-libgearman` that swaps in FFI calls if enabled.
* Async with Tokio; connection pool with retries/backoff; multi-server failover; optional TCP keepalive.
* Basil surface API:

    * Client: `GEARMAN.SUBMIT$`, `GEARMAN.SUBMIT_BG%`, `GEARMAN.STATUS$`, `GEARMAN.CANCEL%`, `GEARMAN.ECHO$`, `GEARMAN.SERVERS$`, `GEARMAN.SUBMIT_STREAM%` (streaming callback).
    * Worker: `GEARMAN.WORKER_ADD%`, `GEARMAN.WORKER_START%`, `GEARMAN.WORKER_STOP%`, `GEARMAN.WORKER_ALIVE%`, `GEARMAN.WORKER_SET_STATUS%`, plus advanced `GEARMAN.WORKER_YIELD$`.
* Map Rust errors to Basil: set `GEARMAN.ERR$`, `GEARMAN.ERRCODE%`, and raise `GEARMAN_ERROR` exception on fatal.
* Config:

    * Load `.basil-gearman.toml` with keys `servers`, `client_id`, `timeout_ms`, `retries`, `keepalive`.
    * Env overrides `GEARMAN_SERVERS`, `GEARMAN_CLIENT_ID`, `GEARMAN_TIMEOUT_MS`, `GEARMAN_RETRIES`.
* Provide examples under `/examples/gearman/`:

    * `client_sync.basil`, `client_bg_status.basil`, `worker_resize.basil`, `stream_demo.basil`.
* Add `docker-compose.yml` for local `gearmand` testing; Makefile targets `gearman-up`, `gearman-down`, `integ`.
* Tests:

    * Unit tests for protocol encode/decode.
    * Integration tests for submit/echo/status and worker complete.
* Documentation: `/docs/OBJ_GEARMAN.md` with quickstart, API reference, error codes, and operational tips.

Acceptance:

* `cargo run -p basilc --features obj-gearman -- run examples/gearman/client_sync.basil` works against `docker compose` gearmand.
* Worker demo processes a job and returns a string; status polling reflects progress.

---

If you want, I can also draft the `protocol.rs` packet structs + a minimal encoder/decoder to kickstart Junie’s implementation.
