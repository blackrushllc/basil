# ChatGPT's original ideas for Mods

(We already have a few of these)

Love this question. Here are **hit-maker mods** that give Basil huge “I can ship something tonight” energy without bloating the core. I grouped them by what devs do most.

# Top 12 “people actually use this” mods

1. **HTTP / REST client** (`obj-http`)

    * GET/POST, headers, JSON helpers, retries/timeouts.
    * Bonus: `WebSocket` connect/send/recv.
    * *Why:* glue everything—APIs, webhooks, price feeds.
    * *Snippet:* `PRINT HTTP().Get$("https://api.example.com/ping")`

2. **SQLite** (`obj-sqlite`)

    * File DB, `Execute`, `Query$()` → rows as JSON/String[].
    * *Why:* zero-install storage for scripts and CLIs.

3. **CSV / Excel I/O** (`obj-csv`, `obj-excel`)

    * Read/write CSV; basic XLSX write (tables, formats).
    * *Why:* 90% of data interchange at work.

4. **PDF (compose)** (`obj-pdf`)

    * Generate simple PDFs (text, images, tables).
    * *Why:* invoices, reports, labels without a server.

5. **Image utils** (`obj-image`)

    * Load/save PNG/JPEG, resize/crop, annotate text, thumbnails, QR/barcodes.
    * *Why:* dashboards, receipts, badges.

6. **Scheduler / Cron** (`obj-cron`)

    * `EVERY "5m" DO …`, `CRON "*/10 * * * *"` callbacks (in-process).
    * *Why:* automation without external cron.

7. **Filesystem & ZIP pro** (`obj-fs`, complement to your zip)

    * Safe FS ops (glob, copy trees, tempdirs, hashing), robust ZIP (password later).
    * *Why:* file processing pipelines.

8. **Regex & Text** (`obj-regex`, `obj-text`)

    * Find/replace, capture groups; slugify, wrap, pad, case, transliterate.
    * *Why:* data cleaning, logs, ETL.

9. **Logging & Metrics** (`obj-log`)

    * Levels, rotating files, JSON logs; counters/gauges → stdout or file.
    * *Why:* makes scripts productionable.

10. **CLI & Process** (`obj-proc`)

* Spawn processes, capture output, exit codes, env; parse CLI args.
* *Why:* orchestrate other tools safely.

11. **Chat & Notifications** (`obj-slack`, `obj-discord`, `obj-twilio`)

* Post messages, upload files, send SMS.
* *Why:* “tell me when it’s done” is the killer feature.

12. **Hardware / IoT (Pi-friendly)** (`obj-gpio`, `obj-serial`)

* GPIO read/write, I²C/SPI later; serial read/write.
* *Why:* kiosks, makers, labs.

---

# “Nice extras” that punch above their weight

* **HTML → Text / Scrape** (`obj-html`): tidy, select with CSS/XPath; light scrape (no headless browser).
* **Markdown → HTML/PDF** (`obj-markdown`): static reports from data.
* **Time & TZ** (`obj-time`): parse/format with time zones, durations, business day math.
* **UUID & Hashes** (`obj-id`): `UUID$()`, `SHA256$()`, `HMAC$()`.
* **Geo & Maps lite** (`obj-geo`): GeoIP (offline DB optional), geocode via API, distance calc.
* **TTS/STT local** (`obj-voice`): offline speech (e.g., Vosk for STT), or Polly later (you’ve got AWS).
* **Mini HTTP server** (`obj-serve`): serve a folder or simple webhook endpoint for local integrations.

---

# How to ship them smoothly (same playbook you’re using)

* **Feature-gate** each (`obj-http`, `obj-sqlite`, …) so users opt-in.
* **One-page docs + examples** per mod; e.g., `/examples/http_quickstart.basil`, `/examples/sqlite_basic.basil`.
* **Keep surfaces tiny**: 6–10 “do the thing” methods each.
* **Return types**: strings, ints, floats, String[]; JSON strings for structured data.
* **Errors** → Basil exceptions, so `TRY/CATCH` just works.

---

# Quick starter shapes (high level)

* **HTTP**

    * `Get$(url$)`, `Post$(url$, json$|form$)`, `SetHeader(name$, value$)`, `TimeoutMs%`, `Retry%`.
* **SQLite**

    * `Open(file$)`, `Execute(sql$)`, `QueryRows$(sql$)` → JSON array, or `QueryTable$()` → CSV/array.
* **CSV**

    * `Read$(file$)` → String[] of lines or JSON rows; `Write(file$, rows_json$)`.
* **PDF**

    * `Begin(file$)`, `Text(x%, y%, s$)`, `Image(x%, y%, path$)`, `Table(json$)`, `End()`.
* **Image**

    * `Open(path$)`, `Resize(w%, h%)`, `Annotate(x%, y%, s$)`, `Save(out$)`, `QRToFile$(text$, out$)`.
* **Cron**

    * `Every$(spec$)` returns a handle; `Start()`, `Stop()`. (In interpreter, it’s often enough to block/run loop.)
* **Regex**

    * `IsMatch$(s$, pat$)`, `FindAll$(s$, pat$)` → String[], `Replace$(s$, pat$, repl$)`.
* **Log**

    * `SetLevel$("info")`, `ToFile(path$)`, `Info$(s$)`, `Error$(s$)`.
* **Proc**

    * `Run$(cmd$, args$[])` → exit% and stdout/stderr; `EnvSet$(k$, v$)`.

---



If later you want WebSockets, we can add a NET_WS object (connect/send/recv/close) behind obj-net-ws to the HTTPS/REST 
task, but this prompt keeps Phase 1 focused and shippable.