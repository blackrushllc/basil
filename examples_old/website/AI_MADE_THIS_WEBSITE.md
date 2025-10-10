> ###### AI MADE THIS WEBSITE
>
> Me:
>
>Create the database file if it does not exist.
>
>You can put the database file in the main website directory.
>
>Make it look minimally pretty.
>
>This is intended to be the most minimal skeleton website for us to look at and consider the viability of making a small
> Basil web framework similar to Slim for Php.
>
>You can add some content to the home page view and the logged in view explaining what this example code is and why we
> made it.
>
>We aren't worried about pretty URLs at all. The Apache configration for the web server that we will host this on will
> be a minimal configuration with no special Rewrite directives. We'll do that later too for pretty URLs.
>
>
>Have fun!
>

Junie Ultimate AI agent:

Plan:

1. Verify whether examples\website already exists and if any files need creation or updates.
2. Create directory structure: examples\website, examples\website\views, examples\website\css, examples\website\js.
3. Add Basil pages using the specified content.

- index.basil
- login.basil
- register.basil
- user_home.basil
- logout.basil

4. Add static view files in examples\website\views.

- home.html
- login.html
- register.html
- logged_in.html

5. Add assets in examples\website.

- css\site.css
- js\site.js

6. Sanity-check the created files for typos, path correctness, and runtime assumptions (SQLite file path, headers).
7. Summarize changes and finalize submission.

Junie's Answer:

### What you’ll build

A minimal CGI website skeleton under `examples\website` with:

- Home page
- Login page (form + POST handler)
- Registration page (form + POST handler)
- Logged-in home page with greeting and Logout button
- Static `views` for inner page bodies, emitted from Basil files that render full HTML (doctype, head, body)
- SQLite user store (`website.db`), auto-created with table on first run
- Lightweight cookie-based login session
- Minimal CSS and tiny JS

This is intentionally small and vanilla so you can evaluate a Basil-based micro web framework (Slim-like) running via
CGI with Apache.

---

### Directory layout to add (under `examples\website`)

- `index.basil` — public home page
- `login.basil` — shows login form; on POST sets a session cookie
- `register.basil` — shows registration form; on POST inserts user
- `user_home.basil` — requires auth; greets the logged-in user; shows logout button
- `logout.basil` — clears cookie and redirects to home
- `views\home.html` — inner content for public home
- `views\login.html` — inner content with a login form
- `views\register.html` — inner content with a registration form
- `views\logged_in.html` — inner content for logged-in page
- `css\site.css` — tiny style
- `js\site.js` — tiny script
- `website.db` — SQLite DB file (auto-created at runtime)

Note: Basil doesn’t yet support cross-file imports/includes, so each page carries small shared helpers (headers,
cookies, params, DB open). That keeps the example self-contained and easy to read.

---

### Shared concepts used in the pages

- CGI prelude: `#CGI_NO_HEADER` so we explicitly emit headers (and can send `Set-Cookie`, `Location`, etc.)
- Request parameters: use `REQUEST$()` which returns an array of `name=value` strings (already URL-decoded)
- Cookies: read `ENV$("HTTP_COOKIE")`; set via printing `Set-Cookie:` header before the blank line
- SQLite: `SQLITE_OPEN%`, `SQLITE_EXEC%`, `SQLITE_QUERY2D$`, `SQLITE_LAST_INSERT_ID%`, `SQLITE_CLOSE`
- Files: `READFILE$(path$)` for including view HTML
- Escaping: `HTML$(s$)` when reflecting user input back to the page

Security note: for skeleton simplicity, passwords are stored as plain text. For anything real, hash with a proper
algorithm (e.g., bcrypt/scrypt/argon2) and use secure cookies, SameSite, and CSRF tokens.

---

### 1) Public home page — `examples\website\index.basil`

