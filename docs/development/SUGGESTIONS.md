# 🔝 My top recommendation

**Ship a tiny “web-ready Basil toolbelt” bundle**: `obj-all` + WASM demo + 5 core objects.

* Add 2 quick wins next: **`obj-json`** (`serde_json`) and **`obj-csv`** (`csv`).
* Result: you can read/write CSVs, parse/emit JSON, fetch via CURL, zip/unzip, base64… all from browser or CGI.

If you want, I’ll give you ready-to-paste Junie prompts for `obj-json` and `obj-csv` immediately.

# Other excellent next steps (choose one)

1. **CLASS quality-of-life**

    * Minimal **`SAVE class@ TO file$` / `LOAD class@ FROM file$`** (JSON under the hood).
    * Optional `CLASSVARS()` / `CLASSFUNCS()` introspection.
    * Gives you persistence with almost no new concepts.

2. **CURL polish (still simple)**

    * Add `HTTP_STATUS%()` and `HTTP_HEADER$(name$)` for the *last* request.
    * Optional 3rd param for timeout seconds on GET/POST.
    * Keeps v1 spirit, unlocks real-world APIs.

3. **PDF “hello world”**

    * `obj-pdf` with **printpdf**: start page, draw text, save file. No layout engine yet—just enough for receipts/labels.

4. **SQLite basics**

    * `obj-sqlite` with **rusqlite**: `SQLITE_OPEN`, `SQLITE_EXEC`, `SQLITE_QUERY$` (CSV/JSON result). Great for CGI.

5. **WASM runner UX**

    * Replace textarea with **xterm.js**, add a **Run/Reset** button and a file picker that loads a `.basilx`. It’ll feel like a tiny online IDE.

6. **Tooling/ergonomics**

    * **TextMate grammar** for `.basil` (VS Code + Geany), and a tiny **VS Code extension** to auto-detect/run via your wasm runner.

# Suggested order (fastest value → deeper power)

1. `obj-json`, `obj-csv` (fast, huge payoff)
2. CURL status/headers
3. CLASS save/load
4. SQLite or PDF (pick what you’ll use first)
5. xterm.js WASM runner
6. Syntax coloring + VS Code/Geany glue

