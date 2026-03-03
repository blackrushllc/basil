Below is a **copy/paste “exhaustive” Junie prompt** that should be enough for her to build a fully demonstratable UI (login → dashboard → workspace desktop with persistent draggable windows, mock data, exports, and theme toggle) using **plain HTML/CSS/JS** (no framework) and a tiny Basil server stub later.

---

## ✅ Junie Prompt: Build UI Prototype (Login + Dashboard + Desktop Windowed Workspace)

You are working in our Basil (Rust BASIC) web project repo. Build a **front-end-only demonstratable UI** using **plain HTML5 + CSS + vanilla JS** (no React/Vue). The goal is to produce a polished prototype we can tweak and later wire to Basil endpoints.

### 0) Project Layout & Files

Create these files (and any minimal supporting assets you need):

```
/web/
  index.html                 # Login page (hardcoded login)
  dashboard.html             # Dashboard + Workspace Desktop
  /assets/
    app.css                  # full styling, includes light/dark themes
    app.js                   # shared utilities (storage, theme, fetch wrapper)
    login.js                 # login behavior
    dashboard.js             # dashboard + window manager
    mock-api.js              # mock API responder for /api/query and /api/export
    icons.svg                # optional inline svg symbol sprite
  /data/
    mock-data.js             # hard-coded mock datasets (customers, appts, etc.)
/docs/
  UI_PROTOTYPE.md            # how to run + what’s implemented
```

No build step. Must run by just opening HTML files or via a simple static server.

---

## 1) Visual Design Requirements (Detailed)

### Global look and feel

* Professional, modern admin tool.
* Clean spacing, subtle shadows, rounded corners.
* Layout should feel like “analytics desktop/workbench”.
* Provide **light & dark mode** toggled in the top bar. Persist theme in localStorage.
* Use system font stack (no external fonts).

### Theme tokens

Define CSS variables for both themes:

* `--bg`, `--panel`, `--panel2`, `--text`, `--muted`, `--border`
* `--primary`, `--primary2`, `--danger`, `--warn`, `--good`
* `--shadow`, `--radius`, `--mono`
* Use `[data-theme="dark"]` on `<html>` to switch.

### Visual interactions

* Buttons have hover/active states.
* Inputs show focus rings.
* Windows have a crisp header bar with controls (minimize/maximize/close).
* Draggable windows show subtle “grab” cursor in the title bar.
* Resize handle bottom-right.
* Minimized windows appear in a **taskbar** at bottom (click to restore).
* Z-order: clicking a window brings to front.

---

## 2) HTML Structure (Exact)

### 2.1 `/web/index.html` (Login)

Must match this structure:

```html
<!doctype html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>EMC Desk — Login</title>
  <link rel="stylesheet" href="assets/app.css" />
</head>
<body class="page page-login">
  <div class="login-shell">
    <header class="login-header">
      <div class="brand">
        <div class="brand-mark">EMC</div>
        <div class="brand-name">
          <h1>EMC Desk</h1>
          <p class="muted">Exploration dashboard prototype</p>
        </div>
      </div>
      <button id="themeToggle" class="btn btn-ghost" type="button" aria-label="Toggle theme">
        <span class="icon">🌓</span>
        <span class="label">Theme</span>
      </button>
    </header>

    <main class="login-card">
      <h2>Sign in</h2>
      <p class="muted">Hard-coded prototype login (replace later with Basil auth).</p>

      <form id="loginForm" class="form">
        <label class="field">
          <span>Username</span>
          <input id="username" name="username" autocomplete="username" placeholder="owner" required />
        </label>

        <label class="field">
          <span>Password</span>
          <input id="password" name="password" type="password" autocomplete="current-password" placeholder="demo" required />
        </label>

        <label class="field checkbox">
          <input id="rememberMe" type="checkbox" checked />
          <span>Remember me</span>
        </label>

        <div class="row">
          <button class="btn btn-primary" type="submit">Login</button>
          <button id="fillDemo" class="btn btn-secondary" type="button">Fill Demo</button>
        </div>

        <div id="loginError" class="alert alert-danger hidden"></div>

        <details class="login-hint">
          <summary>Demo credentials</summary>
          <div class="mono">
            Username: <b>owner</b><br/>
            Password: <b>demo</b>
          </div>
        </details>
      </form>
    </main>

    <footer class="login-footer muted">
      <span>Prototype UI — no real patient/medical treatment data.</span>
    </footer>
  </div>

  <script src="assets/app.js"></script>
  <script src="assets/login.js"></script>
</body>
</html>
```

