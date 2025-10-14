# Junie Pro task: Basil **HTTP / REST Client** (GET/POST/etc., headers, query, timeouts, file I/O)

## Context

Basil uses feature-gated objects registered at runtime (e.g., obj-aws, obj-net-sftp, obj-net-smtp). We want a **general HTTP client** that’s easy to script: plain text/JSON APIs, file download/upload, auth, and sane defaults. Errors must map to Basil exceptions so users can `TRY/CATCH`.

**Process rules**

* Do **not** perform git actions (no branches/PRs/commits).
* **Must include runnable examples under `/examples/` and user docs** (this is part of acceptance).
* No formal tests required in this pass.

## Crate & features

Use the existing **`basil-objects-net/`** crate and add an HTTP module with feature flags:

* `obj-net-http` → enables HTTP object
* umbrella `obj-net` should include `obj-net-http` alongside any existing `obj-net-sftp`, `obj-net-smtp`.
+ `obj-all = **everything**` umbrella

Wire via workspace deps with forward slashes; register the object with the global registry only when the feature is enabled.

## Runtime & dependencies (Rust)

* Use **Tokio** runtime (reuse the same global, lazily initialized runtime you set up for NET/AWS).
* HTTP client: **`reqwest`** with **rustls** TLS backend (disable native-tls to avoid OpenSSL issues).

    * Enable features: `json`, `gzip`, `brotli`, `deflate`, `stream` (if trivial), `rustls-tls`.
* Optional: `mime` for content types, `multipart` feature in reqwest for file upload.

## Object: `HTTP`

A stateful client where you can set defaults (base URL, headers, auth, timeout) and then fire requests.

### Constructor

```
DIM http@ AS HTTP()
```

On creation, the client has:

* default timeout = 30_000 ms
* empty default headers
* no base URL / auth

### Properties (get/set)

* `BaseUrl$`              (prefix for relative URLs; optional)
* `TimeoutMs%`            (per-request default timeout; can be overridden per call)
* `RaiseForStatus%`       (0/1; if 1, 4xx/5xx raise exceptions automatically; default 0)
* `LastStatus%`           (read-only; status code of last response or 0 if none)
* `LastUrl$`              (read-only; fully resolved URL of last request)
* `LastHeaders$`          (read-only; JSON string of response headers)
* `LastError$`            (read-only; last request error message if any)

### Header & auth helpers

* `SetHeader(name$, value$)` → ok%
* `ClearHeaders()` → ok%
* `SetBearer$(token$)` → ok%
* `SetBasicAuth$(user$, pass$)` → ok%
* `SetQueryParam(name$, value$)` → ok%   *(applies to subsequent requests; useful with BaseUrl)*
* `ClearQueryParams()` → ok%

### Core request methods

All body-returning methods yield the **response body as a string** (binary-safe if reqwest returns bytes we can losslessly pass; otherwise document file APIs below).

* `Get$(url$)` → body$
* `Delete$(url$)` → body$
* `Head$(url$)` → ok% (also sets LastStatus/LastHeaders)
* `Post$(url$, body$)` → body$                    (raw body; caller sets headers)
* `Put$(url$, body$)` → body$
* `Patch$(url$, body$)` → body$

**JSON convenience**

* `PostJson$(url$, json$)` → body$                 (sets `Content-Type: application/json`)
* `PutJson$(url$, json$)` → body$
* `PatchJson$(url$, json$)` → body$

**Query/timeout overrides (optional convenience)**

* Accept an optional trailing timeout in ms for any call if trivial:

    * e.g., `Get$(url$, timeout_ms%)`. If supplied, it overrides `TimeoutMs%` for that call.

### File I/O helpers

* `DownloadToFile(url$, out_path$)` → ok%        (streams to disk; creates dirs if needed)
* `UploadFile$(url$, file_path$, field_name$?, content_type$?)` → body$
  *(multipart/form-data with one file part; default `field_name="file"`; if `content_type$` empty, guess from extension or omit)*

### Behavior & error handling

* Requests respect: `BaseUrl$`, default headers, default query params, default timeout, and auth.
* If `RaiseForStatus% = 1`, 4xx/5xx map to Basil exceptions with a readable message:
  `HTTP 404 Not Found at <url>` and include response body snippet in the exception message if short (truncate to ~512 chars).
