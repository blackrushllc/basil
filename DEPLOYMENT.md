There are three practical ways to sit Basil behind Apache, from easiest to most “production-y”:

1. the “it just works today” route (classic CGI)

Yesss! 🎉 So glad it’s all wired up and humming on both CLI and Apache CGI!!!

Basil can now tell the difference between running in CLI mode vs web mode, and you can write Basil scripts that respond 
to HTTP requests or run as normal CLI programs.

# Now we have it working as CGI, we can evolve to:


2. the “faster without rewriting Apache” route (FastCGI)
3. the “treat Basil like PHP-FPM or a microservice” route (reverse proxy to a long-running Basil server)



Now here's a tiny “polish” checklist:

* Add a `--cgi` flag (or `BASIL_FORCE_MODE=cgi`) so you can force CGI mode in tests without setting multiple env vars.
* Forward request headers: you already pass `HTTP_*`; keep that—super handy for auth/cookies later.
* Guardrails: cap body size (`CONTENT_LENGTH`) and add an execution timeout in the shim (e.g., kill child after 3–5s on prod).
* Better errors in prod: write detailed traces to stderr (Apache error log), show friendly 500 pages to users.
* File mapping: keep the `RewriteCond %{HANDLER} =basil-script` trick—rock solid.

For speed & state, two upgrade paths:

1. **FastCGI** (drop-in speedup, PHP-FPM-style)

    * Run a Basil FastCGI worker pool; point Apache via `mod_fcgid` or `proxy_fcgi`.
    * You keep the same `.basil` mapping; no fork/exec per request.

2. **Basil HTTP server behind Apache** (most flexible)

    * Axum/Actix service; Apache proxies only dynamic routes.
    * Lets you preload stdlib/modules, cache compiled bytecode, and keep VM state (sessions).

next we can:

* stub a minimal FastCGI worker in Rust, or
* sketch a `BasilRequest -> BasilResponse` API so your shim calls the VM directly (no subprocess), or
* design a tiny web stdlib for Basil (`request.get`, `request.post`, `env`, `print/echo`, headers, cookies).







Below are working snippets for each, plus a tiny Rust CGI shim to drop in right now:

---

# 1) Classic CGI (quickest to prove out)

### What you get

* No daemon to manage.
* Apache spawns your Rust binary per request.
* Perfect for a prototype and correctness testing.

### Steps

**A. Enable modules**

```bash
sudo a2enmod cgi rewrite actions headers
sudo systemctl restart apache2
```

**B. Place your CGI binary**

* Build Basil’s runner (release):

```bash
cargo build --release -p basilc
```

* Copy it where Apache expects CGI and make it executable:

```bash
sudo install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi
```

**C. vhost config (map *.basil to the CGI)**
Put this inside your site’s `<VirtualHost …>`:

```apache
# Allow CGI under /cgi-bin/
ScriptAlias /cgi-bin/ /usr/lib/cgi-bin/
<Directory "/usr/lib/cgi-bin/">
    Options +ExecCGI
    AllowOverride None
    Require all granted
</Directory>

# 1) Tell Apache that ".basil" files should be handled by a named action
AddHandler basil-script .basil
Action basil-script /cgi-bin/basil.cgi

# 2) (Optional) Rewrite to pass the actual file path to the CGI via env
RewriteEngine On
# Map URL like /app/foo.basil -> sets SCRIPT_FILENAME to /var/www/app/foo.basil
RewriteCond %{REQUEST_FILENAME} !-f
RewriteRule ^/app/(.+\.basil)$ /cgi-bin/basil.cgi [QSA,PT,E=SCRIPT_FILENAME:/var/www/app/$1]

# Security & sane CGI timeouts
Timeout 30
```

**D. How your CGI should behave**

* Read the request from `stdin` (POST body) and `QUERY_STRING`/`REQUEST_METHOD`/`CONTENT_TYPE`/`CONTENT_LENGTH` from env.
* Figure out which Basil source to run (from `SCRIPT_FILENAME` if present, or derive from `PATH_TRANSLATED`/`PATH_INFO`).
* Print valid CGI headers, then a blank line, then the body.

**Minimal Rust CGI shim (drop-in)**
This wraps your existing Basil interpreter entry point. Replace the `run_basil_file` stub with the call into your VM.

```rust
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

fn run_basil_file(path: &str, query: &str, method: &str, body: &[u8]) -> Result<String, String> {
    // TODO: Call into your Basil VM here, injecting env/query/body as needed.
    // For now we just echo:
    Ok(format!(
        "<html><body><h1>Basil CGI</h1>\
         <p>File: {}</p><p>Method: {}</p><p>Query: {}</p><pre>{}</pre></body></html>",
        path, method, query, String::from_utf8_lossy(body)
    ))
}

fn main() {
    // 1) Resolve target script
    let script_path = env::var("SCRIPT_FILENAME")
        .or_else(|_| env::var("PATH_TRANSLATED"))
        .or_else(|_| env::var("PATH_INFO").map(|p| format!("/var/www{}", p)))
        .unwrap_or_else(|_| "/var/www/html/index.basil".to_string());

    if !Path::new(&script_path).exists() {
        // CGI error response
        println!("Status: 404 Not Found");
        println!("Content-Type: text/plain");
        println!();
        println!("Basil file not found: {}", script_path);
        return;
    }

    // 2) Collect request info
    let method = env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
    let query = env::var("QUERY_STRING").unwrap_or_default();
    let content_len: usize = env::var("CONTENT_LENGTH").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut body = Vec::with_capacity(content_len);
    if content_len > 0 {
        let mut stdin = io::stdin();
        stdin.take(content_len as u64).read_to_end(&mut body).ok();
    }

    // 3) Run Basil
    match run_basil_file(&script_path, &query, &method, &body) {
        Ok(html) => {
            // You can set cookies/headers as needed:
            println!("Status: 200 OK");
            println!("Content-Type: text/html; charset=utf-8");
            println!();
            print!("{}", html);
        }
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Basil runtime error:\n{}", e);
        }
    }
}
```