Login behavior:

* Hard-coded credentials: `owner/demo`.
* If correct: set `localStorage.authToken = "demo-token"` (and optionally `rememberMe` logic).
* Redirect to `dashboard.html`.
* If incorrect: show `#loginError`.

---

### 2.2 `/web/dashboard.html` (Dashboard + Desktop)

Must match this structure:

```html
<!doctype html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>EMC Desk — Dashboard</title>
  <link rel="stylesheet" href="assets/app.css" />
</head>
<body class="page page-dashboard">
  <div class="app-shell">

    <!-- Top bar -->
    <header class="topbar">
      <div class="left">
        <button id="navToggle" class="btn btn-ghost" type="button" aria-label="Toggle sidebar">☰</button>
        <div class="brand-inline">
          <span class="brand-mark small">EMC</span>
          <span class="brand-title">EMC Desk</span>
        </div>
      </div>

      <div class="center">
        <div class="global-search">
          <input id="globalPrompt" placeholder="Ask: no-shows last 30 days by chiropractor..." />
          <button id="runPrompt" class="btn btn-primary" type="button">Run</button>
        </div>
      </div>

      <div class="right">
        <button id="newWindowMenu" class="btn btn-secondary" type="button">New ▾</button>
        <button id="themeToggle" class="btn btn-ghost" type="button">🌓</button>
        <div class="user-menu">
          <button id="userBtn" class="btn btn-ghost" type="button">owner ▾</button>
          <div id="userDropdown" class="menu hidden">
            <button id="openDocs" class="menu-item">UI Help</button>
            <button id="logout" class="menu-item danger">Logout</button>
          </div>
        </div>
      </div>
    </header>

    <!-- Main split -->
    <div class="main">

      <!-- Sidebar -->
      <aside id="sidebar" class="sidebar">
        <section class="panel">
          <h3>Workspace</h3>
          <div class="muted small">Local persisted desktop</div>

          <div class="row wrap">
            <button id="openDesktop" class="btn btn-primary">Open Desktop</button>
            <button id="resetDesktop" class="btn btn-danger">Reset</button>
          </div>
        </section>

        <section class="panel">
          <h3>Quick Reports</h3>
          <div id="quickReports" class="list"></div>
        </section>

        <section class="panel">
          <h3>Saved</h3>
          <div id="savedArtifacts" class="list"></div>
        </section>

        <section class="panel">
          <h3>Filters</h3>
          <div class="filters">
            <label class="field">
              <span>Date range</span>
              <select id="dateRange">
                <option value="7">Last 7 days</option>
                <option value="30" selected>Last 30 days</option>
                <option value="90">Last 90 days</option>
                <option value="365">Last 12 months</option>
              </select>
            </label>
            <label class="field">
              <span>Reseller</span>
              <select id="resellerFilter"></select>
            </label>
            <label class="field">
              <span>Status</span>
              <select id="statusFilter"></select>
            </label>
          </div>
        </section>
      </aside>

      <!-- Content -->
      <main class="content">
        <!-- Dashboard view -->
        <section id="dashboardView" class="dashboard-view">
          <div class="kpi-grid" id="kpiGrid"></div>

          <div class="dash-row">
            <section class="panel grow">
              <header class="panel-header">
                <h3>Trends</h3>
                <div class="muted small">Mock chart tiles (wire to real data later)</div>
              </header>
              <div id="trendTiles" class="tile-grid"></div>
            </section>

            <section class="panel side">
              <header class="panel-header">
                <h3>Recent Activity</h3>
              </header>
              <div id="activityFeed" class="feed"></div>
            </section>
          </div>

          <section class="panel">
            <header class="panel-header">
              <h3>Suggested Questions</h3>
              <div class="muted small">Click to open as windows</div>
            </header>
            <div id="suggestedQuestions" class="chips"></div>
          </section>
        </section>

        <!-- Desktop view -->
        <section id="desktopView" class="desktop-view hidden">
          <div class="desktop-toolbar">
            <div class="left">
              <button id="backToDashboard" class="btn btn-secondary">← Dashboard</button>
              <div class="muted small">Drag windows. Double-click title to maximize.</div>
            </div>
            <div class="right">
              <button id="tileWindows" class="btn btn-ghost">Tile</button>
              <button id="cascadeWindows" class="btn btn-ghost">Cascade</button>
              <button id="newTableWin" class="btn btn-secondary">+ Table</button>
              <button id="newSummaryWin" class="btn btn-secondary">+ Summary</button>
              <button id="newChartWin" class="btn btn-secondary">+ Chart</button>
            </div>
          </div>

          <div id="desktop" class="desktop" aria-label="Workspace desktop"></div>

          <div id="taskbar" class="taskbar" aria-label="Taskbar"></div>
        </section>
      </main>
    </div>
  </div>

  <script src="assets/mock-data.js"></script>
  <script src="assets/mock-api.js"></script>
  <script src="assets/app.js"></script>
  <script src="assets/dashboard.js"></script>
</body>
</html>
```

