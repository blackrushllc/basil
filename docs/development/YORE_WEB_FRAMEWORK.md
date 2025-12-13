**Prompt for Basil Junie: Implement `obj-yore` web framework feature object**

"Yore" is the name of a PHP-based web framework that we want to port to Rust/BASIC.

You are **Basil Junie**, working inside the **Basil** Rust/BASIC project (not PHP Yore).

Your task:
Implement a new **Feature Object** called **`obj-yore`** that gives Basil a Yore-style web framework kernel:

* Multi-tenant domain routing based on `pages/_domains`.
* Section/pagekey/arg routing.
* Page JSON + view resolution.
* Integration with Basil’s existing `RENDER$()` + FRED directives.
* Support for:

    * Basil-written **modules** under `_modules`.
    * Optional **custom controllers** (web_* / api_*) per section.
    * Basic **Modeltroller** semantics adapted for Basil + `obj-orm`.

You have **no built-in knowledge of the PHP Yore code**, but you may use:

* The GitHub repo: `https://github.com/blackrushllc/yore` as a conceptual reference.
* See AI Onboarding and docs for Yore: /yore/.junie as a conceptual reference.
* 
* The attached AI onboarding files:

    * `guidelines.md` – high-level Yore guidelines + directory structure + definitions.
    * `MODELTROLLERS.md` – Models, Controllers, and Modeltroller routing semantics.
* The attached `Library.php`, `Controller.php` and `Modules.php` source files in this workspace for inspiration only.

You are implementing **Basil’s** version of this in Rust, with a Basil feature object API.

### 0. Constraints and style

* **Do not** create branches or pull requests.
* **Do not** add or modify tests.
* Work directly in the current branch / workspace.
* Only modify the **Basil** repo; do **not** change the PHP Yore repo.
* Keep code cohesive and reasonably small; follow existing Basil style and architecture for Feature Objects.

**Basil syntax rules (important):**

* Functions: type suffix only if **return is string or integer**:

    * `FUNCTION FOO$()` → returns string.
    * `FUNCTION BAR%()` → returns integer.
    * `FUNCTION BAZ()` → returns object/list/array/anything not string/int.
* Variables: use `$`, `%`, `@` suffixes as usual for string, integer, object/collection.
* Do **not** put `@` on function names, only on variables.
* Comments: use `//`, `#` or `REM`.

    * **Do not** use single quote `'` for comments (single quotes are valid string delimiters).
* Strings:

    * Single-quoted strings: no escapes, no interpolation.
    * Double-quoted and triple-double strings can use escapes and interpolation (`"Hello #{name$}"`).

Keep all sample Basil code you produce compliant with these rules.

### 1. Directory + routing model (Basil/Yore)

Implement Basil’s `obj-yore` to honor the Yore directory semantics from `guidelines.md`, adapted as follows:

#### 1.1 Root layout

* Framework content root (relative to Basil binary’s working directory) is:

  ```text
  pages/
      _domains/
          <domain>/
              env.json
              modules.json
              default/ or _default/
              <other section(s)>/
              _modules/ or modules/ or Modules/
              _models/ or Models/
              [optional per-section Controller files]
  ```

* `<domain>` is the lowercase host name; examples:

    * `app.yoreweb.com`
    * `blackrushbasic.com`
    * `_local` for a special local dev domain (see below).

#### 1.2 Domains and special `_local`

* Resolve the requested **domain** from the environment, typically:

    * `HTTP_HOST` or similar.
    * Normalize to lowercase, strip port if present (`:8080`).
* Domain folder resolution:

    * If host maps to `localhost` / `127.0.0.1` / `::1`, use the **special dev domain**:

        * Prefer `pages/_domains/_local/`
        * Fallback to `pages/_domains/local/` if `_local` does not exist.
    * Otherwise, use `pages/_domains/<domain>/` where `<domain>` is the normalized host. If not found, you may abort with 404 or a friendly error.

#### 1.3 Section, pagekey, arg1–arg3

* Based on the **path** part of the request URI (without query string):

  ```text
  /                      → section = "default", pagekey = "home"
  /about                 → section = "about", pagekey = "home"
  /about/team            → section = "about", pagekey = "team"
  /about/team/john       → section = "about", pagekey = "team", arg1 = "john"
  /about/team/john/doe   → ... arg1 = "john", arg2 = "doe"
  /about/team/john/doe/x → ... arg3 = "x"
  ```

* First slug → `section$`

* Second slug → `pagekey$`

* Third, fourth, fifth slugs → `arg1$`, `arg2$`, `arg3$`

