// ══════════════════════════════════════════
// MOVE SOUNDS
// ══════════════════════════════════════════
const _soundPool = {};
const _soundNames = ['move-self', 'capture', 'castle', 'move-check', 'promote'];
let _soundsPreloaded = false;

function preloadSounds() {
  if (_soundsPreloaded) return;
  _soundsPreloaded = true;
  for (const name of _soundNames) {
    const a = new Audio(`sounds/${name}.mp3`);
    a.preload = 'auto';
    _soundPool[name] = [a];
  }
}

function playSound(name) {
  if (!name) return;
  preloadSounds();
  const pool = _soundPool[name] || [];
  let audio = pool.find(a => a.paused || a.ended);
  if (!audio) {
    audio = new Audio(`sounds/${name}.mp3`);
    pool.push(audio);
    _soundPool[name] = pool;
  }
  try {
    audio.currentTime = 0;
    audio.play().catch(() => {});
  } catch {}
}

function soundForMove(game, move) {
  if (!move) return null;
  const flags = move.flags || '';
  // chess.js v1 uses isCheck(); older versions used in_check(). Support both.
  const inCheck = game && (
    (typeof game.isCheck === 'function' && game.isCheck()) ||
    (typeof game.in_check === 'function' && game.in_check())
  );
  if (inCheck) return 'move-check';
  if (flags.includes('p') || move.promotion) return 'promote';
  if (flags.includes('c') || flags.includes('e') || move.captured) return 'capture';
  if (flags.includes('k') || flags.includes('q')) return 'castle';
  return 'move-self';
}

// ══════════════════════════════════════════
// RESPONSIVE BOARD RESIZE
// ══════════════════════════════════════════
function resizeBoards() {
  // Resize analysis board
  const cbWrap = document.getElementById('chessboardWrap');
  if (cbWrap) {
    const w = cbWrap.clientWidth;
    const inner = document.getElementById('chessboard');
    if (inner && w > 0) { inner.style.width = w + 'px'; inner.style.height = w + 'px'; }
    // Sync eval bar height to board
    const evalBar = document.getElementById('analysisEvalBar');
    if (evalBar && w > 0) { evalBar.style.height = w + 'px'; }
  }
  // Resize trainer board
  const tbWrap = document.getElementById('trainerBoardWrap');
  if (tbWrap) {
    const w = tbWrap.clientWidth;
    const inner = document.getElementById('trainerBoard');
    if (inner && w > 0) { inner.style.width = w + 'px'; inner.style.height = w + 'px'; }
  }
  // Resize maia board
  const mbWrap = document.getElementById('maiaBoardWrap');
  if (mbWrap) {
    const w = mbWrap.clientWidth;
    const inner = document.getElementById('maiaBoard');
    if (inner && w > 0) { inner.style.width = w + 'px'; inner.style.height = w + 'px'; }
  }
  // Redraw active Chessground instances
  if (_cgInstance) _cgInstance.redrawAll();
  if (_trainerCgInstance) _trainerCgInstance.redrawAll();
  if (typeof _solveCg !== 'undefined' && _solveCg) _solveCg.redrawAll();
  if (typeof _treeCgInstance !== 'undefined' && _treeCgInstance) _treeCgInstance.redrawAll();
  if (typeof _maiaCg !== 'undefined' && _maiaCg) _maiaCg.redrawAll();
}
let _resizeTimer = null;
window.addEventListener('resize', () => {
  clearTimeout(_resizeTimer);
  _resizeTimer = setTimeout(resizeBoards, 150);
});

// ══════════════════════════════════════════
// DASHBOARD TABS
// ══════════════════════════════════════════
function switchDashTab(tab) {
  document.querySelectorAll('.dash-tab').forEach(b => b.classList.toggle('active', b.dataset.tab === tab));
  document.querySelectorAll('.dash-tab-panel').forEach(p => {
    p.style.display = p.id === 'dashTab-' + tab ? '' : 'none';
  });
  // Re-trigger fade-up animations in the newly visible panel
  const panel = document.getElementById('dashTab-' + tab);
  if (panel) {
    panel.querySelectorAll('.fade-up').forEach(el => {
      el.style.animation = 'none';
      el.offsetHeight;
      el.style.animation = '';
    });
  }
}