```basil
#CGI_NO_HEADER
<?basil
  REM ----- helpers -----
  LET SITE_TITLE$ = "Basil Website Skeleton"
  SUB send_header_ok_html()
    PRINT "Status: 200 OK\r\n";
    PRINT "Content-Type: text/html; charset=utf-8\r\n";
    PRINT "Cache-Control: no-store\r\n\r\n";
  END SUB

  SUB layout_start(title$)
    PRINT "<!doctype html>\n";
    PRINT "<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>" + HTML$(title$) + "</title>\n";
    PRINT "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n";
    PRINT "<link rel=\"stylesheet\" href=\"css/site.css\">\n";
    PRINT "</head>\n<body>\n<header class=\"top\"><div class=\"wrap\"><h1>" + HTML$(SITE_TITLE$) + "</h1></div></header>\n";
    PRINT "<main class=\"wrap\">\n";
  END SUB

  SUB layout_end()
    PRINT "</main>\n<footer class=\"foot\"><div class=\"wrap\">\n";
    PRINT "<small>Example Basil CGI skeleton • <a href=\"index.basil\">Home</a></small>\n";
    PRINT "</div></footer>\n<script src=\"js/site.js\"></script>\n</body></html>\n";
  END SUB

  FUNCTION param$(name$)
    DIM arr$(0)
    LET arr$() = REQUEST$()
    FOR EACH kv$ IN arr$()
      LET eq% = FIND(kv$, "=")
      IF eq% >= 0 THEN
        LET k$ = LEFT$(kv$, eq%)
        LET v$ = MID$(kv$, eq%+2)
        IF k$ == name$ THEN RETURN v$
      ENDIF
    NEXT
    RETURN ""
  END FUNCTION

  FUNCTION cookie$(name$)
    LET raw$ = ENV$("HTTP_COOKIE")
    IF LEN(raw$) == 0 THEN RETURN ""
    LET s$ = raw$
    REM split on ';'
    LET i% = 1
    WHILE i% <= LEN(s$)
      LET semi% = FIND(s$, ";", i%-1)
      IF semi% < 0 THEN LET semi% = LEN(s$)
      LET part$ = TRIM$(MID$(s$, i%, semi% - i% + 1))
      LET eq% = FIND(part$, "=")
      IF eq% >= 0 THEN
        LET ck$ = LEFT$(part$, eq%)
        LET cv$ = MID$(part$, eq%+2)
        IF ck$ == name$ THEN RETURN cv$
      ENDIF
      LET i% = semi% + 2
    WEND
    RETURN ""
  END FUNCTION

  SUB send_header_redirect(loc$)
    PRINT "Status: 302 Found\r\n";
    PRINT "Location: " + loc$ + "\r\n\r\n";
  END SUB

  REM ----- page -----
  LET user$ = cookie$("user")
  CALL send_header_ok_html()
  CALL layout_start("Welcome")
?>
<?basil PRINT READFILE$("views/home.html"); ?>
<?basil
  IF LEN(user$) > 0 THEN
    PRINT "<section class=\"notice\">You appear to be logged in as <strong>" + HTML$(user$) + "</strong>.\n";
    PRINT " Visit your <a href=\"user_home.basil\">dashboard</a>.";
    PRINT "</section>\n";
  ELSE
    PRINT "<p><a class=\"btn\" href=\"login.basil\">Log in</a> or <a class=\"btn secondary\" href=\"register.basil\">Create an account</a></p>\n";
  ENDIF
  CALL layout_end()
?>
```

---

### 2) Login page — `examples\website\login.basil`