* If no section slug (bare `/`), use `section = "default"`.

* If there is a section but no pagekey, use `pagekey = "home"`.

* Slug normalization:

    * Lowercase slugs.
    * Replace spaces, `-`, `.` with `_`.
    * Trim leading/trailing `/`.

This logic should be implemented in Rust (in `obj-yore`) consistently, mirroring what `App\Library::init()` does in PHP Yore.

#### 1.4 Reserved folder underscore variants

When mapping `section$` to a folder under the domain:

* For section `"default"`:

    * Prefer `_default` if the folder `pages/_domains/<domain>/_default/` exists.
    * Else use `pages/_domains/<domain>/default/`.
* For models and modules under the domain:

    * Models:

        * Prefer `_models` if `pages/_domains/<domain>/_models/` exists.
        * Else use `pages/_domains/<domain>/Models/`.
    * Modules:

        * Prefer `_modules` if present.
        * Else use `modules`, then `Modules` as fallback.

Implement helper functions internally so this resolution logic is centralized and not duplicated.

### 2. Page JSON and views

Page JSON rules come from `guidelines.md` and must be re-implemented in Basil/Yore:

* Each page of a domain has a corresponding JSON file in the appropriate section folder:

    * Top-level:

        * `pages/_domains/<domain>/default/home.json` for the domain home page.
        * Other pages: `admin.json`, `guest.json`, etc.
    * Sub-sections:

        * `pages/_domains/<domain>/<section>/home.json` home for that section.
        * `pages/_domains/<domain>/<section>/<pagekey>.json` for other pages.

* Views live under `views/` parallel to JSON:

    * `pages/_domains/<domain>/<section>/views/<view>.html`
    * `pages/_domains/<domain>/<section>/views/<view>.blade.php`
    * We will map this to Basil’s `RENDER$` using a plain text view file (no PHP).

* JSON fields to support (not exhaustive, but must cover these):

    * `view` → primary view name (without extension).
    * `views` → optional map of role → view name (for role-based overrides).
    * `theme` → theme name (folder under `web/themes` or Basil’s equivalent).
    * `title`, `page`, `description`, and any other arbitrary settings used by modules or views.
    * `module_hook` or similar string to indicate a Basil module function to run for this page (see modules section).
    * Any other fields should be passed through into the context dictionary for `RENDER$` as appropriate.

### 3. Basil-facing API of `obj-yore`

Implement the following BASIC-level functions exposed by `obj-yore`:

```basic
FUNCTION YORE_INIT%(OPTIONAL db_or_handle)
FUNCTION YORE_REQUEST()
FUNCTION YORE_RESOLVE_PAGE(req@)
FUNCTION YORE_BUILD_CONTEXT(req@, page@)
FUNCTION YORE_RENDER_PAGE$(page@, ctx@)
' Optional convenience
FUNCTION YORE_HANDLE_REQUEST$()
```

#### 3.1 `YORE_INIT%(OPTIONAL db_or_handle)`

* Purpose:

    * Initialize Yore/Basil context for the current request.
    * Optionally take a DB connection or handle created by `obj-sql` / `obj-orm`.

* Parameter behavior:

    * If called with an object (e.g. `db@ AS DB_MYSQL`):

        * Store a reference to this DB connection in the Yore context for later use in modules.
    * If called with an integer (e.g. SQLite handle from `SQLITE_OPEN%()`):

        * Store that handle in the context as a SQLite connection.
    * If called with no argument:

        * Assume “no database”; Yore may still run static pages.

* Additional responsibilities:

    * Determine and store the active `domain$` and domain folder path.
    * Load `env.json` for the current domain (if present), and:

        * Merge these values into Basil’s ENV abstraction if feasible, or
        * Store them in an internal config object accessible to `obj-yore` and modules.
    * Load `modules.json` for the current domain (if present) and keep the module enable/disable + debug settings available.

* Return:

    * Integer `%`:

        * `1` on success.
        * `0` on failure (e.g. missing domain, badly formatted JSON).

#### 3.2 `YORE_REQUEST()`

* Returns a request object `req@` (dictionary or object) containing at least:

    * `req@["domain$"]`
    * `req@["section$"]`
    * `req@["pagekey$"]`
    * `req@["arg1$"]`, `req@["arg2$"]`, `req@["arg3$"]` (may be empty strings when unused)
    * `req@["method$"]` (GET, POST, etc.)
    * `req@["path$"]` (the normalized path)
    * `req@["query@"]` (dictionary of query string params)
    * `req@["post@"]` (dictionary of POST form data)
    * Optionally:

        * `req@["headers@"]` (map of HTTP headers)
        * `req@["is_debug%"]` flag based on `?debug=1` and/or `modules.json` debug mode.

