# HTTP object (HTTP/REST client)

The HTTP object provides an easy-to-script HTTP/REST client with sane defaults, header/auth helpers, timeouts, and file I/O for downloads/uploads. It uses the Tokio runtime and reqwest with the rustls TLS backend (no OpenSSL), and respects system proxy environment variables.

Enable it via Cargo features:

- obj-net-http → enables HTTP object
- obj-net → umbrella that includes obj-net-http (and other NET objects)
- obj-all → umbrella that includes everything

Examples:
- cargo build --features obj-net-http
- cargo run -p basilc --features obj-net -- run examples\http_quickstart.basil

What you get:

- Stateful defaults: BaseUrl$, headers, auth, timeout
- Core methods returning response bodies as strings: Get$/Delete$/Post$/Put$/Patch$ (+ JSON variants)
- Head$ → returns ok%
- File I/O helpers: DownloadToFile, UploadFile$ (multipart)
- Error behavior: optional RaiseForStatus% to auto-raise on 4xx/5xx; LastStatus%/LastUrl$/LastHeaders$/LastError$ always updated
- Proxies via env, compression, redirects via reqwest defaults

Constructor

- DIM http@ AS HTTP()

Defaults on creation:

- TimeoutMs% = 30000 (30s)
- RaiseForStatus% = 0
- BaseUrl$ empty, no auth, no headers

Properties

- BaseUrl$            RW — prefix for relative URLs
- TimeoutMs%          RW — default per-request timeout (ms)
- RaiseForStatus%     RW — 1 to raise exceptions automatically on 4xx/5xx
- LastStatus%         RO — status code of last response (0 if none)
- LastUrl$            RO — final URL of the last response
- LastHeaders$        RO — JSON map of response headers
- LastError$          RO — last error message (empty on success)

Header/auth/query helpers

- SetHeader(name$, value$) → ok%
- ClearHeaders() → ok%
- SetBearer$(token$) → ok%            (sets Authorization: Bearer)
- SetBasicAuth$(user$, pass$) → ok%   (sets Authorization: Basic)
- SetQueryParam(name$, value$) → ok%  (default query params added to every request)
- ClearQueryParams() → ok%

Core request methods

All body-returning methods yield the body as a string. For binary responses, prefer DownloadToFile.

- Get$(url$, timeout_ms%?) → body$
- Delete$(url$, timeout_ms%?) → body$
- Head$(url$, timeout_ms%?) → ok%
- Post$(url$, body$, timeout_ms%?) → body$
- Put$(url$, body$, timeout_ms%?) → body$
- Patch$(url$, body$, timeout_ms%?) → body$

JSON convenience

- PostJson$(url$, json$, timeout_ms%?) → body$    (Content-Type: application/json)
- PutJson$(url$, json$, timeout_ms%?) → body$
- PatchJson$(url$, json$, timeout_ms%?) → body$

File I/O helpers

- DownloadToFile(url$, out_path$, timeout_ms%?) → ok%        (streams to disk; creates parent dirs)
- UploadFile$(url$, file_path$, field_name$?, content_type$?) → body$
  - Sends multipart/form-data with one file part
  - Default field name is "file"
  - If content_type$ empty, a simple guess is made from the file extension (png, jpg, gif, json, txt, csv, xml, pdf, etc.) or omitted

Behavior and error handling

- Requests respect BaseUrl$, default headers, default query params, TimeoutMs%, and auth.
- If RaiseForStatus% = 1, 4xx/5xx raise exceptions with messages like:
  - HTTP 404 Not Found at <url> — <body snippet>
- On network/TLS errors, raises:
  - HTTP RequestFailed: <reason> at <url>
- Always updates LastStatus%, LastUrl$, LastHeaders$, LastError$.
- Compression and redirects handled by reqwest defaults; up to 10 redirects.
- Proxies: standard env vars HTTP_PROXY/HTTPS_PROXY/NO_PROXY are supported by reqwest automatically.

Notes on text vs binary

- Methods returning strings attempt UTF-8 decoding (lossy if needed). For binary payloads (images, ZIPs, etc.), use DownloadToFile.

Quickstart

```
REM Basic GET + JSON POST
DIM http@ AS HTTP()
PRINTLN "GET:"
PRINTLN http@.Get$("https://httpbin.org/get")

PRINTLN "\nPOST JSON:"
PRINTLN http@.PostJson$("https://httpbin.org/post", "{""hello"":""basil""}")
```

Base URL, headers, auth, error handling

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

File download and upload

```
REM Download to file and upload a file (multipart)
DIM http@ AS HTTP()

PRINTLN "Downloading…"
PRINTLN http@.DownloadToFile("https://httpbin.org/image/png", "out/test.png")

PRINTLN "Uploading…"
PRINTLN http@.UploadFile$("https://httpbin.org/post", "out/test.png", "file", "image/png")
```

Error handling and per-call timeouts

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

Implementation details

- Tokio runtime: shared, lazily initialized (multi-threaded) runtime reused by NET objects
- HTTP client: reqwest with rustls TLS backend; JSON, gzip, brotli, deflate, stream, multipart features enabled
- Redirects: default policy (up to 10)
- Proxies: env-based
- We do not log or echo auth credentials; error messages avoid including Authorization headers