```basil
#CGI_NO_HEADER
<?basil
  REM ----- minimal helpers (same as index, plus DB and cookie setter) -----
  LET SITE_TITLE$ = "Basil Website Skeleton"
  SUB send_header_ok_html()
    PRINT "Status: 200 OK\r\n";
    PRINT "Content-Type: text/html; charset=utf-8\r\n";
    PRINT "Cache-Control: no-store\r\n";
  END SUB
  SUB header_flush()
    PRINT "\r\n";
  END SUB
  SUB set_cookie(name$, value$)
    PRINT "Set-Cookie: " + name$ + "=" + value$ + "; Path=/; HttpOnly\r\n";
  END SUB
  SUB send_header_redirect(loc$)
    PRINT "Status: 302 Found\r\n";
    PRINT "Location: " + loc$ + "\r\n\r\n";
  END SUB
  SUB layout_start(title$)
    PRINT "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>" + HTML$(title$) + "</title>";
    PRINT "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">";
    PRINT "<link rel=\"stylesheet\" href=\"css/site.css\"></head><body><header class=\"top\"><div class=\"wrap\"><h1>" + HTML$(SITE_TITLE$) + "</h1></div></header><main class=\"wrap\">\n";
  END SUB
  SUB layout_end()
    PRINT "</main><footer class=\"foot\"><div class=\"wrap\"><small><a href=\"index.basil\">Home</a></small></div></footer><script src=\"js/site.js\"></script></body></html>\n";
  END SUB

  FUNCTION param$(name$)
    DIM arr$(0)
    LET arr$() = REQUEST$()
    FOR EACH kv$ IN arr$()
      LET eq% = FIND(kv$, "=")
      IF eq% >= 0 THEN
        LET k$ = LEFT$(kv$, eq%)
        LET v$ = MID$(kv$, eq%+2)
        IF k$ == name$ THEN RETURN v$
      ENDIF
    NEXT
    RETURN ""
  END FUNCTION

  REM ----- SQLite helpers -----
  FUNCTION db_open%()
    LET db% = SQLITE_OPEN%("website.db")
    IF db% == 0 THEN RETURN 0
    LET _ = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT UNIQUE, password TEXT)")
    RETURN db%
  END FUNCTION
  FUNCTION user_exists%(db%, username$)
    DIM rows$(0,0)
    LET rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username = '" + username$ + "' LIMIT 1")
    RETURN ARRAY_ROWS%(rows$) > 0
  END FUNCTION
  FUNCTION check_login%(db%, username$, password$)
    DIM rows$(0,0)
    LET rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username='" + username$ + "' AND password='" + password$ + "' LIMIT 1")
    RETURN ARRAY_ROWS%(rows$) > 0
  END FUNCTION

  REM ----- handle POST -----
  LET method$ = ENV$("REQUEST_METHOD")
  IF method$ == "POST" THEN
    LET u$ = param$("username")
    LET p$ = param$("password")

    LET db% = db_open%()
    IF db% == 0 THEN
      PRINT "Status: 500 Internal Server Error\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDB open failed";
      EXIT 0
    ENDIF

    IF check_login%(db%, u$, p$) THEN
      REM Set cookie and redirect to user_home
      PRINT "Status: 302 Found\r\n";
      CALL set_cookie("user", u$)
      PRINT "Location: user_home.basil\r\n\r\n";
      SQLITE_CLOSE(db%)
      EXIT 0
    ELSE
      REM fall through to render with error
      LET err$ = "Invalid username or password"
    ENDIF
    SQLITE_CLOSE(db%)
  ENDIF

  REM ----- GET or failed POST -> show form -----
  CALL send_header_ok_html()
  CALL layout_start("Log in")
?>
<?basil PRINT READFILE$("views/login.html"); ?>
<?basil
  IF LEN(err$) > 0 THEN
    PRINT "<p class=\"error\">" + HTML$(err$) + "</p>\n";
  ENDIF
  CALL layout_end()
?>
```

---

### 3) Registration page — `examples\website\register.basil`

