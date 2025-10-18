# Basilica (GUI Starter App)

This guide explains what landed in this iteration for the Basilica GUI starter app and how to build and run it alongside the existing Basil toolchain and sample programs.

Note: This is the first incremental drop. The executable and config machinery are present; a fuller GUI + embedded VM loop will follow in subsequent increments.


## What’s included now

New workspace members (under crates/):
- basilica — the GUI starter binary crate (basilica.exe on Windows)
- basil-embed — a thin adapter for embedding the Basil VM (scaffolded)
- basil-host — host surface definitions for APP.*, WEB.*, and BASILICA.MENU.* (scaffolded)

Initial features implemented in this increment:
- Config persistence in a standard config directory (basilica.json), with a seeded default menu layout on first run.
- Example Basil programs under examples/ for quick testing and future GUI menu wiring.
- A placeholder basilica main that loads or seeds config and exits; GUI rendering and bootstrap mode are scaffolded but not yet wired through in main.

Planned for upcoming increments (already scaffolded in source):
- eframe/egui main window with menus for CLI Scripts / GUI Scripts, ad‑hoc Run Script…, and Manage Scripts… dialogs.
- Console instance(s) and paired HTML webview windows via wry, with APP.* and WEB.* host bindings.
- Bootstrap CLI mode: basilica --bootstrap <path> to run a Basil script that programmatically sets menus via BASILICA.MENU.*.


## Repository layout (relevant parts)

- crates/basilica/ — GUI app
  - src/config.rs — basilica.json schema, load/save, seed config
  - src/app.rs — GUI scaffolding (menus, consoles, dialogs)
  - src/instance.rs — Console/Webview instance wiring (scaffolded)
  - src/main.rs — current entry point (placeholder flow)
- crates/basil-embed/ — BasilRunner scaffold
- crates/basil-host/ — host API type scaffolding
- examples/ — sample Basil scripts


## Build

The full workspace contains optional crates that may pull in native dependencies (e.g., nettle-sys) which require pkg-config, etc. If you see pkg-config errors on Windows, build only the Basilica-related crates for now.

Recommended commands:

- Build just Basilica-related packages:
  - Windows PowerShell:
    - cargo build -p basil-host -p basil-embed -p basilica

- Build entire workspace (may require pkg-config and other native deps depending on features in your environment):
  - cargo build --workspace

If you want colored output but hit terminal quirks, keep it simple; the minimal commands above are sufficient.


## First run and config

On first run, Basilica creates a basilica.json with the seed menu in your OS config directory:
- Windows: %APPDATA%\Basilica\basilica\basilica.json (via directories crate)
- Linux/macOS: ~/.config/basilica/basilica.json

Seed menu (summary):
- CLI Scripts
  - Basil Prompt — bare, mode=cli
  - Run Hello — file examples/hello.basil, mode=run
- GUI Scripts
  - Blank GUI Prompt — bare, mode=cli
  - GUI Hello — file examples/gui_hello.basil, mode=run

Run the app:
- cargo run -p basilica

Current behavior: prints a short status line indicating the number of CLI/GUI items loaded and exits. This verifies config creation and load. The GUI window and embedded VM are part of the next increment; their scaffolding is already present in the source tree.


## Running sample Basil programs today (via basilc)

While Basilica’s GUI execution loop is being wired up, you can run the sample Basil programs with the existing CLI tool basilc:

- Run the simple hello:
  - cargo run -p basilc -- run examples\hello.basil

- Explore other examples in examples\ like:
  - examples\gui_hello.basil — demonstrates WEB.* calls intended for Basilica’s paired webview.
  - examples\bootstrap_minimal.basil — demonstrates BASILICA.MENU.* API usage (to be enabled by Basilica’s bootstrap mode).
  - examples\bootstrap_pos_demo.basil — seeds a small “POS” demo menu.
  - examples\pos_cashier.basil, examples\pos_inventory.basil — GUI-oriented scripts for future Basilica webview.

Note: The GUI/webview behaviors in these scripts will show their PRINT output in basilc today; the webview effects require Basilica’s GUI, which will be activated in a subsequent increment.


## Future: Running Basil programs inside Basilica

Once the GUI entry point in basilica/src/main.rs is switched over to launch the eframe app:
- Launch basilica (cargo run -p basilica) to open the main window.
- Use the “CLI Scripts” and “GUI Scripts” menus to start instances from basilica.json.
- Use “Run Script…” for ad-hoc files; choose window type (CLI vs GUI) and mode (run/test/cli).
- Use “Manage Scripts…” to add/edit/delete menu items and save atomically to basilica.json.

Webview integration (wry):
- WEB.SET_HTML$(html$) sets the page DOM.
- WEB.EVAL$(js$) runs JavaScript.
- WEB.ON%(event$, id$, label) registers event routing back into your Basil code.


## Bootstrap mode (headless)

Planned CLI:
- basilica --bootstrap examples\bootstrap_minimal.basil

Behavior:
- Loads basilica.json, creates a pending config.
- Runs the Basil script inside an embedded VM with BASILICA.MENU.* enabled to mutate the pending config.
- If the script calls BASILICA.MENU.SAVE%(), writes basilica.json atomically and exits 0.

Current status: the flag is recognized in main.rs as a placeholder. The full flow will be enabled in an upcoming increment.


## Troubleshooting

- pkg-config / nettle-sys errors on Windows during a workspace build:
  - Workaround: build just the Basilica-related crates: cargo build -p basil-host -p basil-embed -p basilica
  - Or install pkg-config (e.g., choco install pkgconfiglite) if you need to build the full workspace.

- Where is basilica.json?
  - See “First run and config” above; use the OS-specific config dir. If directories lookup fails, Basilica falls back to the executable directory.

- I don’t see a GUI window yet.
  - That’s expected in this increment: the main entry point currently prints status to stdout to validate config creation. The eframe/egui UI and wry webview are scaffolded and will be enabled next.


## Quick commands (copy/paste)

- Build Basilica packages only:
  - cargo build -p basil-host -p basil-embed -p basilica

- Run Basilica (current placeholder):
  - cargo run -p basilica

- Run Hello with the CLI compiler/runner:
  - cargo run -p basilc -- run examples\hello.basil

- Try a bootstrap script (placeholder mode today):
  - cargo run -p basilica -- --bootstrap examples\bootstrap_minimal.basil


---
Last updated: 2025-10-18