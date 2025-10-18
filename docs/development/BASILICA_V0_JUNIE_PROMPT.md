Awesome — here’s a clean, copy-paste **Junie prompt** to build the **Basilica** starter app exactly as we discussed, including the new **bootstrap Basil script** flow that can populate Basilica’s menus from the command line.

---



---

Implement “Basilica” (GUI Starter App with Embedded Basil)

Goal
Create a new GUI binary crate named **basilica** (produces `basilica.exe` on Windows) inside our existing Cargo workspace. Basilica must:

1. Launch a desktop GUI with two dynamic menu groups: **CLI Scripts** and **GUI Scripts**.
2. Allow users to **Add / Edit / Delete** items in these menus (persisted in a JSON config).
3. Open **multiple Basil instances**:

    * Console-only REPL windows
    * Console + **paired HTML webview** windows (JS/IPC round-trip)
4. Support **Run Modes** for scripts: `run` / `test` / `cli`.
5. Support **“Run Script…”** (ad-hoc) from disk with chosen run mode and window type.
6. Support a **bootstrap mode** via CLI: `basilica --bootstrap path/to/setup.basil`.

    * Basilica runs the bootstrap Basil script inside an embedded VM with special **BASILICA.MENU** API to populate menus programmatically, then saves config and exits.
7. Include runnable **example Basil scripts** under `examples/`.

Constraints

* Do **not** write tests.
* Do **not** perform any Git operations.
* Provide working examples in `examples/`.
* Keep `basilc` and `bcc` builds/usage **undisturbed**.

Workspace layout (additions only)

```
/crates/
  basilica/                 # NEW: GUI app binary (outputs basilica or basilica.exe)
  basil-embed/              # NEW: thin adapter to embed Basil VM + host APIs
  basil-host/               # NEW: small host “mods” exposed to Basil (APP.*, WEB.*, BASILICA.MENU.*)
```

Top-level Cargo
Add `basilica`, `basil-embed`, and `basil-host` to the workspace members. Basilica depends on eframe/egui for GUI, `wry` for webview, `rfd` for native file dialogs.

Basilica features (must implement)

A) Config & persistence

* JSON file `basilica.json` in a config dir (use OS-appropriate: `%APPDATA%/Basilica/` on Windows, `~/.config/basilica/` on Linux/macOS).
* Schema:

  ```
  {
    "version": 1,
    "cli_scripts": [ MenuItem... ],
    "gui_scripts": [ MenuItem... ]
  }
  ```
* `MenuItem`:

    * `id: string` (stable id)
    * `name: string` (menu label)
    * `mode: "run" | "test" | "cli"`
    * `kind: "bare" | "file"`
    * `path?: string` (required for kind="file")
    * `args?: string` (optional)
* On first run, if config does not exist, create it with a **seed config** (see Examples).

B) GUI (eframe/egui)

* Menu bar:

    * **CLI Scripts** → items from config; each opens **Console** window instance.
    * **GUI Scripts** → items from config; each opens **Console+Browser** instance.
    * **Run Script…** → picker + dialog (choose CLI/GUI & mode); launches as new instance.
    * **Manage Scripts…** → CRUD dialog to add/edit/delete items across both lists; **Save** persists JSON atomically.
    * **Quit**
* A side “busy-box” panel with:

    * “Click me” counter
    * Start/Stop fake “animation”
    * “Open File…” (native dialog)
* Instances:

    * Console window with a text log and single-line input.
    * GUI instance also opens a `wry` webview window (or tab) paired to that console.

C) Embedded Basil VM adapter (`basil-embed`)

* Provide a `BasilRunner` that:

    * Spawns a VM instance on a worker thread.
    * Accepts input lines (REPL style) and returns output lines via channels.
    * Recognizes `exit` / `quit` to terminate the instance (GUI closes that console + webview).
* Replace the stub with our real Basil VM hook points (feed line / run file).
* Provide helpers to feed:

    * `RUN "path"` or `TEST "path"` or `CLI` start (equivalent to starting REPL)
    * Optionally `RUNBC "path.basilx"` (future)
* Each instance may optionally register host mods supplied by `basil-host`.

D) Host APIs for embedding (`basil-host`)
Implement three small host surfaces exposed to Basil (as Feature-Object-like bindings on embed):

1. `APP.*` (parent app utilities)

    * `APP.OPEN_FILE$()` → show native file dialog, return chosen path or empty.
    * `APP.ALERT%(msg$)` → show a GUI message box / egui window.
    * `APP.START_ANIM%()` / `APP.STOP_ANIM%()` → toggle the busy-box flag in GUI.

2. `WEB.*` (paired webview control)

    * `WEB.SET_HTML$(html$)` → replace DOM with provided HTML.
    * `WEB.EVAL$(js$)` → evaluate JS in the webview.
    * `WEB.ON%(event$, id$, label)` → register a mapping (event,id) → Basil label.
      Runtime: Inject a JS bootstrap in the webview that sends events via `window.ipc.postMessage({event,id})`; Rust IPC handler routes to the instance’s event queue and invokes the registered Basil label.