```basil
#CGI_NO_HEADER
<?basil
  REM helpers (same pattern as login)
  LET SITE_TITLE$ = "Basil Website Skeleton"
  SUB send_header_ok_html()
    PRINT "Status: 200 OK\r\n";
    PRINT "Content-Type: text/html; charset=utf-8\r\n";
    PRINT "Cache-Control: no-store\r\n";
  END SUB
  SUB set_cookie(name$, value$)
    PRINT "Set-Cookie: " + name$ + "=" + value$ + "; Path=/; HttpOnly\r\n";
  END SUB
  SUB send_header_redirect(loc$)
    PRINT "Status: 302 Found\r\n";
    PRINT "Location: " + loc$ + "\r\n\r\n";
  END SUB
  SUB layout_start(title$)
    PRINT "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>" + HTML$(title$) + "</title>";
    PRINT "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">";
    PRINT "<link rel=\"stylesheet\" href=\"css/site.css\"></head><body><header class=\"top\"><div class=\"wrap\"><h1>" + HTML$(SITE_TITLE$) + "</h1></div></header><main class=\"wrap\">\n";
  END SUB
  SUB layout_end()
    PRINT "</main><footer class=\"foot\"><div class=\"wrap\"><small><a href=\"index.basil\">Home</a></small></div></footer><script src=\"js/site.js\"></script></body></html>\n";
  END SUB
  FUNCTION param$(name$)
    DIM arr$(0)
    LET arr$() = REQUEST$()
    FOR EACH kv$ IN arr$()
      LET eq% = FIND(kv$, "=")
      IF eq% >= 0 THEN
        LET k$ = LEFT$(kv$, eq%)
        LET v$ = MID$(kv$, eq%+2)
        IF k$ == name$ THEN RETURN v$
      ENDIF
    NEXT
    RETURN ""
  END FUNCTION

  FUNCTION db_open%()
    LET db% = SQLITE_OPEN%("website.db")
    IF db% == 0 THEN RETURN 0
    LET _ = SQLITE_EXEC%(db%, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT UNIQUE, password TEXT)")
    RETURN db%
  END FUNCTION
  FUNCTION create_user%(db%, username$, password$)
    LET _ = SQLITE_EXEC%(db%, "INSERT INTO users(username, password) VALUES ('" + username$ + "','" + password$ + "')")
    RETURN SQLITE_LAST_INSERT_ID%(db%)
  END FUNCTION
  FUNCTION user_exists%(db%, username$)
    DIM rows$(0,0)
    LET rows$() = SQLITE_QUERY2D$(db%, "SELECT id FROM users WHERE username = '" + username$ + "' LIMIT 1")
    RETURN ARRAY_ROWS%(rows$) > 0
  END FUNCTION

  LET method$ = ENV$("REQUEST_METHOD")
  IF method$ == "POST" THEN
    LET u$ = param$("username")
    LET p$ = param$("password")
    LET p2$ = param$("password2")

    LET db% = db_open%()
    IF db% == 0 THEN
      PRINT "Status: 500 Internal Server Error\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDB open failed";
      EXIT 0
    ENDIF

    IF LEN(u$) == 0 OR LEN(p$) == 0 THEN
      LET err$ = "Please fill in all fields"
    ELSEIF p$ <> p2$ THEN
      LET err$ = "Passwords do not match"
    ELSEIF user_exists%(db%, u$) THEN
      LET err$ = "That username is already taken"
    ELSE
      LET _id% = create_user%(db%, u$, p$)
      PRINT "Status: 302 Found\r\n";
      CALL set_cookie("user", u$)
      PRINT "Location: user_home.basil\r\n\r\n";
      SQLITE_CLOSE(db%)
      EXIT 0
    ENDIF
    SQLITE_CLOSE(db%)
  ENDIF

  CALL send_header_ok_html()
  CALL layout_start("Register")
?>
<?basil PRINT READFILE$("views/register.html"); ?>
<?basil
  IF LEN(err$) > 0 THEN PRINT "<p class=\"error\">" + HTML$(err$) + "</p>\n";
  CALL layout_end()
?>
```

---

### 4) Logged-in home — `examples\website\user_home.basil`