* Implementation:

    * Parse CGI/HTTP environment variables and fill this structure.
    * Apply slug normalization rules from §1.3.

#### 3.3 `YORE_RESOLVE_PAGE(req@)`

* Purpose:

    * Given the request object, determine which page JSON file to load and produce a **page metadata object** `page@`.

* Behavior:

    * Map `req@["section$"]` to an actual folder name using underscore rules (e.g. `"default"` may map to `_default`). See §1.4.
    * Determine the pagekey:

        * From `req@["pagekey$"]` or default to `"home"` if missing.
    * Compose the JSON path:

        * `${domain_folder}/${section}/home.json` or `${pagekey}.json` as appropriate.
    * Load and parse the JSON into `page@`.

* `page@` should contain at least:

    * `page@["section$"]`
    * `page@["pagekey$"]`
    * `page@["view$"]` (primary view name, possibly defaulted to `"home"`).
    * `page@["theme$"]` (explicit, or filled in from defaults / domain settings).
    * `page@["title$"]` (optional, can be derived from section/pagekey if absent).
    * `page@["views@"]` (role-based alternative views, if defined).
    * `page@["raw_json@"]` (optional: the full parsed JSON for debugging or advanced usage).
    * Optional `page@["module_hook$"]` or similar for module-based data enrichment.

* If the JSON is missing or invalid:

    * You may choose to return a special `page@` indicating a 404 page, or simply signal failure and let `YORE_HANDLE_REQUEST$` or caller decide.

#### 3.4 `YORE_BUILD_CONTEXT(req@, page@)`

* Purpose:

    * Build the dictionary `ctx@` of variables that will be used by Basil’s `RENDER$(view$, ctx@)`.

* Sources of data:

    * Request:

        * `section$`, `pagekey$`, `arg1$`, `arg2$`, `arg3$`, query, form, etc.
    * Domain:

        * Values from `env.json` (DB connection, domain title, etc).
        * Values from `modules.json` (enabled modules, debug flag) if relevant to views.
    * Page:

        * All relevant keys from the page JSON (`view`, `views`, `theme`, `title`, custom fields).
    * Modules:

        * If the page JSON declares a `module_hook` function name, call that Basil function and merge its returned dictionary into `ctx@`. See modules section below.

* Module hook invocation:

    * Given `page@["module_hook$"] = "SHOP_PAGE_DATA"` (for example):

        * Look for this function in domain Basil modules under:

            * `pages/_domains/<domain>/_modules/`
            * or `modules/` / `Modules/` as fallback.
        * Ensure that Basil source for those modules is loaded before invocation.
        * Call:

          ```basic
          ctxExtra@ = SHOP_PAGE_DATA(req@)
          ```

          where `ctxExtra@` is expected to be a dictionary.
        * Merge `ctxExtra@` into `ctx@` (page context). Callers later pass this `ctx@` into `RENDER$`.

#### 3.5 `YORE_RENDER_PAGE$(page@, ctx@)`

* Purpose:

    * Resolve the **view string** (layout + theme + body view) but do not apply FRED/RENDER$ yet.

* Responsibilities:

    * Determine the correct `section` folder (underscore variants, see §1.4).
    * Resolve the view name:

        * Prefer:

            * Role-based view from `page@["views@"]` if the current user role is known and mapped.
            * Else `page@["view$"]`.
            * Else default `"home"` if nothing is set.
    * Build the filesystem path under the domain’s section `views/` folder:

        * `<domain_folder>/<section>/views/<view>.html`
        * Or `<view>.blade` / `.blade.php` if you decide to maintain the extension semantics and then feed the file contents to Basil’s `RENDER$()`. (There is no PHP execution in Basil, just text templates)
    * Load the view file content into a string `template$`.
    * Optionally apply theming layout:

        * For example, wrap the view body into a theme layout file representing the full HTML document, but still in the form of a single string with FRED/RENDER$ placeholders.
        * You can mirror what PHP Yore does with BladeRenderer + theme packs, but adapted to Basil’s `RENDER$`.

* Return:

    * `template$` – the final string passed to `RENDER$(template$, ctx@)`.

#### 3.6 `YORE_HANDLE_REQUEST$()` (optional but recommended)

* High-level convenience:

    * Internally perform:

        * `YORE_INIT` (if not already called, or if Yore context is lazily initialized).
        * `req@   = YORE_REQUEST()`
        * `page@  = YORE_RESOLVE_PAGE(req@)`
        * `ctx@   = YORE_BUILD_CONTEXT(req@, page@)`
        * `html$  = YORE_RENDER_PAGE$(page@, ctx@)`
        * `output$ = RENDER$(html$, ctx@)`
    * Return `output$` as the fully rendered HTML body.