---

## 3) JS Data Model (Windows/Artifacts)

Create a strongly defined model in `dashboard.js` and document it in `UI_PROTOTYPE.md`.

### WorkspaceState (persisted)

Persist into localStorage key: `emcDesk.workspace.v1`

```js
/**
 * WorkspaceState
 * - id: string
 * - name: string
 * - createdAt: ISO
 * - updatedAt: ISO
 * - activeView: "dashboard" | "desktop"
 * - windows: WindowState[]
 * - zCounter: number
 * - settings: { theme: "light"|"dark", snap: boolean }
 */
```

### WindowState

```js
/**
 * WindowState
 * - id: string (uuid)
 * - type: "table" | "summary" | "chart" | "report" | "export_job"
 * - title: string
 * - prompt: string
 * - opts: object (parsed from JSON-ish or kept as string)
 * - queryContext: { dateRangeDays, reseller, status }  // from sidebar filters at time of creation
 * - x,y,w,h: numbers (px)
 * - z: number
 * - minimized: boolean
 * - maximized: boolean
 * - pinned: boolean
 * - createdAt, updatedAt, lastRunAt: ISO or null
 * - lastResult: QueryResult | null
 * - lastError: string | null
 */
```

### QueryResult shapes

```js
/**
 * QueryResult (union)
 * { kind:"table", columns:[{key,label,type}], rows:[{...}], rowCount:number }
 * { kind:"summary", text:string, bullets?:string[] }
 * { kind:"chart", chartType:"bar"|"line"|"pie", labels:string[], series:[{name,values:number[]}] }
 * { kind:"report", blocks:[ ... table/chart/summary blocks ... ] }
 */
```

### Artifact concept (Saved)

Persist “saved artifacts” into localStorage key: `emcDesk.savedArtifacts.v1`

SavedArtifact:

```js
/**
 * SavedArtifact
 * - id, title, type
 * - prompt
 * - queryContext
 * - createdAt, updatedAt
 * - snapshotResult?: QueryResult (optional)
 * - shareId?: string (placeholder for later server sharing)
 */
```

---

## 4) Window Manager Requirements (Behavior)

Implement a basic window manager in `dashboard.js`:

* Create window:

    * default placement: cascade offset (e.g., +24px each new window)
    * default size: 560x360
    * default z: ++zCounter
* Drag:

    * Only dragging the `.window-titlebar` moves it
    * Keep within desktop bounds (clamp)
* Resize:

    * bottom-right handle; min size 320x200
* Focus:

    * click brings to front (z = ++zCounter)
* Minimize:

    * window disappears from desktop, appears in taskbar as a pill
* Restore:

    * click taskbar pill restores
* Close:

    * remove window from state
* Maximize:

    * double-click title bar toggles maximized (fills desktop area under toolbar)
* Persist:

    * Any change (drag/resize/prompt edit/run/result) updates state and saves (debounced 400ms)

Window HTML template (exact structure inside desktop):