```basil
#CGI_NO_HEADER
<?basil
  LET SITE_TITLE$ = "Basil Website Skeleton"
  SUB send_header_ok_html()
    PRINT "Status: 200 OK\r\n";
    PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  END SUB
  SUB layout_start(title$)
    PRINT "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>" + HTML$(title$) + "</title>";
    PRINT "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">";
    PRINT "<link rel=\"stylesheet\" href=\"css/site.css\"></head><body><header class=\"top\"><div class=\"wrap\"><h1>" + HTML$(SITE_TITLE$) + "</h1></div></header><main class=\"wrap\">\n";
  END SUB
  SUB layout_end()
    PRINT "</main><footer class=\"foot\"><div class=\"wrap\"><small><a href=\"index.basil\">Home</a></small></div></footer><script src=\"js/site.js\"></script></body></html>\n";
  END SUB
  FUNCTION cookie$(name$)
    LET raw$ = ENV$("HTTP_COOKIE")
    IF LEN(raw$) == 0 THEN RETURN ""
    LET s$ = raw$
    LET i% = 1
    WHILE i% <= LEN(s$)
      LET semi% = FIND(s$, ";", i%-1)
      IF semi% < 0 THEN LET semi% = LEN(s$)
      LET part$ = TRIM$(MID$(s$, i%, semi% - i% + 1))
      LET eq% = FIND(part$, "=")
      IF eq% >= 0 THEN
        LET ck$ = LEFT$(part$, eq%)
        LET cv$ = MID$(part$, eq%+2)
        IF ck$ == name$ THEN RETURN cv$
      ENDIF
      LET i% = semi% + 2
    WEND
    RETURN ""
  END FUNCTION
  SUB send_header_redirect(loc$)
    PRINT "Status: 302 Found\r\n";
    PRINT "Location: " + loc$ + "\r\n\r\n";
  END SUB

  LET user$ = cookie$("user")
  IF LEN(user$) == 0 THEN
    CALL send_header_redirect("login.basil")
    EXIT 0
  ENDIF

  CALL send_header_ok_html()
  CALL layout_start("Your dashboard")
?>
<?basil PRINT READFILE$("views/logged_in.html"); ?>
<?basil
  PRINT "<p class=\"greet\">Welcome, <strong>" + HTML$(user$) + "</strong>!</p>\n";
  PRINT "<form method=\"post\" action=\"logout.basil\"><button class=\"btn\">Log out</button></form>\n";
  CALL layout_end()
?>
```

---

### 5) Logout — `examples\website\logout.basil`

```basil
#CGI_NO_HEADER
<?basil
  REM Clear cookie by expiring it
  PRINT "Status: 302 Found\r\n";
  PRINT "Set-Cookie: user=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT\r\n";
  PRINT "Location: index.basil\r\n\r\n";
?>
```

---

### 6) Static inner views (HTML only)

- `examples\website\views\home.html`

```html

<section class="intro">
    <h2>Minimal Basil CGI site</h2>
    <p>
        This demo shows a tiny CGI web skeleton rendered by Basil. Basil files
        emit the full HTML page (doctype, head, header, footer) and pull in these
        view snippets for the inner body. It also demonstrates a SQLite-backed
        user store and the simplest cookie-based session.
    </p>
</section>
```

- `examples\website\views\login.html`

```html
<h2>Log in</h2>
<form method="post" action="login.basil" class="card">
    <label>Username<br><input name="username" required></label>
    <label>Password<br><input name="password" type="password" required></label>
    <button class="btn" type="submit">Log in</button>
</form>
<p class="muted">Don’t have an account? <a href="register.basil">Register here</a>.</p>
```

- `examples\website\views\register.html`

```html
<h2>Create an account</h2>
<form method="post" action="register.basil" class="card">
    <label>Username<br><input name="username" required></label>
    <label>Password<br><input name="password" type="password" required></label>
    <label>Confirm Password<br><input name="password2" type="password" required></label>
    <button class="btn" type="submit">Register</button>
</form>
```

- `examples\website\views\logged_in.html`

```html
<h2>Your dashboard</h2>
<p>
    You’re seeing this because login succeeded and a cookie was set. This
    example aims to validate Basil as a minimal CGI templating option
    (think Slim for PHP): one file per page, clear entry points, static views,
    and a small sprinkle of helpers.
</p>
```

