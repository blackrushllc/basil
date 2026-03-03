/**
 * mock-api.js - Mock API responder for prototype
 */

var mockApi = (() => {
  const interceptedUrls = [];
  const jobs = new Map();

  function shouldIntercept(url) {
    return interceptedUrls.some(u => url.includes(u));
  }

  async function handle(url, options) {
    const body = options.body ? JSON.parse(options.body) : {};
    console.log(`[Mock API] ${options.method || 'GET'} ${url}`, body);

    // Artificial delay
    await new Promise(r => setTimeout(r, 600));

    if (url.includes('api/query.basil')) {
      return handleQuery(body);
    }
    if (url.includes('api/export.basil')) {
      return handleExport(body);
    }

    return { ok: false, error: 'Not Found' };
  }

  function handleQuery(req) {
    const prompt = (req.prompt || '').toLowerCase();
    const context = req.context || { dateRangeDays: 30, reseller: 'All', status: 'All' };
    
    let result = { kind: 'summary', text: 'I found some data matching your request.' };
    let title = 'Query Result';

    // Basic prompt routing
    if (prompt.includes('no-show')) {
      title = 'No-Show Report';
      const rows = window.MOCK_DATA.appointments
        .filter(a => a.status === 'No Show')
        .slice(0, 20)
        .map(a => ({
          ID: a.id,
          Customer: a.customerId,
          Date: a.apptDate.split('T')[0],
          Provider: a.chiroName,
          Type: a.apptType
        }));
      result = { kind: 'table', columns: Object.keys(rows[0] || {}).map(k => ({key: k, label: k})), rows, rowCount: rows.length };
    } else if (prompt.includes('missing paperwork')) {
      title = 'Missing Paperwork by Reseller';
      const grouped = {};
      window.MOCK_DATA.customers.filter(c => c.flags.missingPaperwork).forEach(c => {
        grouped[c.reseller] = (grouped[c.reseller] || 0) + 1;
      });
      const rows = Object.entries(grouped).map(([name, count]) => ({ Reseller: name, Count: count }));
      result = { kind: 'table', columns: [{key: 'Reseller', label: 'Reseller'}, {key: 'Count', label: 'Count'}], rows, rowCount: rows.length };
    } else if (prompt.includes('trend') || prompt.includes('per week')) {
      title = 'Weekly Appointment Trend';
      result = {
        kind: 'chart',
        chartType: 'line',
        labels: ['Week 1', 'Week 2', 'Week 3', 'Week 4'],
        series: [{ name: 'Appointments', values: [45, 52, 38, 65] }]
      };
    } else if (prompt.includes('top') && prompt.includes('law firm')) {
      title = 'Top Law Firms by Volume';
      const rows = window.MOCK_DATA.lawFirms.slice(0, 5).map((f, i) => ({ Firm: f, Volume: 100 - (i * 15) }));
      result = { kind: 'table', columns: [{key: 'Firm', label: 'Firm'}, {key: 'Volume', label: 'Volume'}], rows, rowCount: rows.length };
    } else {
      // Default fallback
      result = {
        kind: 'summary',
        text: 'Based on your query, here is a quick summary of the recent activity.',
        bullets: [
          'Total records analyzed: 400+',
          'Primary trend: Increasing volume in office visits',
          'Suggested next step: Review missing info for upcoming week'
        ]
      };
    }

    return {
      ok: true,
      artifact: {
        title,
        kind: result.kind,
        result,
        sqlPreview: `SELECT * FROM appointments WHERE status = '...' AND date > NOW() - INTERVAL '${context.dateRangeDays} days' -- (mock)`,
        diagnostics: { tookMs: 123, source: 'mock-engine' }
      }
    };
  }

  function handleExport(req) {
    const jobId = 'exp_' + Math.random().toString(36).substr(2, 9);
    jobs.set(jobId, {
      jobId,
      status: 'queued',
      progress: 0,
      type: req.exportType || 'csv'
    });

    // Start background "processing"
    simulateJob(jobId);

    return { ok: true, job: jobs.get(jobId) };
  }

  function handleExportStatus(jobId) {
    const job = jobs.get(jobId);
    if (!job) return { ok: false, error: 'Job not found' };
    return { ok: true, job };
  }

  function simulateJob(jobId) {
    const job = jobs.get(jobId);
    let p = 0;
    const interval = setInterval(() => {
      p += 0.2;
      if (p >= 1) {
        job.progress = 1;
        job.status = 'done';
        job.downloadUrl = `downloads/mock_export_${jobId}.${job.type}`;
        clearInterval(interval);
      } else {
        job.progress = p;
        job.status = 'running';
      }
    }, 800);
  }

  return { shouldIntercept, handle };
})();

window.mockApi = mockApi;