**Try it**

* Put a Basil script at `/var/www/app/hello.basil`.
* Visit `https://yourhost/app/hello.basil?name=Erik`.

That’s enough to prove the plumbing works.

---

# 2) FastCGI (same Apache, much faster)

### Why

CGI forks a new process per request. FastCGI keeps a Basil process (or pool) warm, so no cold start on every hit.

### Shape of it

* Install/enable `mod_fcgid`:

  ```bash
  sudo apt-get install libapache2-mod-fcgid
  sudo a2enmod fcgid actions proxy_fcgi
  sudo systemctl restart apache2
  ```
* Build a FastCGI Basil server (there are small Rust crates that speak FastCGI, or you can implement the protocol—straightforward).
* Point Apache at a Unix socket or TCP port exposed by your Basil FastCGI.

**Example Apache config (socket):**

```apache
# Tell Apache to send .basil to FastCGI socket
AddHandler basil-fcgi .basil
Action basil-fcgi /fcgi-basil

# Proxy to a Unix socket your Basil service creates (e.g., /run/basil-fcgi.sock)
ProxyPassMatch ^/fcgi-basil$ unix:/run/basil-fcgi.sock|fcgi://localhost

# Optional rewrite to let pretty URLs hit Basil
RewriteEngine On
RewriteRule ^/app/(.+\.basil)$ /fcgi-basil [QSA,PT,E=SCRIPT_FILENAME:/var/www/app/$1]
```

**Systemd unit (Basil FastCGI service):**

```ini
[Unit]
Description=Basil FastCGI
After=network.target

[Service]
User=www-data
Group=www-data
ExecStart=/usr/local/bin/basil-fcgi --socket /run/basil-fcgi.sock --workers 4
RuntimeDirectory=basil-fcgi
RuntimeDirectoryMode=0755
Restart=always
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
MemoryMax=256M

[Install]
WantedBy=multi-user.target
```

This gives you PHP-FPM-like performance characteristics without changing your site layout.

---

# 3) Reverse proxy to a long-running Basil HTTP server (most flexible)

### Why

* You keep Basil in Rust’s happy place: an async HTTP server (Axum/Actix/Hyper).
* Apache does TLS/virtual hosts/static files; it forwards dynamic routes to Basil.
* Easiest to scale horizontally; you can also keep “session”/VM state across requests.

**Apache vhost:**

```apache
# Enable proxying
ProxyPreserveHost On
ProxyPass        /basil/ http://127.0.0.1:4000/
ProxyPassReverse /basil/ http://127.0.0.1:4000/

# Or: only certain extensions
RewriteEngine On
RewriteRule ^/app/(.+\.basil)$ http://127.0.0.1:4000/app/$1 [P,QSA,L]
```

**Basil server responsibilities**

* Parse URL/query/headers/body, run the Basil program (mapped by route or extension), return HTTP directly.
* Can implement caching, module preloading, per-tenant sandboxes—things that are harder via CGI.

---

## Making Basil feel like PHP

PHP’s tight integration comes from three pieces you can emulate:

1. **File mapping convention**

    * `.basil` files live alongside `.php` files.
    * Apache routes them via Action/FastCGI/Proxy based on extension.
    * Your runtime resolves `SCRIPT_FILENAME`, `DOCUMENT_ROOT`, and `PATH_INFO` just like PHP does.

2. **Request API**

    * Standardize `$_GET`, `$_POST`, `$_SERVER` analogs in Basil (e.g., `env.get("QUERY_STRING")`, `request.post["foo"]`, `request.header("X-…")`).
    * Provide a core stdlib module for web I/O so Basil scripts don’t deal with raw CGI envs.

3. **Process manager**

    * For production, avoid classic CGI. Use FastCGI or a basil-http server with a small pool of workers.
    * Add preload hooks (autoload modules), per-site config (ini/toml), and opcache-like bytecode caching if your VM supports it.

---

## Practical extras

* **Input size & time limits**

    * Apache: `LimitRequestBody`, `Timeout`.
    * Basil runner: enforce `CONTENT_LENGTH` caps; set an execution deadline (e.g., 2–5s).

* **Security boundary**

    * Run as `www-data`, drop privileges, no shelling out.
    * Chroot-like path guard: deny `..` path traversal, restrict to a base directory.

* **Error surfacing**

    * Map Basil VM errors to `Status: 500` with a clean text/html body.
    * In dev mode, show trace; in prod, log to syslog/journal, show a friendly page.

* **Static vs dynamic**

    * Let Apache serve static assets. Only route `.basil` or specific prefixes to Basil.

* **Testing**

    * Add a `/cgi-bin/diag.cgi` that dumps the env so you can compare what Basil sees.
    * Unit test your request parser (env/POST handling) separately.

---

## First steps:

1. Implement the **Rust CGI shim** above (swap `run_basil_file` to call your VM).
2. Add the **vhost** bits that map `.basil` → `/cgi-bin/basil.cgi`.
3. Put a trivial `hello.basil` that reads `name` from the query and prints HTML.
4. Confirm GET and POST work; confirm 404 and 500 paths work.
5. Once happy, graduate to **FastCGI** (you’ll feel the perf difference immediately).
6. Long-term, consider a **Basil HTTP server** behind `mod_proxy`, so you get sessions, preloads, caching, and zero fork/exec per request.