---

### 7) Minimal styling — `examples\website\css\site.css`

```css
:root {
    --ink: #222;
    --muted: #667;
    --bg: #f7f7fb;
    --acc: #2867c6;
    --acc2: #eef4ff;
}

* {
    box-sizing: border-box
}

body {
    margin: 0;
    font: 16px/1.5 system-ui, Segoe UI, Roboto, Helvetica, Arial, sans-serif;
    color: var(--ink);
    background: var(--bg)
}

.wrap {
    max-width: 900px;
    margin: 0 auto;
    padding: 1rem
}

.top {
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, .06)
}

.top h1 {
    margin: 0;
    font-size: 1.25rem
}

.foot {
    color: var(--muted);
    font-size: .9rem
}

main h2 {
    margin-top: .2rem
}

.intro p {
    max-width: 60ch
}

.btn {
    display: inline-block;
    background: var(--acc);
    color: #fff;
    border: 0;
    padding: .6rem .9rem;
    border-radius: .4rem;
    text-decoration: none;
    cursor: pointer
}

.btn.secondary {
    background: transparent;
    color: var(--acc);
    border: 1px solid var(--acc)
}

.card {
    background: #fff;
    border: 1px solid #e5e7eb;
    padding: 1rem;
    border-radius: .5rem;
    max-width: 420px
}

label {
    display: block;
    margin: .5rem 0
}

input {
    width: 100%;
    padding: .5rem;
    border: 1px solid #cbd5e1;
    border-radius: .3rem
}

.error {
    color: #b00020;
    margin-top: .8rem
}

.notice {
    background: var(--acc2);
    padding: .8rem;
    border-radius: .5rem
}

.greet {
    font-size: 1.1rem
}
```

---

### 8) Tiny script — `examples\website\js\site.js`

```js
console.log("Basil website skeleton loaded");
```

---

### Running with obj-all and CGI

- Build the Basil CLI with all objects:
    - `cargo run -q -p basilc --features obj-all -- run examples\sqlite_demo.basil` (sanity-check SQLite locally)
- Deploy for CGI as shown in `README.md`/`BASIL_CGI.md` (Linux Apache flow):
    1) Build release and install CGI runner:
        - `cargo build -q -p basilc --release --features obj-all`
        - `install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi`
    2) Map URLs to Basil files in your web root. With minimal Apache (no rewrites), you typically expose Basil pages
       directly, e.g.:
        - Put this folder at `/var/www/html/website`.
        - Configure your CGI handler so requests like `/website/index.basil` are executed by
          `/usr/lib/cgi-bin/basil.cgi` with `SCRIPT_FILENAME` pointing at that file (see `BASIL_CGI.md` for exact
          SetEnvIf/AddHandler examples). The project already supports classic CGI detection via env vars.
    3) Then visit:
        - `http://your-host/website/index.basil`
        - `http://your-host/website/login.basil`, `register.basil`, `user_home.basil`, `logout.basil`

Notes:

- Because we send headers manually (`#CGI_NO_HEADER`), we can set cookies and do redirects before the blank line.
- `website.db` will be created alongside the pages on first write.
- This intentionally avoids any “pretty URL” layer for now, matching your constraints.

---

### Next steps if you evolve this into a mini-framework

- Factor helpers into a common include once Basil gains file-include/import support
- Add password hashing, CSRF tokens, SameSite cookies, and flash messages
- Provide a `render(view$, vars)` that merges variables into views (today we just `READFILE$`)
- Add a tiny router that switches on the path and dispatches subroutines
- Optionally move to cookie+session table instead of storing the username directly in a cookie

> Me: Perfect!  Please do this now

Junie Ultimate AI agent:

( work work work work ..)

Implemented a minimal CGI website skeleton with Basil pages and static views. Added cookie-based session handling and
SQLite user storage creating website.db on first use. Included basic CSS and JavaScript assets with proper headers and
local asset paths.