3. `BASILICA.MENU.*` (bootstrap customization API)
   Exposed only during **bootstrap mode** or when explicitly enabled for admin use:

    * `BASILICA.MENU.CLEAR%()`
    * `BASILICA.MENU.ADD_CLI%(name$, mode$, kind$, path$, args$)`
    * `BASILICA.MENU.ADD_GUI%(name$, mode$, kind$, path$, args$)`
    * `BASILICA.MENU.SAVE%()` → validates + writes `basilica.json` and returns 1 on success.
      Conventions:
    * `mode$` is `"run"|"test"|"cli"`.
    * `kind$` is `"bare"|"file"`.
    * For `bare`, ignore `path$`.
    * Permit empty `args$`.
    * Generate stable ids (`slugify(name$)` + numeric suffix if needed).

E) Webview (wry)

* Create a `wry::WebView` per GUI instance.
* Inject this JS at startup:

  ```html
  <script>
    window.BASIL = {
      send: (obj) => window.ipc.postMessage(obj),
      receive: (payload) => { /* handle Rust→JS messages; payload is JSON */ }
    };
    document.addEventListener('click', (e) => {
      const id = e.target?.id;
      if (id) BASIL.send({ event: 'click', id });
    });
  </script>
  ```
* Rust→JS: `webview.evaluate_script(&format!("BASIL.receive({});", json))?`
* JS→Rust: IPC handler parses JSON `{event,id}` and dispatches via the `WEB.ON%` mapping.

F) CLI flags for Basilica

* `basilica` → normal GUI.
* `basilica --bootstrap <path-to-basil>` → run headless/bootstrap:

    * Create a **temporary Basil VM** with `BASILICA.MENU.*` enabled and run the script in `run` mode (or accept `--mode` but `run` is fine).
    * If script finishes successfully and called `BASILICA.MENU.SAVE%()`, exit code 0.
    * On error, exit non-zero and print a succinct message.
    * No main window is needed for bootstrap; you may log to a minimal console.

G) Window types and run modes

* For any MenuItem:

    * If **Console-only**:

        * `bare + cli` → open REPL (no auto-run).
        * `file + run` → feed `RUN "path"`; close instance when program ends.
        * `file + test` → feed `TEST "path"`.
        * `file + cli` → feed `RUN "path"` then remain in REPL.
    * For **Console+Browser**: same as above **plus** create the webview and register `WEB.*`.

H) UX niceties

* “Manage Scripts…” dialog:

    * Tabs: CLI Scripts / GUI Scripts
    * Table of items with Add/Edit/Delete
    * Editor fields: Name, Kind (Bare/File), File chooser (if File), Mode, Args
    * Validate required fields; persist atomically to `basilica.json`.
* “Run Script…”:

    * File picker (filters `*.basil`)
    * Radio: **CLI window** or **GUI window**
    * Mode selector (run/test/cli)

Files to create (high-level)

`/crates/basilica/Cargo.toml`

* Depends on: `eframe`, `egui`, `wry`, `rfd`, `serde`, `serde_json`, `anyhow`, `parking_lot`, `crossbeam-channel`, `directories` (for config dir), plus local `basil-embed` and `basil-host`.

`/crates/basilica/src/main.rs`

* `fn main()` wires eframe app, parses CLI (`--bootstrap`), and either runs bootstrap flow or launches GUI.

`/crates/basilica/src/app.rs`

* `struct BasilicaApp { config, consoles, busy_box_state, alerts }`
* Menu bar, busy-box panel, consoles management, manage-scripts dialog.

`/crates/basilica/src/config.rs`

* `BasilicaConfig`, `MenuItem`, enums, load/save helpers, default seed config.

`/crates/basilica/src/instance.rs`

* Instance management for:

    * Console-only: holds `BasilRunner`, io channels, log buffer, input buffer.
    * Console+Browser: plus `WebView` handle and event routing.

`/crates/basil-embed/src/lib.rs`

* `BasilRunner` (spawn thread, channels, graceful shutdown on `exit`).
* Trait to register host mods from `basil-host`.
* Helpers: `run_file(mode, path, args)`, `start_cli()`.
* Placeholder lines to plug our real Basil VM (call into interpreter/compiler crates as needed).

`/crates/basil-host/src/lib.rs`

* Implement bindings for:

    * `APP.OPEN_FILE$`, `APP.ALERT%`, `APP.START_ANIM%`, `APP.STOP_ANIM%`.
    * `WEB.SET_HTML$`, `WEB.EVAL$`, `WEB.ON%`.
    * `BASILICA.MENU.CLEAR%`, `BASILICA.MENU.ADD_CLI%`, `BASILICA.MENU.ADD_GUI%`, `BASILICA.MENU.SAVE%`.
