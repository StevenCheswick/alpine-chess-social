// ══════════════════════════════════════════
// AUTH + INIT
// ══════════════════════════════════════════
function showApp() {
  document.getElementById('login-page').style.display = 'none';
  document.getElementById('app-shell').style.display = 'flex';
  document.getElementById('mobile-nav').style.display = '';
  let page = location.pathname.slice(1) || 'dashboard';
  if (page === 'tactics') page = 'puzzles';
  switchPage(page, true);
}

function showLogin() {
  document.getElementById('login-page').style.display = 'flex';
  document.getElementById('app-shell').style.display = 'none';
}

document.getElementById('login-form').addEventListener('submit', async (e) => {
  e.preventDefault();
  const btn = document.getElementById('login-btn');
  const errEl = document.getElementById('login-error');
  const username = document.getElementById('login-username').value;
  const password = document.getElementById('login-password').value;

  btn.textContent = 'Signing in...';
  btn.disabled = true;
  errEl.classList.add('hidden');

  try {
    const res = await fetch(API_URL + '/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.detail || 'Login failed');
    }
    const data = await res.json();
    localStorage.setItem('alpine_token', data.token);
    localStorage.setItem('alpine_user', JSON.stringify(data.user));
    showApp();
  } catch (err) {
    errEl.textContent = err.message;
    errEl.classList.remove('hidden');
  } finally {
    btn.textContent = 'Sign in';
    btn.disabled = false;
  }
});

// Console-only registration: registerUser('username', 'password')
window.registerUser = async function(username, password) {
  try {
    const res = await fetch(API_URL + '/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.detail || 'Registration failed');
    console.log('Registered successfully:', data.user);
    return data;
  } catch (err) {
    console.error('Registration error:', err.message);
  }
};

// Check for existing session — validate token with /api/auth/me
(async function checkSession() {
  const token = localStorage.getItem('alpine_token');
  if (!token) { showLogin(); return; }

  try {
    const res = await fetch(API_URL + '/api/auth/me', {
      headers: { 'Authorization': 'Bearer ' + token },
    });
    if (!res.ok) throw new Error('Invalid token');
    const data = await res.json();
    if (data.user) localStorage.setItem('alpine_user', JSON.stringify(data.user));
    showApp();
  } catch {
    localStorage.removeItem('alpine_token');
    localStorage.removeItem('alpine_user');
    showLogin();
  }
})();

// Navigate to path on initial load (must be after all variable declarations)
(function() {
  let page = location.pathname.slice(1) || 'dashboard';
  if (page === 'tactics') page = 'puzzles';
  if (document.getElementById('page-' + page)) switchPage(page, true);
})();
