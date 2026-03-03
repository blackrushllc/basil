# EMC Desk — UI Prototype

This is a front-end-only prototype for the EMC Desk exploration dashboard. It demonstrates the user interface, window management, and mock data interactions intended for the Basil programming language implementation.

## How to Run

1.  **Static Server (Recommended):** Use a simple static server like `npx serve .` or `python -m http.server` from the root directory.
2.  **Direct Open:** You can also open `web/index.html` directly in a modern web browser, though some features like `fetch` might behave differently depending on browser security settings (local storage should work).

## Demo Walkthrough

1.  **Login:**
    *   Navigate to `index.html`.
    *   Click "Fill Demo" or use `owner` / `demo`.
    *   Click "Login".
2.  **Dashboard:**
    *   Observe the KPI cards and trend charts (mocked).
    *   View the recent activity feed.
    *   Click on a "Suggested Question" chip (e.g., "No-shows last 30 days...").
3.  **Workspace Desktop:**
    *   The Desktop view will open automatically when a query is run.
    *   **Window Manager:**
        *   **Drag:** Move windows by grabbing the title bar.
        *   **Resize:** Use the handle at the bottom-right corner.
        *   **Minimize:** Click "—" to move the window to the taskbar. Restore by clicking the taskbar pill.
        *   **Maximize:** Click "▢" or double-click the title bar to toggle full screen.
        *   **Close:** Click "✕" to remove the window.
4.  **Queries & Results:**
    *   Edit the prompt in a window and click "Run" to see different mock results (table, chart, summary).
    *   Use the Global Search at the top to create new windows.
5.  **Exporting:**
    *   Click "Export" in any window and select CSV, PDF, or Text.
    *   A new window will track the progress of the mock export job.
    *   Download the result when complete.
6.  **Persistence:**
    *   Refresh the page. Your desktop layout and open windows will be restored from `localStorage`.
7.  **Theme Toggle:**
    *   Click the 🌓 icon in the top bar to toggle between Light and Dark modes.

## Technical Details

### Project Layout
*   `/web/index.html`: Login page.
*   `/web/dashboard.html`: Main application shell.
*   `/web/assets/app.css`: Unified styling with CSS variables for themes.
*   `/web/assets/app.js`: Shared utilities (storage, theme, fetch).
*   `/web/assets/dashboard.js`: Window manager and dashboard logic.
*   `/web/assets/mock-api.js`: Intercepts API calls to return simulated data.
*   `/web/data/mock-data.js`: Contains the mock datasets (Customers, Appointments).

### Data Model

#### WorkspaceState
```js
{
  activeView: "dashboard" | "desktop",
  windows: WindowState[],
  zCounter: number,
  settings: { theme: "light" | "dark", snap: boolean }
}
```

#### WindowState
```js
{
  id: string,
  type: "table" | "summary" | "chart" | "export_job",
  title: string,
  prompt: string,
  x, y, w, h: number,
  z: number,
  minimized: boolean,
  maximized: boolean,
  lastResult: QueryResult | null
}
```

## Future Basil Integration

To wire this prototype to a real Basil server:
1.  Replace `assets/mock-api.js` with real API endpoints in `appFetch`.
2.  Implement `/api/query` and `/api/export` in Basil.
3.  Implement authentication in Basil and update `login.js`.
4.  Replace hard-coded KPI logic with server-side aggregations.
