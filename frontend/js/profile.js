// ══════════════════════════════════════════
// PROFILE PAGE INIT
// ══════════════════════════════════════════
let _currentProfile = null; // cache for edit modal

async function initProfile() {
  window._profileInit = true;
  const user = JSON.parse(localStorage.getItem('alpine_user') || '{}');
  const token = localStorage.getItem('alpine_token');
  if (!user.username) return;

  // Fetch profile from API for game count
  let profile = null;
  try {
    const res = await fetch(API_URL + `/api/users/${user.username}`, { headers: { 'Authorization': 'Bearer ' + token } });
    if (res.ok) profile = await res.json();
  } catch {}

  _currentProfile = profile;

  renderProfile(user, profile);
}

function renderProfile(user, profile) {
  const displayName = profile?.displayName || user.displayName || user.username;
  const bio = profile?.bio || user.bio || '';
  const gamesCount = profile?.gamesCount || 0;
  const chessComUser = profile?.chessComUsername || user.chessComUsername || '';
  const initials = displayName.slice(0, 2).toUpperCase();

  document.getElementById('profileCard').innerHTML = `
    <div class="flex flex-col sm:flex-row items-center sm:items-start gap-4 sm:gap-5">
      <div class="w-20 h-20 rounded-xl bg-gradient-to-br from-sky-300 via-sky-400 to-cyan-500 flex items-center justify-center text-2xl font-bold text-black shadow-lg shadow-sky-500/20 shrink-0">${initials}</div>
      <div class="flex-1 min-w-0 text-center sm:text-left">
        <div class="flex items-center justify-center sm:justify-start gap-3 flex-wrap">
          <h1 class="text-xl font-bold text-white">${displayName}</h1>
          <button onclick="openEditProfile()"
            class="px-3 py-1.5 rounded-lg text-label font-medium text-muted border border-slate-700 hover:bg-slate-800 hover:text-white transition-colors">
            Edit Profile
          </button>
        </div>
        <p class="text-body text-secondary mt-0.5">@${user.username}</p>
        ${bio ? `<p class="text-body text-secondary mt-2 leading-relaxed">${esc(bio)}</p>` : ''}
        <div class="flex items-center justify-center sm:justify-start gap-4 sm:gap-6 mt-4 flex-wrap">
          <div>
            <p class="text-lg font-bold font-mono text-white">${gamesCount}</p>
            <p class="text-meta text-secondary uppercase tracking-wider">Games</p>
          </div>
          ${chessComUser ? `
          <div class="hidden sm:block w-px h-8 bg-slate-800"></div>
          <div class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-md bg-slate-900/80 border border-slate-800/60">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="var(--text-muted)" stroke-width="1.5"/><path d="M8 12l3 3 5-5" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            <span class="text-label text-muted">Chess.com</span>
            <span class="text-label font-mono text-white font-medium">${esc(chessComUser)}</span>
          </div>` : `
          <div class="hidden sm:block w-px h-8 bg-slate-800"></div>
          <button onclick="openEditProfile()"
            class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-md bg-slate-900/80 border border-slate-800/60 hover:bg-slate-800 transition-colors cursor-pointer">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
            <span class="text-label text-muted">Link Chess.com</span>
          </button>`}
        </div>
      </div>
    </div>

    <!-- Logout -->
    <div class="mt-5 pt-4" style="border-top:1px solid var(--border-subtle)">
      <button onclick="logoutUser()"
        class="flex items-center gap-2 px-3 py-2 rounded-lg text-label font-medium text-muted hover:text-white hover:bg-slate-800 transition-colors">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
        Sign Out
      </button>
    </div>`;
}

// HTML escape helper
function esc(str) {
  const d = document.createElement('div');
  d.textContent = str;
  return d.innerHTML;
}

// ── Edit Profile Modal ──
function openEditProfile() {
  const user = JSON.parse(localStorage.getItem('alpine_user') || '{}');
  const profile = _currentProfile;
  const displayName = profile?.displayName || user.displayName || user.username;
  const bio = profile?.bio || user.bio || '';
  const chessComUser = profile?.chessComUsername || user.chessComUsername || '';

  document.getElementById('edit-displayName').value = displayName;
  document.getElementById('edit-bio').value = bio;
  document.getElementById('edit-chesscom').value = chessComUser;
  document.getElementById('edit-displayName-count').textContent = displayName.length;
  document.getElementById('edit-bio-count').textContent = bio.length;
  document.getElementById('edit-profile-error').classList.add('hidden');
  hideDeleteConfirm();
  document.getElementById('edit-profile-modal').style.display = 'flex';
}

