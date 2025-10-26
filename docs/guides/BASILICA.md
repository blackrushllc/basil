# Basilica (GUI Starter App) — Proof of Concept

This document describes the current state of the Basilica desktop GUI and how to build and run it with the included Basil examples.

Important: Basilica is currently a proof‑of‑concept (POC) GUI for Basil. It demonstrates the full loop end‑to‑end (menus → Basil VM → optional webview) but some edges are intentionally minimal.


## What’s included in this POC

New workspace members (under crates/):
- basilica — the GUI binary (basilica.exe on Windows)
- basil-embed — thin adapter that embeds the Basil VM and exposes host APIs
- basil-host — host “objects” available to Basil programs: APP.*, WEB.*, BASILICA.MENU.*

Implemented features:
- Config persistence (basilica.json) located in the OS‑standard config dir; default menu is seeded on first run.
- GUI built with eframe/egui:
  - CLI Scripts and GUI Scripts menus (driven by basilica.json)
  - Run Script… dialog (ad‑hoc .basil file; choose Run/Test/CLI mode and window type)
  - Manage Scripts… dialog (Add/Edit/Delete items; Save writes basilica.json atomically)
  - Multiple console instances; each GUI instance also owns a paired HTML webview
- Webview via wry with Windows‑safe helper process strategy:
  - On Windows, a helper mode (`--webview-helper`) runs Tao/Wry on the main thread; Basilica spawns it automatically when needed.
  - The webview injects a small JS bootstrap and relays DOM events (clicks with element id) back to the console instance.
- Host APIs available to Basil scripts:
  - APP.OPEN_FILE$(), APP.ALERT%(), APP.START_ANIM%(), APP.STOP_ANIM%()
  - WEB.SET_HTML$(html$), WEB.EVAL$(js$), WEB.ON%(event$, id$, label)
  - BASILICA.MENU.* for bootstrap/menu seeding
- Bootstrap mode is implemented headless: `basilica --bootstrap <script.basil>` mutates a pending menu via BASILICA.MENU.* and saves on SAVE%().
- Feature‑flag parity with basilc: you can run Basilica with the same `--features` (e.g., `obj-all`, `obj-safe`, `obj-json`…).
- Examples under examples/ (hello, gui_hello, POS stubs, bootstrap scripts).

What’s intentionally minimal (for now):
- WEB.ON% event routing is fully registered and events are received; invoking the mapped Basil label is logged today but the callback back into the running VM is not yet executed.


## Repository layout (relevant parts)

- crates/basilica/
  - src/config.rs — basilica.json schema, load/save, seed config
  - src/app.rs — GUI (menus, consoles, dialogs)
  - src/instance.rs — console instances, host request handling, webview launch
  - src/main.rs — app entry point; implements `--bootstrap` and `--webview-helper`
  - src/bin/basilica-webview-helper.rs — dedicated helper binary to host Tao/Wry on Windows
- crates/basil-embed/ — BasilRunner + REPL/file‑run helpers; seeds APP/WEB/BASILICA.MENU
- crates/basil-host/ — host object implementations + thread context
- examples/ — sample Basil scripts


## Build

The workspace contains optional crates that may pull in native deps. If you hit pkg‑config or native library issues on Windows, build just the Basilica‑related crates.

Recommended commands (Windows PowerShell):
- Build Basilica‑related packages only:
  - cargo build -p basil-host -p basil-embed -p basilica
- Build entire workspace (may require extra native deps):
  - cargo build --workspace

Features (parity with basilc):
- Run all objects enabled:
  - cargo run -p basilica --features obj-all
- Safer portable set (excludes audio/DAW/SFTP):
  - cargo run -p basilica --features obj-safe
- Fine‑grained examples:
  - cargo run -p basilica --features obj-json,obj-csv


## First run and config

On first run, Basilica writes a seed basilica.json into your config dir:
- Windows: %APPDATA%\Basilica\basilica\basilica.json
- Linux/macOS: ~/.config/basilica/basilica.json

Seed menu summary:
- CLI Scripts
  - Basil Prompt — bare, mode=cli
  - Run Hello — file examples/hello.basil, mode=run
- GUI Scripts
  - Blank GUI Prompt — bare, mode=cli
  - GUI Hello — file examples/gui_hello.basil, mode=run


## Run the GUI

- Start the app:
  - cargo run -p basilica
- Use the top menu:
  - CLI Scripts → opens a console instance
  - GUI Scripts → opens a console paired with a webview window
  - Run Script… → pick a .basil file, choose mode and window type, then Launch
  - Manage Scripts… → CRUD the menu entries; Save writes basilica.json atomically

Expected behavior with examples:
- GUI Scripts → GUI Hello: opens a webview window that renders the inline HTML; console shows PRINT output.
- Run Script… + GUI window + examples\gui_hello.basil: same as above.
- Clicking “Go” logs a [WEB.EVENT] line; the OnGo label is not invoked yet in this POC.


## Bootstrap mode (headless)

Run a bootstrap script to populate menus programmatically:
- cargo run -p basilica -- --bootstrap examples\bootstrap_minimal.basil
- cargo run -p basilica -- --bootstrap examples\bootstrap_pos_demo.basil

Behavior:
- Loads current basilica.json into a pending config.
- Runs the Basil script with BASILICA.MENU.* enabled.
- If the script calls BASILICA.MENU.SAVE%(), Basilica writes basilica.json atomically and prints a summary like:
  - Saved N CLI items, M GUI items.
- Exits 0 on success; non‑zero on error.


## Running examples with basilc (CLI)

You can still run samples via the CLI tool for comparison:
- cargo run -p basilc -- run examples\hello.basil


## Troubleshooting

- Webview didn’t appear on Windows:
  - Basilica uses a helper mode that runs Tao/Wry on a main thread (either a dedicated helper exe or `--webview-helper` in the same binary). If something fails, the console prints messages prefixed with [helper] or [webview-helper].
- Where is basilica.json?
  - See “First run and config” above. If directories lookup fails, Basilica falls back to the executable directory.
- Native dependency issues while building:
  - Build just the Basilica crates: `cargo build -p basil-host -p basil-embed -p basilica`.


## Quick commands (copy/paste)

- Build Basilica packages only:
  - cargo build -p basil-host -p basil-embed -p basilica
- Run Basilica (GUI):
  - cargo run -p basilica
- Run Basilica with all objects:
  - cargo run -p basilica --features obj-all
- Run the GUI Hello ad‑hoc:
  - cargo run -p basilica --
    (then in the app: Run Script… → pick examples\gui_hello.basil → GUI window → Launch)
- Bootstrap menus from a Basil script:
  - cargo run -p basilica -- --bootstrap examples\bootstrap_minimal.basil

---
Last updated: 2025-10-18 10:23