// ══════════════════════════════════════════
// PAGE SWITCHING
// ══════════════════════════════════════════
function switchPage(name, skipHash) {
  // Kill trainer timers when navigating away
  if (name !== 'trainer') {
    if (typeof cleanupTrainerDrill === 'function') cleanupTrainerDrill();
    if (typeof cleanupMaiaDrill === 'function') cleanupMaiaDrill();
    const treeView = document.getElementById('treeViewerView');
    if (treeView) treeView.style.display = 'none';
    const selectView = document.getElementById('trainerSelectView');
    if (selectView) selectView.style.display = '';
  }

  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.sidebar-item').forEach(s => s.classList.remove('active'));
  document.querySelectorAll('.mobile-nav-item').forEach(s => s.classList.remove('active'));
  document.getElementById('page-' + name).classList.add('active');
  document.querySelector(`.sidebar-item[data-page="${name}"]`)?.classList.add('active');
  document.querySelector(`.mobile-nav-item[data-page="${name}"]`)?.classList.add('active');

  const urlName = name === 'dashboard' ? '' : name;
  if (!skipHash) history.pushState(null, '', '/' + urlName);

  // Re-trigger animations
  document.querySelectorAll('#page-' + name + ' .fade-up').forEach(el => {
    el.style.animation = 'none';
    el.offsetHeight;
    el.style.animation = '';
  });

  // Init charts/board on first visit
  if (name === 'dashboard' && !window._dashInit) initDashboard();
  if (name === 'trainer' && !window._trainerInit) initTrainer();
  if (name === 'games' && !window._gamesInit) initGames();
  if (name === 'analysis' && !window._analysisInit) initAnalysis();
  if (name === 'profile' && !window._profileInit) initProfile();

  // Resize boards after page is visible
  setTimeout(resizeBoards, 50);
}

// Handle browser back/forward
window.addEventListener('popstate', () => {
  let page = location.pathname.slice(1) || 'dashboard';
  if (page === 'tactics' || page === 'puzzles' || page === 'endgames') page = 'dashboard';
  if (document.getElementById('page-' + page)) switchPage(page, true);
});

// Navigate to hash on initial load — deferred to end of script (see auth.js)

// ══════════════════════════════════════════
// DESIGN TOKEN READER
// ══════════════════════════════════════════
const T = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

// ══════════════════════════════════════════
// CHART HELPERS
// ══════════════════════════════════════════
Chart.defaults.font.family = "'DM Sans', system-ui, sans-serif";
Chart.defaults.color = T('--text-dim');
Chart.defaults.borderColor = T('--border-subtle');
Chart.defaults.plugins.legend.display = false;
Chart.defaults.plugins.tooltip.backgroundColor = '#0f172a';
Chart.defaults.plugins.tooltip.borderColor = '#1e293b';
Chart.defaults.plugins.tooltip.borderWidth = 1;
Chart.defaults.plugins.tooltip.cornerRadius = 6;
Chart.defaults.plugins.tooltip.titleFont = { family: "'DM Sans'", size: 10 };
Chart.defaults.plugins.tooltip.bodyFont = { family: "'JetBrains Mono'", size: 11 };
Chart.defaults.plugins.tooltip.padding = 8;

const dates = Array.from({length:50},(_,i)=>{const d=new Date(2025,0,1+i*7);return d.toLocaleDateString('en',{month:'short',day:'numeric'})});

function wave(base, amp, freq, phase, len) {
  return Array.from({length:len},(_,i)=>+(base+amp*Math.sin(i*freq+phase)+(Math.random()-0.5)*3).toFixed(1));
}

const axisOpts = {
  grid: { color: T('--chart-grid'), drawBorder: false },
  ticks: { font: { family: "'JetBrains Mono'", size: 9 }, color: T('--chart-tick'), padding: 4 },
  border: { display: false },
};
