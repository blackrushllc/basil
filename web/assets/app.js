/**
 * app.js - Shared utilities for EMC Desk
 */

const EMC_CONFIG = {
  storageKeyTheme: 'emcDesk.theme',
  storageKeyToken: 'localStorage.authToken',
  storageKeyWorkspace: 'emcDesk.workspace.v1',
  storageKeySaved: 'emcDesk.savedArtifacts.v1'
};

// --- Theme Management ---
function getTheme() {
  return localStorage.getItem(EMC_CONFIG.storageKeyTheme) || 'dark';
}

function setTheme(theme) {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem(EMC_CONFIG.storageKeyTheme, theme);
  showToast(`Theme: ${theme.charAt(0).toUpperCase() + theme.slice(1)}`);
}

function initTheme() {
  const theme = getTheme();
  document.documentElement.setAttribute('data-theme', theme);
}

function toggleTheme() {
  const current = getTheme();
  setTheme(current === 'dark' ? 'light' : 'dark');
}

// --- Toast Notification ---
function showToast(message, duration = 3000) {
  let container = document.querySelector('.toast-container');
  if (!container) {
    container = document.createElement('div');
    container.className = 'toast-container';
    document.body.appendChild(container);
  }

  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.textContent = message;
  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, duration);
}

// --- Storage Utils ---
const storage = {
  get(key, fallback = null) {
    const val = localStorage.getItem(key);
    if (!val) return fallback;
    try { return JSON.parse(val); } catch(e) { return val; }
  },
  set(key, val) {
    localStorage.setItem(key, typeof val === 'string' ? val : JSON.stringify(val));
  },
  remove(key) {
    localStorage.removeItem(key);
  }
};

// --- Fetch Wrapper ---
async function appFetch(url, options = {}) {
  // In this prototype, we intercept calls for mock-api
  if (window.mockApi && window.mockApi.shouldIntercept(url)) {
    return window.mockApi.handle(url, options);
  }

  const token = localStorage.getItem(EMC_CONFIG.storageKeyToken);
  const headers = {
    'Content-Type': 'application/json',
    ...options.headers
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  // Real fetch (for future)
  const response = await fetch(url, {
    ...options,
    headers: headers
  });
  return response.json();
}

// --- UI Helpers ---
function el(id) { return document.getElementById(id); }

function on(id, event, handler) {
  const element = typeof id === 'string' ? el(id) : id;
  if (element) element.addEventListener(event, handler);
}

// Global Init
document.addEventListener('DOMContentLoaded', () => {
  initTheme();
  on('themeToggle', 'click', toggleTheme);
});
