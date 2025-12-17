# Basil NET objects: NET_SFTP and MAIL_SMTP (Phase 1)

This guide introduces two network feature objects for Basil:

- NET_SFTP — secure file transfers over SSH (SFTP)
- MAIL_SMTP — sending email via SMTP (STARTTLS/TLS/plain)

Both objects are feature-gated and only available when enabled at build time.

Contents
- What they do, names, and methods
- Security guidance
- Configuration examples (env vars)
- TRY/CATCH/FINALLY usage (exceptions)
- Feature flags and build examples

Object: NET_SFTP
Type name: NET_SFTP
Constructor
  DIM sftp@ AS NET_SFTP(host$, user$, pass$?, keyfile$?, port%?)
Notes
- Authentication precedence: if keyfile$ is non-empty, key-based auth is used (pass$ can be the key passphrase). Else if pass$ is set, password auth is used. Otherwise, construction succeeds but operations will fail with a clear message when connecting.
- Default port is 22 when omitted.
- Methods return Int ok% (1 for success) or String[] for listings. Errors raise Basil exceptions with server messages (credentials are not included).
- Paths are normalized to forward slashes for remote operations.
Methods
- Connect() → ok%
- Put$(local_file$, remote_path$) → ok%
- GetToFile(remote_path$, local_file$) → ok%
- List$(remote_path$) → String[] (names only, no . or ..)
- Mkdir(remote_path$) → ok%
- Rmdir(remote_path$) → ok%
- Delete(remote_path$) → ok%
- Rename(old_path$, new_path$) → ok%
Example
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

Object: MAIL_SMTP
Type name: MAIL_SMTP
Constructor
  DIM smtp@ AS MAIL_SMTP(host$, user$?, pass$?, port%?, tls_mode$?)
- tls_mode$ is one of: "starttls" (default), "tls", "plain".
- If user$/pass$ are empty, an unauthenticated relay will be attempted (many providers do not permit this; use app passwords).
Methods
- SendEmail(to$, subject$, body$, from$?, is_html%?) → ok% or message_id$
- SendRaw$(mime$) → ok% or message_id$
- MakeMime$(from$, to$, subject$, text_body$?, html_body$?, attach_path$?) → mime$
Notes
- Uses lettre’s async transport with rustls. A global Tokio runtime is lazily initialized and reused.
- The transport is cached per object instance and reused.
- Errors become Basil exceptions including SMTP reply text (code/reason); sensitive data is not logged.
Example
  REM SMTP demo
  DIM smtp@ AS MAIL_SMTP("smtp.mailprovider.com", "me@example.com", "apppass", 587, "starttls")
  TRY
    PRINTLN "Sending..."
    PRINTLN smtp@.SendEmail("you@example.com", "Hello from Basil", "<b>Hi!</b>", "me@example.com", 1)
  CATCH e$
    PRINTLN "SMTP error: ", e$
  END TRY

Security guidance
- Prefer SFTP; avoid plain FTP.
- For SMTP, prefer STARTTLS or TLS. Many providers require app passwords or OAuth; app passwords are usually the simplest.
- Store credentials securely (e.g., environment variables) and avoid printing them.

Configuration examples (environment variables)
- BASIL_SFTP_HOST, BASIL_SFTP_USER, BASIL_SFTP_PASS, BASIL_SFTP_KEY, BASIL_SFTP_PORT
- BASIL_SMTP_HOST, BASIL_SMTP_USER, BASIL_SMTP_PASS, BASIL_SMTP_PORT, BASIL_SMTP_TLS

SFTP key auth
- Provide a PEM private key path in keyfile$.
- If the key is encrypted, pass$ is used as the passphrase.

Exceptions and TRY/CATCH/FINALLY
- All operations may raise exceptions; wrap in TRY/CATCH. Example above shows error capture.

Feature flags and builds
- Enable everything in this crate: obj-net (enables obj-net-sftp and obj-net-smtp)
- Build examples:
  - cargo build -p basilc --features obj-net
  - cargo run -q -p basilc --features obj-net -- run examples\mail_smtp_send.basil
  - cargo run -q -p basilc --features obj-net -- run examples\net_sftp_basic.basil

Notes
- These examples require real servers and valid credentials to succeed.