* This allows a very simple CGI script:

  ```basic
  #CGI_NO_HEADER
  LET okEnv% = LOADENV%()

  LET output$ = YORE_HANDLE_REQUEST$()

  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  PRINT output$
  ```

* But please also support the **more explicit pipeline**, as the user prefers, and ensure the API works with this pattern:

  ```basic
  #CGI_NO_HEADER
  LET okEnv% = LOADENV%()  // defaults to .env

  LET dsn$ = ENV$("DB_DSN")
  DIM db@ AS DB_MYSQL(dsn$)
  // OR:
  // LET sldb% = SQLITE_OPEN%("demo.db")

  LET isOk% = YORE_INIT(db@)
  // OR:
  // LET isOk% = YORE_INIT(sldb%)
  // OR:
  // LET isOk% = YORE_INIT()

  DIM req@   = YORE_REQUEST()
  DIM page@  = YORE_RESOLVE_PAGE(req@)
  DIM ctx@   = YORE_BUILD_CONTEXT(req@, page@)
  LET html$  = YORE_RENDER_PAGE$(page@, ctx@)
  LET output$ = RENDER$(html$, ctx@)

  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  PRINT output$
  ```

### 4. Basil modules under `_modules`

Implement a Basil-friendly module system for Yore:

* Domain module locations (in order of preference):

    * `pages/_domains/<domain>/_modules/`
    * `pages/_domains/<domain>/modules/`
    * `pages/_domains/<domain>/Modules/`

* Each module is a Basil source file (use whatever extension is standard in the Basil repo – likely `.basil`).

* Inside a module, define functions that can be used:

    1. As **page data hooks**:

        * e.g. `FUNCTION SHOP_PAGE_DATA(req@)` returning `ctxExtra@` (dictionary).
        * Called from `YORE_BUILD_CONTEXT` when the page JSON references that function name.

    2. As **routable functions** for `/api/module_name/action` patterns (if you decide to preserve this style now or in a later iteration).

* For now, minimum requirement:

    * Support `module_hook` style page data functions as described in §3.4.

### 5. Custom controllers (web_* / api_*)

Adapt PHP Yore’s **custom controller** pattern to Basil. From MODELTROLLERS and Controller.php:

* For any section, allow an **optional Basil controller** file, e.g.:

  ```text
  pages/_domains/{domain}/{section}/Controller.basil
  ```

* This file defines **global functions** (Basil) with these prefixes:

    * `FUNCTION web_index()` → returns HTML string.
    * `FUNCTION api_status()` → returns arrays/objects that are JSON-encoded by the CGI wrapper.

* Routing rule:

    * For a normal page URL like `/{section}/{pagekey}`:

        * If a controller file exists for `{section}`, and it defines `web_{pagekey}`, then:

            * Prefer that handler and **skip** the page JSON/view pipeline.
        * Else, if it defines `api_{pagekey}`, treat it as an API call returning raw data (you may check a query parameter or request header to decide whether to use `api_` vs `web_` if both exist, but simplest is:

            * Web path uses `web_*` if present, else falls back to JSON page.
            * API paths (like `/api/...`) use `api_*` – see next bullet).
    * For API-style calls, you may:

        * Use an `/api/...` prefix in the path to choose `api_*` explicitly if you want to mirror Yore’s behavior.

* Implementation pattern:

    * `obj-yore` should expose internal logic that:

        * Detects whether a controller file exists for the resolved section.

        * Dynamically loads that Basil file in the Basil environment (however Basil currently supports dynamic loading – you know this context; re-use existing mechanisms).

        * Checks for presence of `web_{action}` or `api_{action}` functions and calls them with:

          ```basic
          ' signature suggestion
          FUNCTION web_name(req@)  ' returns HTML string
          FUNCTION api_name(req@)  ' returns array/object/dictionary
          ```

          or

          ```basic
          FUNCTION web_name(req@, arg1$, arg2$, arg3$)
          ```

        * If `web_*` returns HTML, then Yore should still apply theming as necessary (wrap that HTML in the domain theme layout) before emitting the response; you can mirror PHP Yore’s `renderHtmlBody` + `ensurePageDataForCustomController` behavior in Basil’s logic.

* Ensure that when a custom controller handles a request, you still have enough `page@`/`ctx@` data (possibly synthesized) for theming and debug output.