* Provide a small registry struct per instance to store WEB event routes and pending MENU changes.

Bootstrap flow (must implement)

* CLI: `basilica --bootstrap <script.basil>`
* Steps:

    1. Load existing `basilica.json` into a mutable “pending config”.
    2. Start embedded VM with `BASILICA.MENU.*` implemented to mutate the **pending config**.
    3. Execute the given Basil script (`RUN "..."`).
    4. If script calls `BASILICA.MENU.SAVE%()`, write **pending config** to disk and print “Saved N CLI items, M GUI items.”
    5. Exit with code 0 on success, non-zero on errors.
* Disable `WEB.*` in pure bootstrap runs (not needed).

Examples (place these exact files)

`examples/hello.basil`

```
PRINT "Hello from Basil!"
FOR i = 1 TO 3
  PRINT "i="; i
NEXT
```

`examples/gui_hello.basil`

```
WEB.SET_HTML$("<h3 id='h'>Hi from Basilica</h3><button id='go'>Go</button><div id='log'></div>")
WEB.ON%("click","go", OnGo)
PRINT "Open the GUI window and click Go."
END

OnGo:
PRINT "Go clicked!"
WEB.EVAL$("document.getElementById('log').innerText += 'Clicked!\\n';")
RETURN
```

`examples/bootstrap_minimal.basil`

```
' Clear and add two quick items, then save
BASILICA.MENU.CLEAR%()

BASILICA.MENU.ADD_CLI%("Basil Prompt", "cli", "bare", "", "")
BASILICA.MENU.ADD_CLI%("Run Hello", "run", "file", "examples/hello.basil", "")

BASILICA.MENU.ADD_GUI%("Blank GUI Prompt", "cli", "bare", "", "")
BASILICA.MENU.ADD_GUI%("GUI Hello", "run", "file", "examples/gui_hello.basil", "")

IF BASILICA.MENU.SAVE%() = 1 THEN
  PRINT "Basilica menu updated."
ELSE
  PRINT "Failed to save menu."
END IF
```

`examples/bootstrap_pos_demo.basil`

```
' Example: seed a tiny “POS” menu set
BASILICA.MENU.CLEAR%()

BASILICA.MENU.ADD_GUI%("POS - Cashier", "run", "file", "examples/pos_cashier.basil", "")
BASILICA.MENU.ADD_GUI%("POS - Inventory", "run", "file", "examples/pos_inventory.basil", "")
BASILICA.MENU.ADD_CLI%("Maintenance Shell", "cli", "bare", "", "")

BASILICA.MENU.SAVE%()
```

`examples/pos_cashier.basil` (stub GUI)

```
WEB.SET_HTML$("
  <style>body{font-family:sans-serif} .btn{padding:8px;margin:4px}</style>
  <h2>POS - Cashier</h2>
  <button id='sale' class='btn'>New Sale</button>
  <div id='o'></div>
")
WEB.ON%("click","sale", OnSale)
PRINT "Cashier ready."
STOP

OnSale:
PRINT "Start sale"
WEB.EVAL$("document.getElementById('o').innerText='Sale started';")
RETURN
```



Default seed config (write on first run if no config)

* CLI Scripts:

    * “Basil Prompt” → kind=bare, mode=cli
    * “Run Hello” → kind=file, path=`examples/hello.basil`, mode=run
* GUI Scripts:

    * “Blank GUI Prompt” → kind=bare, mode=cli
    * “GUI Hello” → kind=file, path=`examples/gui_hello.basil`, mode=run

Build/Run (developer convenience)

* `cargo run -p basilica` → launches GUI
* `cargo run -p basilica -- --bootstrap examples/bootstrap_minimal.basil` → applies menu bootstrap then exits
* `cargo run -p basilica -- --bootstrap examples/bootstrap_pos_demo.basil`

Implementation notes

* Use `directories` crate to resolve config path; fallback to executable dir if needed.
* Ensure atomic writes (write to temp then rename).
* Webview must be owned per instance; keep a handle for IPC.
* Basil VM must be per instance; wire `APP.*`, `WEB.*` only when relevant; `BASILICA.MENU.*` only in bootstrap or if a hidden “Admin Mode” toggle is on.
* Keep UI thread non-blocking: run VM on worker, communicate via channels.
* Exit an instance cleanly when user types `exit`/`quit` or closes its window; closing webview should not crash the instance.

Deliverables

* New crates `basilica/`, `basil-embed/`, `basil-host/` with the functionality above.
* `basilica.json` created on first run (or after bootstrap).
* All example Basil scripts under `examples/` included as listed.

Out of Scope (now)

* Tests.
* Git ops.
* Packaging/installer/signing.
* Advanced web assets bundling (keep HTML inline for the demo).

Please implement exactly as specified.