```html
<div class="win" data-win-id="...">
  <div class="win-titlebar">
    <div class="win-title">
      <span class="win-icon">📊</span>
      <span class="win-text">Title</span>
    </div>
    <div class="win-controls">
      <button class="win-btn" data-action="min">—</button>
      <button class="win-btn" data-action="max">▢</button>
      <button class="win-btn danger" data-action="close">✕</button>
    </div>
  </div>

  <div class="win-header">
    <textarea class="win-prompt" rows="2"></textarea>
    <div class="win-actions">
      <button class="btn btn-primary btn-sm" data-action="run">Run</button>
      <button class="btn btn-secondary btn-sm" data-action="save">Save</button>
      <button class="btn btn-ghost btn-sm" data-action="dup">Duplicate</button>
      <div class="menu-wrap">
        <button class="btn btn-ghost btn-sm" data-action="exportMenu">Export ▾</button>
        <div class="menu hidden">
          <button class="menu-item" data-action="exportCsv">CSV</button>
          <button class="menu-item" data-action="exportPdf">PDF</button>
          <button class="menu-item" data-action="exportTxt">Text</button>
        </div>
      </div>
      <button class="btn btn-ghost btn-sm" data-action="delete">Delete</button>
    </div>

    <div class="win-meta muted small">
      <span class="meta-left">Last run: <span class="last-run">never</span></span>
      <span class="meta-right">Rows: <span class="row-count">—</span></span>
    </div>
  </div>

  <div class="win-body">
    <!-- table/chart/summary renderer -->
  </div>

  <div class="win-resize-handle" title="Resize"></div>
</div>
```

---

## 5) Mock API Contract (to implement in Basil later)

Even though this is front-end only now, implement a mock fetch layer in `mock-api.js` that intercepts calls made by `appFetch()`.

### `/api/query` (POST)

Request:

```json
{
  "prompt": "string",
  "kindHint": "table|summary|chart|report",
  "context": { "dateRangeDays": 30, "reseller": "All", "status": "All" },
  "limit": 200,
  "offset": 0,
  "format": "json"
}
```

Response:

```json
{
  "ok": true,
  "artifact": {
    "title": "string",
    "kind": "table|summary|chart|report",
    "result": { ...QueryResult... },
    "sqlPreview": "SELECT ... (mock)",
    "diagnostics": { "tookMs": 123, "source": "mock" }
  }
}
```

Error response:

```json
{ "ok": false, "error": "message" }
```

### `/api/export` (POST)

Request:

```json
{
  "prompt": "string",
  "context": { ... },
  "exportType": "csv|pdf|txt",
  "artifactId": "optional",
  "format": "json"
}
```

Response (async job model):

```json
{
  "ok": true,
  "job": {
    "jobId": "exp_123",
    "status": "queued|running|done|error",
    "progress": 0.0,
    "downloadUrl": null,
    "message": "string"
  }
}
```

Also implement `/api/export/status?jobId=...` in mock mode:
Returns same job shape with updated progress until done, then sets a mock `downloadUrl` like `downloads/export_exp_123.csv`.

Front-end behavior:

* Export opens/creates an `export_job` window showing job progress and a download link when ready.

---

## 6) Mock Data (Hard-coded but realistic)

In `/web/data/mock-data.js`, create realistic fake datasets:

* `customers` (at least 120 sample rows)

    * id, fullName, phone, email (masked), city, state
    * reseller, lawFirm, carrier
    * flags: noShow, missingInfo, missingPaperwork
    * createdAt, lastUpdated

* `appointments` (at least 200 rows)

    * id, customerId, apptDate, apptType ("Office"/"Home"), status
    * chiroName, doctorName
    * coverageAmount, referralOwed
    * notes

* `carriers`, `resellers`, `chiropractors`, `doctors`, `attorneys` arrays (10–20 each)

Also create “status enums”:

* `Scheduled`, `Completed`, `No Show`, `Missing Info`, `Missing Paperwork`, `Needs Review`, `Approved`, `Declined`.

### Mock KPI computations

In dashboard.js, compute KPIs from the mock data and display in KPI cards:

KPI cards (at least 8):

* Total customers
* Appointments in range
* No-show rate
* Missing paperwork count
* Approved vs Declined (ratio)
* Total coverage amount (sum in range)
* Total referral owed (sum in range)
* Average days to decision (mock computed)

Each KPI card includes:

* Big number
* Small delta (“+4.2% vs previous period” mocked)
* Clicking a KPI opens a relevant window (table or chart).