### 6. Modeltrollers (Basil version)

Adapt the **Modeltroller** pattern described in `MODELTROLLERS.md` to Basil + `obj-orm`:

* Concept:

    * A **Modeltroller** is a Basil ORM model that also defines routable `web_*` and `api_*` functions.
    * Routing pattern to support (mirroring Yore):

      ```text
      /{section}/models/{Model}/{action}[/{arg1}/{arg2}/{arg3}]
      ```

* Resolution rules (adapted):

    * Derive a “Studly” model name from the `{Model}` slug (like PHP Yore’s `studly()`):
      `orders` → `Orders`; `user_profile` → `UserProfile`.
    * Expect to find a Basil ORM model with that name in the domain’s **models** folder:

        * `pages/_domains/{domain}/_models/` (preferred) or
        * `pages/_domains/{domain}/Models/`.

* Dispatch:

    * For a URL like `/customers/models/orders/store`:

        * Domain: resolved normally.
        * Section: `"customers"`.
        * Model slug: `"orders"` → Studly `"Orders"`.
        * Action: `"store"`.
        * Arg1–3: extra path components after the action.

    * Given the Basil ORM model (or model-like module), we want to call:

        * Prefer:

            * `web_store(...)` for web routes
        * Else:

            * `api_store(...)` for API routes

    * Recommended signature for Basil Modeltroller functions:

      ```basic
      FUNCTION web_index(req@, arg1$, arg2$, arg3$)  ' returns HTML string
      FUNCTION api_store(req@, arg1$, arg2$, arg3$)  ' returns array/object/dict
      ```

    * The exact binding between Basil ORM models and these functions depends on how `obj-orm` defines models. Use existing Basil ORM patterns to attach Modeltroller methods, but keep the external routing and naming consistent with the PHP description.

### 7. RENDER$ integration

Basil already has `RENDER$(view$, ctx@)` and FRED directives:

* `RENDER$(template$, ctx@)`:

    * Second argument is a dictionary of variables.
    * Keys become variable names in the view context.
    * String and integer values are appended with `$` or `%` type symbols automatically unless the key already has them.
    * User-defined functions may be called from inside the view (`{{ MyFunc$(x) }}`) if they are defined before rendering.
    * FRED directives like `@IF`, `@FOREACH`, etc., should already work.

`obj-yore` must:

* Always build a `ctx@` that is ready for `RENDER$`.
* Never attempt to parse or execute FRED/Blade syntax itself; just pass raw template strings to `RENDER$`.
* Provide enough context variables so that views can reference:

    * `section$`, `pagekey$`, `arg1$`, `arg2$`, `arg3$`.
    * Domain name, theme, title, and arbitrary page or env settings.

### 8. Examples and docs to add

Please also:

1. **Examples**

   Add at least two example Basil programs under `examples/`:

    * `examples/yore_cgi_mysql.basil`

        * Uses:

            * `LOADENV%()`
            * `DB_MYSQL` (or whatever MySQL connector obj-sql/obj-orm exposes)
            * `YORE_INIT(db@)`
            * Full explicit pipeline: `YORE_REQUEST`, `YORE_RESOLVE_PAGE`, `YORE_BUILD_CONTEXT`, `YORE_RENDER_PAGE`, `RENDER$`, printing CGI headers.

    * `examples/yore_cgi_sqlite.basil`

        * Similar, but uses a SQLite handle instead (e.g. `SQLITE_OPEN%("demo.db")` and `YORE_INIT(sldb%)`).

2. **Docs**

   Add a short doc page in the Basil docs tree, e.g. `docs/FEATURES/obj-yore.md`, including:

    * Overview: what `obj-yore` is and when to use it.
    * Directory structure expectations (multi-tenant `_domains`, section/pagekey/args).
    * The BASIC API:

        * `YORE_INIT%`
        * `YORE_REQUEST`
        * `YORE_RESOLVE_PAGE`
        * `YORE_BUILD_CONTEXT`
        * `YORE_RENDER_PAGE$`
        * Optional `YORE_HANDLE_REQUEST$`
    * A minimal end-to-end CGI example (like `yore_cgi_mysql.basil`).
    * A quick note on custom controllers and Modeltrollers (referring users to the Basil version of `MODELTROLLERS` ideas).

3. **Feature registration**

    * Implement `obj-yore` as a feature object consistent with the rest of the Basil codebase.
    * Ensure it is gated by an appropriate Cargo feature flag (`obj-yore` or similar).
    * Register the BASIC functions into the interpreter’s function registry in the same style as existing Feature Objects.

---

**End of prompt for Basil Junie.**