* On **network/TLS** errors, raise: `HTTP RequestFailed: <reason> at <url>`.
* Always update `LastStatus%`, `LastUrl$`, `LastHeaders$` (and `LastError$` on failures).

### Notes

* Proxies: honor standard env vars (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`) automatically via reqwest (document this).
* Compression: transparently handled by reqwest (accept-encoding).
* Redirects: allow reqwest defaults (follow up to 10). If trivial, add property `FollowRedirects%` later; not required now.

## Docs (must include)

Create **`docs/guides/HTTP.md`** explaining for end users:

* What `HTTP` provides; method table with short one-liners.
* Quickstart with `GET`/`POST JSON` and base URL/headers.
* Auth helpers (Bearer/Basic) and per-request vs default headers.
* Timeouts, `RaiseForStatus%`, and error handling with `TRY/CATCH`.
* File download/upload examples and notes about binary vs text.
* Proxies via env, TLS via rustls (no extra system deps).
* Feature flags and build example:

    * `cargo build --features obj-net-http` or `--features obj-net`

## Examples (must include; runnable)

Place under `/examples/`:

1. **`http_quickstart.basil`**

```
REM Basic GET + JSON POST
DIM http@ AS HTTP()
PRINTLN "GET:"
PRINTLN http@.Get$("https://httpbin.org/get")

PRINTLN "\nPOST JSON:"
PRINTLN http@.PostJson$("https://httpbin.org/post", "{""hello"":""basil""}")
```

2. **`http_headers_auth.basil`**

```
REM Default headers, bearer auth, query params
DIM http@ AS HTTP()
http@.SetHeader("X-Client", "Basil")
http@.SetBearer$("test-token-123")
http@.SetQueryParam("lang", "en")
http@.TimeoutMs% = 10000
http@.RaiseForStatus% = 1

TRY
  PRINTLN http@.Get$("https://httpbin.org/anything/path")
  PRINTLN "Status:", http@.LastStatus%, " URL:", http@.LastUrl$
  PRINTLN "Resp headers:", http@.LastHeaders$
CATCH e$
  PRINTLN "HTTP error: ", e$
END TRY
```

3. **`http_download_upload.basil`**

```
REM Download to file and upload a file (multipart)
DIM http@ AS HTTP()

PRINTLN "Downloading…"
PRINTLN http@.DownloadToFile("https://httpbin.org/image/png", "out/test.png")

PRINTLN "Uploading…"
PRINTLN http@.UploadFile$("https://httpbin.org/post", "out/test.png", "file", "image/png")
```

4. **`http_error_handling.basil`**

```
REM Demonstrate RaiseForStatus and timeout override
DIM http@ AS HTTP()
http@.RaiseForStatus% = 1

TRY
  PRINTLN http@.Get$("https://httpbin.org/status/404")
CATCH e$
  PRINTLN "Caught expected 404: ", e$
END TRY

TRY
  PRINTLN http@.Get$("https://httpbin.org/delay/3", 1000)  REM 1s timeout override
CATCH e$
  PRINTLN "Caught timeout: ", e$
END TRY
```

## Implementation notes (guidance)

* Build a reusable `reqwest::Client` per `HTTP` instance, rebuilt when key defaults change (timeout, auth, default headers). Or track defaults and apply per request—choose the simpler path initially.
* Resolve URLs by combining `BaseUrl$` (if set) + provided `url$` (treat absolute URLs as-is).
* Convert bodies to/ from UTF-8 where reasonable; for binary responses, prefer `DownloadToFile`.
* Marshal response headers into a small JSON map for `LastHeaders$`.
* **Do not** log auth tokens or passwords. Redact in error strings.

## Wiring

* Add module `http.rs` inside `basil-objects-net/` behind `#[cfg(feature = "obj-net-http")]`.
* Register the object factory into the global registry under the type name `HTTP`.
* Update workspace `Cargo.toml` to include the feature flag and reqwest deps (rustls).

## Acceptance checklist

* [ ] Workspace builds with `--features obj-net-http` and with features off.
* [ ] `DESCRIBE http@` lists properties & methods.
* [ ] The four example programs exist and run (with internet access) and demonstrate headers, auth, query params, file I/O, timeouts, and error handling.
* [ ] `docs/guides/HTTP.md` exists and matches the implemented surface.
* [ ] Errors map to Basil exceptions with helpful messages; `LastStatus%`, `LastUrl$`, `LastHeaders$`, `LastError$` behave as described.

