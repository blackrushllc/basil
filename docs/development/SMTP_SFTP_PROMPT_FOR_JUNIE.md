# Junie Pro task: Basil **NET Feature Objects – Phase 1** (SFTP + SMTP)

## Context

Basil uses feature-gated objects (`obj-*`) with a shared registry. We want two network mods:

* **NET_SFTP** for file transfer (prefer SFTP over FTP/FTPS for security).
* **MAIL_SMTP** for sending mail (simple HTML/text + optional raw MIME).

**Process rules**

* Do **not** perform git actions (no branches/PRs/commits).
* No tests required in this pass.
* **Must include runnable examples under `/examples/` and user docs.**

## Crate & features

Create a new crate: **`basil-objects-net/`** with Cargo features:

* `obj-net-sftp` → enables SFTP module
* `obj-net-smtp` → enables SMTP module
* `obj-net = ["obj-net-sftp","obj-net-smtp"]` umbrella
+ `obj-all = **everything**` umbrella


Wire via workspace deps (forward slashes) and register objects with the existing Basil object registry only when features are enabled.

## Runtime & deps (Rust)

* Reuse / create a **global Tokio runtime** (lazy init via `once_cell`) for async code.
* Prefer TLS via **rustls**; avoid OpenSSL friction.
* Suggested crates:

    * SFTP: either **`async-sftp`** (Tokio-native) or **`ssh2`** wrapped with `spawn_blocking` (choose the more stable path you’re comfortable with).
    * SMTP: **`lettre`** with `tokio1` + `rustls-tls` features; **`mailbuilder`** or `lettre::message` for basic MIME building.

## Object: `NET_SFTP`

**Constructor**

```
DIM sftp@ AS NET_SFTP(host$, user$, pass$?, keyfile$?, port%?)
```

* Auth precedence: if `keyfile$` non-empty → key auth (optionally support passphrase); else if `pass$` → password; otherwise fail with a clear message.
* Reasonable default `port% = 22` if omitted.

**Methods**

* `Connect()` → ok% (optional: auto-connect on first use)
* `Put$(local_file$, remote_path$)` → ok%
* `GetToFile(remote_path$, local_file$)` → ok%
* `List$(remote_path$)` → String[] (names, no dot-entries)
* `Mkdir(remote_path$)` → ok%
* `Rmdir(remote_path$)` → ok%
* `Delete(remote_path$)` → ok%
* `Rename(old_path$, new_path$)` → ok%

**Notes**

* Stream large files to disk; avoid loading entire file into memory.
* Normalize path separators.
* Errors → Basil exceptions with server message (redact creds).

## Object: `MAIL_SMTP`

**Constructor**

```
DIM smtp@ AS MAIL_SMTP(host$, user$?, pass$?, port%?, tls_mode$?)
```

* `tls_mode$` in { `"starttls"`, `"tls"`, `"plain"` } (default `"starttls"`).
* If `user$`/`pass$` empty → attempt unauthenticated relay (document this is rare).

**Methods**

* `SendEmail(to$, subject$, body$, from$?, is_html%?)` → ok% or message_id$ (provider-dependent)
* `SendRaw$(mime$)` → ok%
* (Optional helpers if trivial)

    * `MakeMime$(from$, to$, subject$, text_body$?, html_body$?, attach_path$?)` → mime$

**Behavior**

* Use lettre’s async transport; reuse connection if feasible.
* Errors → Basil exceptions with SMTP reply code + reason.

## Docs (must include)

Create **`docs/guides/SMTP_SFTP.md`** covering:

* What NET_SFTP and NET_SMTP do; object names and method tables.
* Security guidance:

    * Prefer **SFTP**; avoid plain FTP.
    * SMTP: prefer STARTTLS/TLS; many providers require **app passwords**.
* Configuration:

    * Env examples (optional): `BASIL_SFTP_HOST`, `BASIL_SFTP_USER`, etc.
    * Key auth for SFTP (PEM path), passphrase handling notes.
* Usage with `TRY/CATCH/FINALLY` (Basil exceptions).
* Feature flags and build examples:
  `cargo build --features obj-net` or specific ones.

## Examples (must include; runnable)

Place under `/examples/`:

1. **`net_sftp_basic.basil`**

```
REM SFTP demo
LET host$ = "sftp.example.com"
LET user$ = "demo"
LET pass$ = "apppass"
DIM sftp@ AS NET_SFTP(host$, user$, pass$, "", 22)

TRY
  PRINTLN "MKDIR: ", sftp@.Mkdir("/incoming")
  PRINTLN "PUT: ", sftp@.Put$("report.csv", "/incoming/report.csv")
  LET names$ = sftp@.List$("/incoming")
  PRINTLN "Listing:"
  FOR EACH n$ IN names$ : PRINTLN " - ", n$ : NEXT
  PRINTLN "GET: ", sftp@.GetToFile("/incoming/report.csv", "report_downloaded.csv")
  PRINTLN "RENAME: ", sftp@.Rename("/incoming/report.csv", "/incoming/report_old.csv")
  PRINTLN "DELETE: ", sftp@.Delete("/incoming/report_old.csv")
CATCH err$
  PRINTLN "SFTP error: ", err$
END TRY
```

2. **`mail_smtp_send.basil`**

```
REM SMTP demo
DIM smtp@ AS MAIL_SMTP("smtp.mailprovider.com", "me@example.com", "apppass", 587, "starttls")
TRY
  PRINTLN "Sending..."
  PRINTLN smtp@.SendEmail("you@example.com", "Hello from Basil", "<b>Hi!</b>", "me@example.com", 1)
CATCH e$
  PRINTLN "SMTP error: ", e$
END TRY
```

## Acceptance checklist

* [ ] Workspace builds with `--features obj-net` and with features off.
* [ ] Objects register only when features are enabled; `DESCRIBE sftp@` / `DESCRIBE smtp@` show members.
* [ ] Examples compile and run against real servers (with valid creds).
* [ ] `docs/guides/SMTP_SFTP.md` exists and covers setup, security, feature flags, and `TRY/CATCH` usage.
* [ ] Errors are readable (service/operation/reply); sensitive data is not logged.

