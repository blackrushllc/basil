/**
 * dashboard.js - Dashboard + Window Manager
 */

document.addEventListener('DOMContentLoaded', () => {
  // --- Auth Guard ---
  if (!storage.get('localStorage.authToken')) {
    window.location.href = 'index.html';
    return;
  }

  // --- State ---
  let workspace = storage.get(EMC_CONFIG.storageKeyWorkspace, {
    activeView: 'dashboard',
    windows: [],
    zCounter: 100,
    settings: { theme: 'dark', snap: true }
  });

  let savedArtifacts = storage.get(EMC_CONFIG.storageKeySaved, []);

  // --- Initialization ---
  loadKPIs();
  renderActivityFeed();
  renderSuggestedQuestions();
  renderFilters();
  renderSidebarSaved();
  updateView();
  
  // Render existing windows
  workspace.windows.forEach(win => {
    if (!win.minimized) createWinElement(win);
    renderTaskPill(win);
  });

  // --- Event Listeners ---
  on('openDesktop', 'click', () => { workspace.activeView = 'desktop'; saveState(); updateView(); });
  on('backToDashboard', 'click', () => { workspace.activeView = 'dashboard'; saveState(); updateView(); });
  on('resetDesktop', 'click', () => {
    if (confirm('Reset all windows?')) {
      workspace.windows = [];
      document.getElementById('desktop').innerHTML = '';
      document.getElementById('taskbar').innerHTML = '';
      saveState();
    }
  });

  on('runPrompt', 'click', () => {
    const prompt = el('globalPrompt').value;
    if (prompt) createQueryWindow(prompt);
  });

  on('logout', 'click', () => {
    storage.remove('localStorage.authToken');
    window.location.href = 'index.html';
  });

  on('recalculateKpis', 'click', () => fetchKPIs(true));

  on('editAiKnowledge', 'click', async () => {
    const resp = await appFetch('/api/teleemc-dashboard/cgi/api/knowledge.basil');
    const initial = resp && resp.ok ? (resp.knowledge || '') : '';
    openKnowledgeModal(initial);
  });

  function openKnowledgeModal(initialText = '') {
    const backdrop = document.createElement('div');
    backdrop.className = 'modal-backdrop';
    backdrop.id = 'aiKnowledgeModal';

    const dialog = document.createElement('div');
    dialog.className = 'modal';

    const header = document.createElement('div');
    header.className = 'modal-header';
    header.innerHTML = '<div>AI Onboarding — Knowledge Sheet</div>';

    const body = document.createElement('div');
    body.className = 'modal-body';

    const textarea = document.createElement('textarea');
    textarea.value = initialText || '';
    textarea.setAttribute('aria-label', 'AI Onboarding knowledge');
    body.appendChild(textarea);

    const footer = document.createElement('div');
    footer.className = 'modal-footer';

    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-secondary';
    cancelBtn.textContent = 'Cancel';

    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = 'Save';

    footer.appendChild(cancelBtn);
    footer.appendChild(saveBtn);

    dialog.appendChild(header);
    dialog.appendChild(body);
    dialog.appendChild(footer);
    backdrop.appendChild(dialog);
    document.body.appendChild(backdrop);

    // Focus textarea at end
    setTimeout(() => {
      textarea.focus();
      try {
        textarea.selectionStart = textarea.value.length;
        textarea.selectionEnd = textarea.value.length;
      } catch (e) {}
    }, 0);

    function close() {
      window.removeEventListener('keydown', onKey);
      backdrop.remove();
    }

    async function save() {
      saveBtn.disabled = true;
      cancelBtn.disabled = true;
      try {
        const saveResp = await appFetch('/api/teleemc-dashboard/cgi/api/knowledge.basil', {
          method: 'POST',
          body: JSON.stringify({ knowledge: textarea.value })
        });
        if (saveResp && saveResp.ok) {
          showToast('AI Onboarding updated!');
          close();
        } else {
          showToast('Failed to save. Please try again.');
          saveBtn.disabled = false;
          cancelBtn.disabled = false;
        }
      } catch (e) {
        console.error('Failed saving knowledge', e);
        showToast('Error saving.');
        saveBtn.disabled = false;
        cancelBtn.disabled = false;
      }
    }

    function onKey(e) {
      if (e.key === 'Escape') close();
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        save();
      }
    }

    backdrop.addEventListener('click', (e) => {
      if (e.target === backdrop) close();
    });
    cancelBtn.addEventListener('click', close);
    saveBtn.addEventListener('click', save);
    window.addEventListener('keydown', onKey);
  }

  on('userBtn', 'click', () => el('userDropdown').classList.toggle('hidden'));
  on('newWindowMenu', 'click', () => createQueryWindow('New Query'));

  on('newTableWin', 'click', () => createQueryWindow('Show customers', 'table'));
  on('newSummaryWin', 'click', () => createQueryWindow('Explain recent trends', 'summary'));
  on('newChartWin', 'click', () => createQueryWindow('Trend of appointments', 'chart'));

  // --- Dashboard Logic ---
  async function loadKPIs() {
    const cached = storage.get('emcDesk.kpis');
    if (cached) {
      renderKPIs(cached.kpis, cached.timestamp);
    }
    fetchKPIs();
  }

  async function fetchKPIs(isForced = false) {
    if (isForced) showToast('Recalculating KPIs...');
    try {
      const resp = await appFetch('/api/teleemc-dashboard/cgi/api/kpis.basil');
      if (resp.ok) {
        storage.set('emcDesk.kpis', { kpis: resp.kpis, timestamp: resp.timestamp });
        renderKPIs(resp.kpis, resp.timestamp);
        if (isForced) showToast('KPIs updated');
      }
    } catch (e) {
      console.error('Failed to fetch KPIs', e);
    }
  }

  function renderKPIs(kpis, timestamp) {
    if (!kpis) return;
    const grid = el('kpiGrid');
    grid.innerHTML = kpis.map(k => `
      <div class="kpi-card">
        <div class="muted small">${k.label}</div>
        <div class="value">${k.value || 0}</div>
        <div class="delta small muted">Last updated: ${timestamp || 'now'}</div>
      </div>
    `).join('');

    // Trend tiles (keep static for now as mock)
    const trendGrid = el('trendTiles');
    trendGrid.innerHTML = `
      <div class="chart-tile">
        <h4>Appointments per week</h4>
        <div class="bar-chart">
          ${[40, 60, 45, 80, 55, 70].map(v => `<div class="bar-item" style="height: ${v}%"></div>`).join('')}
        </div>
      </div>
      <div class="chart-tile">
        <h4>No-Shows per week</h4>
        <div class="bar-chart">
          ${[10, 15, 8, 20, 12, 18].map(v => `<div class="bar-item" style="height: ${v}%"></div>`).join('')}
        </div>
      </div>
    `;
  }

  function renderActivityFeed() {
    const feed = el('activityFeed');
    const events = [
      { text: 'Export completed: No-shows Jan', time: '5m ago' },
      { text: 'Saved artifact: Missing paperwork by chiro', time: '2h ago' },
      { text: 'Ran query: approvals last 30 days', time: 'Yesterday' },
      { text: 'New customer: CUST-1045', time: 'Yesterday' }
    ];
    feed.innerHTML = events.map(e => `
      <div class="feed-item">
        <div>${e.text}</div>
        <div class="muted small">${e.time}</div>
      </div>
    `).join('');
  }

  function renderSuggestedQuestions() {
    const questions = [
      "No-shows last 30 days by chiropractor",
      "Missing paperwork by reseller",
      "Office vs Home visits trend",
      "Top 10 law firms by volume",
      "Average referral owed by carrier",
      "Approvals vs declines by doctor"
    ];
    const chips = el('suggestedQuestions');
    chips.innerHTML = questions.map(q => `<div class="chip">${q}</div>`).join('');
    chips.querySelectorAll('.chip').forEach(chip => {
      chip.addEventListener('click', () => createQueryWindow(chip.textContent));
    });
  }

  function renderFilters() {
    const resellers = ['All', ...window.MOCK_DATA.resellers];
    el('resellerFilter').innerHTML = resellers.map(r => `<option value="${r}">${r}</option>`).join('');
    const statuses = ['All', ...window.MOCK_DATA.statuses];
    el('statusFilter').innerHTML = statuses.map(s => `<option value="${s}">${s}</option>`).join('');
  }

  function renderSidebarSaved() {
    const list = el('savedArtifacts');
    list.innerHTML = savedArtifacts.map(a => `
      <div class="menu-item small" style="padding: 0.25rem 0.5rem">
        <span>${a.title}</span>
      </div>
    `).join('');
  }

  function updateView() {
    if (workspace.activeView === 'dashboard') {
      el('dashboardView').classList.remove('hidden');
      el('desktopView').classList.add('hidden');
    } else {
      el('dashboardView').classList.add('hidden');
      el('desktopView').classList.remove('hidden');
    }
  }

  // --- Window Manager ---

  function createQueryWindow(prompt, kindHint = 'table') {
    const id = 'win-' + Math.random().toString(36).substr(2, 9);
    const win = {
      id,
      type: kindHint,
      title: prompt,
      prompt: prompt,
      x: 100 + (workspace.windows.length * 24),
      y: 100 + (workspace.windows.length * 24),
      w: 560,
      h: 360,
      z: ++workspace.zCounter,
      minimized: false,
      maximized: false,
      lastResult: null
    };

    workspace.windows.push(win);
    workspace.activeView = 'desktop';
    saveState();
    updateView();
    createWinElement(win);
    renderTaskPill(win);
    runQuery(win.id);
  }

  function createWinElement(win) {
    const desktop = el('desktop');
    const div = document.createElement('div');
    div.className = 'win' + (win.maximized ? ' maximized' : '');
    div.dataset.winId = win.id;
    div.style.left = win.x + 'px';
    div.style.top = win.y + 'px';
    div.style.width = win.w + 'px';
    div.style.height = win.h + 'px';
    div.style.zIndex = win.z;

    div.innerHTML = `
      <div class="win-titlebar">
        <div class="win-title">
          <span class="win-icon">${getIcon(win.type)}</span>
          <span class="win-text">${win.title}</span>
        </div>
        <div class="win-controls">
          <button class="win-btn" data-action="min">—</button>
          <button class="win-btn" data-action="max">▢</button>
          <button class="win-btn danger" data-action="close">✕</button>
        </div>
      </div>
      <div class="win-header">
        <textarea class="win-prompt" rows="2">${win.prompt}</textarea>
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
          <span class="meta-left">Last run: <span class="last-run">${win.lastRunAt || 'never'}</span></span>
          <span class="meta-right">Rows: <span class="row-count">${win.lastResult?.rowCount || '—'}</span></span>
        </div>
      </div>
      <div class="win-body">
        ${renderResult(win.lastResult)}
      </div>
      <div class="win-resize-handle" title="Resize"></div>
    `;

    desktop.appendChild(div);
    attachWinEvents(div, win);
  }

  function attachWinEvents(el, win) {
    const titlebar = el.querySelector('.win-titlebar');
    
    // Dragging
    titlebar.onmousedown = (e) => {
      if (e.target.closest('.win-controls')) return;
      focusWin(win.id);
      let startX = e.clientX;
      let startY = e.clientY;
      let startLeft = win.x;
      let startTop = win.y;

      document.onmousemove = (e) => {
        if (win.maximized) return;
        win.x = startLeft + (e.clientX - startX);
        win.y = startTop + (e.clientY - startY);
        el.style.left = win.x + 'px';
        el.style.top = win.y + 'px';
      };
      document.onmouseup = () => {
        document.onmousemove = null;
        saveState();
      };
    };

    // Resizing
    const handle = el.querySelector('.win-resize-handle');
    handle.onmousedown = (e) => {
      e.stopPropagation();
      let startW = win.w;
      let startH = win.h;
      let startX = e.clientX;
      let startY = e.clientY;

      document.onmousemove = (e) => {
        win.w = Math.max(320, startW + (e.clientX - startX));
        win.h = Math.max(200, startH + (e.clientY - startY));
        el.style.width = win.w + 'px';
        el.style.height = win.h + 'px';
      };
      document.onmouseup = () => {
        document.onmousemove = null;
        saveState();
      };
    };

    // Click to focus
    el.onclick = () => focusWin(win.id);

    // Actions
    el.addEventListener('click', (e) => {
      const action = e.target.dataset.action;
      if (action === 'close' || action === 'delete') {
        workspace.windows = workspace.windows.filter(w => w.id !== win.id);
        el.remove();
        document.querySelector(`[data-task-id="${win.id}"]`)?.remove();
        saveState();
      } else if (action === 'min') {
        win.minimized = true;
        el.remove();
        saveState();
      } else if (action === 'max') {
        win.maximized = !win.maximized;
        el.classList.toggle('maximized');
        saveState();
      } else if (action === 'dup') {
        createQueryWindow(win.prompt, win.type);
      } else if (action === 'run') {
        win.prompt = el.querySelector('.win-prompt').value;
        runQuery(win.id);
      } else if (action === 'save') {
        saveArtifact(win);
      } else if (action === 'exportMenu') {
        el.querySelector('.menu').classList.toggle('hidden');
      } else if (action?.startsWith('export')) {
        runExport(win, action.replace('export', '').toLowerCase());
      }
    });

    // Double click title to maximize
    titlebar.ondblclick = () => {
      win.maximized = !win.maximized;
      el.classList.toggle('maximized');
      saveState();
    };
  }

  function renderTaskPill(win) {
    let pill = document.querySelector(`[data-task-id="${win.id}"]`);
    if (!pill) {
      pill = document.createElement('div');
      pill.className = 'task-pill';
      pill.dataset.taskId = win.id;
      el('taskbar').appendChild(pill);
      pill.onclick = () => {
        if (win.minimized) {
          win.minimized = false;
          createWinElement(win);
        }
        focusWin(win.id);
        saveState();
      };
    }
    pill.textContent = win.title;
  }

  function focusWin(id) {
    const win = workspace.windows.find(w => w.id === id);
    if (win) {
      win.z = ++workspace.zCounter;
      const winEl = document.getElementById(id);
      if (winEl) winEl.style.zIndex = win.z;
      saveState();
    }
  }

  async function runQuery(id) {
    const win = workspace.windows.find(w => w.id === id);
    const winEl = document.querySelector(`[data-win-id="${id}"]`);
    if (!winEl) return;

    const body = winEl.querySelector('.win-body');
    body.innerHTML = '<div class="muted">Thinking...</div>';

    try {
      const resp = await appFetch('/api/teleemc-dashboard/cgi/api/query.basil', {
        method: 'POST',
        body: JSON.stringify({
          prompt: win.prompt,
          kind: win.type,
          context: {
            dateRangeDays: el('dateRange').value,
            reseller: el('resellerFilter').value,
            status: el('statusFilter').value
          }
        })
      });

      if (resp.ok) {
        // If the API returns rows directly, we wrap it in a result object
        win.lastResult = {
          kind: win.type || 'table',
          rows: resp.rows || [],
          columns: resp.rows?.length ? Object.keys(resp.rows[0]).map(k => ({key: k, label: k})) : [],
          rowCount: resp.rows?.length || 0,
          sql: resp.sql,
          artifact_id: resp.artifact_id
        };
        win.title = win.prompt; // Or get a better title from AI if we had a two-pass
        win.lastRunAt = new Date().toLocaleTimeString();
        winEl.querySelector('.win-text').textContent = win.title;
        winEl.querySelector('.last-run').textContent = win.lastRunAt;
        winEl.querySelector('.row-count').textContent = win.lastResult.rowCount || '0';
        body.innerHTML = renderResult(win.lastResult);
        saveState();
        renderTaskPill(win);
      } else {
        body.innerHTML = `<div class="alert alert-danger">${resp.error || 'Unknown error'}</div>`;
      }
    } catch (e) {
      body.innerHTML = `<div class="alert alert-danger">Fetch failed: ${e.message}</div>`;
    }
  }

  function renderResult(res) {
    if (!res) return '<div class="muted">No data. Click Run.</div>';
    
    // Auto-detect kind if not specified but we have rows
    const kind = res.kind || (res.rows ? 'table' : 'summary');

    if (kind === 'table') {
      if (!res.rows || res.rows.length === 0) return '<div class="muted">Empty result set.</div>';
      const cols = res.columns || Object.keys(res.rows[0]).map(k => ({key: k, label: k}));
      return `
        <table>
          <thead>
            <tr>${cols.map(c => `<th>${c.label}</th>`).join('')}</tr>
          </thead>
          <tbody>
            ${res.rows.map(r => `<tr>${cols.map(c => `<td>${r[c.key] !== null ? r[c.key] : ''}</td>`).join('')}</tr>`).join('')}
          </tbody>
        </table>
      `;
    }
    if (kind === 'summary') {
      return `
        <div class="summary-text">${res.text || 'No summary available.'}</div>
        ${res.bullets ? `<ul class="summary-bullets">${res.bullets.map(b => `<li>${b}</li>`).join('')}</ul>` : ''}
      `;
    }
    if (kind === 'chart') {
      // Basic mock chart if real data isn't mapped to chart series yet
      const values = res.series ? res.series[0].values : [30, 50, 40, 60];
      const labels = res.labels || ['A', 'B', 'C', 'D'];
      return `
        <div class="bar-chart" style="height: 150px">
          ${values.map((v, i) => `
            <div class="bar-item" style="height: ${v}%">
              <span class="bar-label">${labels[i] || ''}</span>
            </div>
          `).join('')}
        </div>
      `;
    }
    return '<div class="muted">Unknown result type.</div>';
  }

  async function runExport(win, format) {
    if (!win.lastResult || !win.lastResult.artifact_id) {
      showToast('Run query first to generate artifact');
      return;
    }
    const aid = win.lastResult.artifact_id;
    const url = `/api/teleemc-dashboard/cgi/api/export.basil`;
    
    showToast('Preparing export...');
    
    try {
      const token = localStorage.getItem(EMC_CONFIG.storageKeyToken);
      const headers = { 'Content-Type': 'application/json' };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const response = await fetch(url, {
        method: 'POST',
        headers: headers,
        body: JSON.stringify({ artifact_id: aid, format: format })
      });
      
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        alert('Export failed: ' + (err.error || response.statusText));
        return;
      }
      
      const blob = await response.blob();
      const downloadUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = downloadUrl;
      a.download = `teleemc_${aid}.${format}`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      showToast('Export complete');
    } catch (e) {
      alert('Export error: ' + e.message);
    }
  }

  function saveArtifact(win) {
    const artifact = {
      id: 'art-' + Date.now(),
      title: win.title,
      prompt: win.prompt,
      createdAt: new Date().toISOString()
    };
    savedArtifacts.push(artifact);
    storage.set(EMC_CONFIG.storageKeySaved, savedArtifacts);
    renderSidebarSaved();
    showToast('Artifact saved!');
  }

  function saveState() {
    storage.set(EMC_CONFIG.storageKeyWorkspace, workspace);
  }

  function getIcon(type) {
    const icons = { table: '📊', chart: '📈', summary: '📝', export_job: '📦' };
    return icons[type] || '📄';
  }
});