### Trend tiles (mock charts)

Create 4 tiles:

* Appointments per week (line)
* No-shows per week (line)
* Status distribution (bar)
* Referral owed by reseller (bar)

You can implement lightweight charts with pure HTML (bar charts using divs) for now—no external libs.

### Activity feed

Show ~12 mock events:

* “Export completed: No-shows Jan”
* “Saved artifact: Missing paperwork by chiro”
* “Ran query: approvals last 30 days”
* timestamps “5m ago / 2h ago / yesterday” etc.

### Suggested Questions chips

At least 10 chips. Clicking a chip should:

* Create a window with that prompt
* Auto-run and show result.

Examples:

* “No-shows last 30 days by chiropractor”
* “Missing paperwork by reseller”
* “Office vs Home visits trend”
* “Top 10 law firms by volume”
* “Average referral owed by carrier”
* “Approvals vs declines by doctor”
* “Upcoming appointments next 7 days”
* “Customers with missing info AND scheduled tomorrow”
* “Total coverage approved this month”
* “Show me outliers: referral owed > $X”

---

## 7) Mock Query Engine (very important)

In `mock-api.js`, implement a simple “prompt router”:

* If prompt contains keywords:

    * “no-show” → return table + optional chart
    * “missing paperwork” → return grouped table by reseller/chiro
    * “trend” or “per week” → return chart result
    * “summary” or “explain” → return summary result
    * “top” + entity → return top-N table
    * default → return a summary plus a small table

Return consistent, believable data. Include `sqlPreview` string as a mock.

Also enforce:

* Always apply `context.dateRangeDays` filter (mock: filter appointments by date).
* Apply reseller/status filters when set.

---

## 8) Export behavior (mock)

When user clicks Export:

* Create an export job via mock API
* Show progress updates every 400ms in an `export_job` window:

    * progress bar
    * status text
    * “Download” button when ready (link to a generated blob URL)
* For CSV export:

    * If window has a table result, export that table.
* For TXT export:

    * export prompt + summary text.
* For PDF export:

    * just generate a “fake PDF” placeholder blob with text “PDF export placeholder” (real PDF later on Basil server).

---

## 9) Theme toggle (dark/light)

Implement in `app.js`:

* `getTheme()` from localStorage `emcDesk.theme`
* `setTheme(theme)` updates `<html data-theme="...">`
* Toggle button updates and persists
* Default theme: dark
* Add small toast notification “Theme: Dark” / “Theme: Light”

---

## 10) Auth guard

In `dashboard.js`:

* On load, if `localStorage.authToken` missing → redirect to `index.html`.
  In `login.js`:
* On successful login, set token; redirect.

---

## 11) Documentation

Create `/docs/UI_PROTOTYPE.md` with:

* What this prototype is
* How to run (static server recommended)
* Demo walkthrough steps:

    * login
    * run prompt from top search
    * open desktop
    * drag/resize/minimize windows
    * edit prompt and re-run
    * export csv/text/pdf placeholder
    * refresh page → persistence restored
    * toggle theme

Also include notes: where to wire Basil endpoints later.

---

## 12) Quality / Polish requirements

* Must look good on 1440px wide desktop.
* Must remain usable on 1024px.
* Desktop area should not scroll; windows scroll internally.
* All UI components must be keyboard accessible where reasonable.
* No console errors.

---

## Acceptance Criteria

* Login works, theme works, auth guard works.
* Dashboard displays KPIs, trend tiles, activity feed, suggested question chips.
* Desktop has window manager with drag/resize/minimize/maximize/close.
* Windows persist across refresh via localStorage.
* Run prompt creates windows and displays mock results.
* Export creates async job window; CSV/TXT generate downloadable files; PDF placeholder works.
* “Save” stores artifact into localStorage and shows it under “Saved” in sidebar.

---

If you need placeholder icons, use emojis and/or inline SVG. Keep code modular and commented.

---

When Junie is done, we should be able to demo:

1. Login → Dashboard → Open Desktop
2. Click suggested question → window opens with table
3. Drag it; minimize; restore
4. Edit prompt; rerun; export CSV; refresh page; window persists
5. Toggle dark/light theme

**Implement now.**

