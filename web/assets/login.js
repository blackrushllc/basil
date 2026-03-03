/**
 * login.js - Handles prototype login behavior
 */

document.addEventListener('DOMContentLoaded', () => {
  const loginForm = el('loginForm');
  const loginError = el('loginError');

  if (loginForm) {
    loginForm.addEventListener('submit', async (e) => {
      e.preventDefault();
      loginError.classList.add('hidden');

      const user = el('username').value;
      const pass = el('password').value;

      try {
        const response = await fetch('api/login.basil', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ username: user, password: pass })
        });

        const data = await response.json();

        if (data.ok) {
          storage.set('localStorage.authToken', data.token);
          window.location.href = 'dashboard.html';
        } else {
          loginError.textContent = data.error || 'Invalid email or password.';
          loginError.classList.remove('hidden');
        }
      } catch (err) {
        console.error('Login error:', err);
        loginError.textContent = 'Authentication service unavailable.';
        loginError.classList.remove('hidden');
      }
    });
  }
});