function closeEditProfile() {
  document.getElementById('edit-profile-modal').style.display = 'none';
}

// Character counters
document.getElementById('edit-displayName').addEventListener('input', function() {
  document.getElementById('edit-displayName-count').textContent = this.value.length;
});
document.getElementById('edit-bio').addEventListener('input', function() {
  document.getElementById('edit-bio-count').textContent = this.value.length;
});

// Save profile
document.getElementById('edit-profile-form').addEventListener('submit', async (e) => {
  e.preventDefault();
  const errEl = document.getElementById('edit-profile-error');
  const saveBtn = document.getElementById('edit-profile-save-btn');
  errEl.classList.add('hidden');

  const displayName = document.getElementById('edit-displayName').value.trim();
  const bio = document.getElementById('edit-bio').value.trim();
  const chessComUsername = document.getElementById('edit-chesscom').value.trim();

  if (!displayName) {
    errEl.textContent = 'Display name cannot be empty';
    errEl.classList.remove('hidden');
    return;
  }

  const token = localStorage.getItem('alpine_token');
  const user = JSON.parse(localStorage.getItem('alpine_user') || '{}');
  const profile = _currentProfile;

  // Only include changed fields
  const data = {};
  if (displayName !== (profile?.displayName || user.displayName || user.username)) data.displayName = displayName;
  if (bio !== (profile?.bio || '')) data.bio = bio;
  if (chessComUsername !== (profile?.chessComUsername || '')) data.chessComUsername = chessComUsername;

  if (Object.keys(data).length === 0) {
    closeEditProfile();
    return;
  }

  saveBtn.textContent = 'Saving...';
  saveBtn.disabled = true;

  try {
    const res = await fetch(API_URL + '/api/users/me', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
      body: JSON.stringify(data),
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.detail || 'Failed to update profile');
    }
    const updated = await res.json();
    _currentProfile = updated;

    // Update local storage
    user.displayName = updated.displayName;
    user.bio = updated.bio;
    user.chessComUsername = updated.chessComUsername;
    localStorage.setItem('alpine_user', JSON.stringify(user));

    // If Chess.com was newly linked, trigger game sync in background
    const wasLinked = profile?.chessComUsername;
    if (!wasLinked && data.chessComUsername) {
      fetch(API_URL + '/api/games/sync', {
        method: 'POST',
        headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
        body: '{}',
      })
        .then(() => console.log('Initial games sync started'))
        .catch(err => console.error('Failed to start games sync:', err));
    }

    renderProfile(user, updated);
    closeEditProfile();
  } catch (err) {
    errEl.textContent = err.message;
    errEl.classList.remove('hidden');
  } finally {
    saveBtn.textContent = 'Save Changes';
    saveBtn.disabled = false;
  }
});

// ── Delete Account ──
function showDeleteConfirm() {
  document.getElementById('delete-account-section').classList.add('hidden');
  document.getElementById('delete-confirm-section').classList.remove('hidden');
}

function hideDeleteConfirm() {
  document.getElementById('delete-account-section').classList.remove('hidden');
  document.getElementById('delete-confirm-section').classList.add('hidden');
}

async function confirmDeleteAccount() {
  const btn = document.getElementById('confirm-delete-btn');
  const errEl = document.getElementById('edit-profile-error');
  btn.textContent = 'Deleting...';
  btn.disabled = true;
  errEl.classList.add('hidden');

  try {
    const token = localStorage.getItem('alpine_token');
    const res = await fetch(API_URL + '/api/users/me', {
      method: 'DELETE',
      headers: { 'Authorization': 'Bearer ' + token },
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.detail || 'Failed to delete account');
    }
    logoutUser();
  } catch (err) {
    errEl.textContent = err.message;
    errEl.classList.remove('hidden');
    btn.textContent = 'Confirm Delete';
    btn.disabled = false;
  }
}

// ── Logout ──
function logoutUser() {
  localStorage.removeItem('alpine_token');
  localStorage.removeItem('alpine_user');
  _currentProfile = null;
  window._dashInit = false;
  window._puzzlesInit = false;
  window._endgamesInit = false;
  window._trainerInit = false;
  window._gamesInit = false;
  window._analysisInit = false;
  window._profileInit = false;
  _gamesCache = {};
  closeEditProfile();
  showLogin();
}
