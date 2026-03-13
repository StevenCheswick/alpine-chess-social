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
  // Redraw active Chessground instances
  if (_cgInstance) _cgInstance.redrawAll();
  if (_trainerCgInstance) _trainerCgInstance.redrawAll();
  if (_solveCg) _solveCg.redrawAll();
}
let _resizeTimer = null;
window.addEventListener('resize', () => {
  clearTimeout(_resizeTimer);
  _resizeTimer = setTimeout(resizeBoards, 150);
});

// ══════════════════════════════════════════
// PAGE SWITCHING
// ══════════════════════════════════════════
function switchPage(name, skipHash) {
  // Clean up puzzle solve board if leaving
  if (_solveCg) { _solveCg.destroy(); _solveCg = null; }
  // Kill trainer timers when navigating away
  if (name !== 'trainer' && typeof _trainerGen !== 'undefined') {
    clearTimeout(_trainerAnimTimer);
    clearTimeout(_trainerRestartTimer);
    ++_trainerGen;
    _trainerPhase = 'idle';
    if (typeof _hmGen !== 'undefined') {
      clearTimeout(_hmAnimTimer);
      clearTimeout(_hmRestartTimer);
      ++_hmGen;
      _hmPhase = 'idle';
      _trainerIsHardMoveMode = false;
      const hmView = document.getElementById('hmDrillView');
      if (hmView) hmView.style.display = 'none';
    }
  }

  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.sidebar-item').forEach(s => s.classList.remove('active'));
  document.querySelectorAll('.mobile-nav-item').forEach(s => s.classList.remove('active'));
  document.getElementById('page-' + name).classList.add('active');
  document.querySelector(`.sidebar-item[data-page="${name}"]`)?.classList.add('active');
  document.querySelector(`.mobile-nav-item[data-page="${name}"]`)?.classList.add('active');

  const urlName = name === 'dashboard' ? '' : name === 'puzzles' ? 'tactics' : name;
  if (!skipHash) history.pushState(null, '', '/' + urlName);

  // Re-trigger animations
  document.querySelectorAll('#page-' + name + ' .fade-up').forEach(el => {
    el.style.animation = 'none';
    el.offsetHeight;
    el.style.animation = '';
  });

  // Init charts/board on first visit
  if (name === 'dashboard' && !window._dashInit) initDashboard();
  if (name === 'puzzles' && !window._puzzlesInit) initPuzzles();
  if (name === 'endgames' && !window._endgamesInit) initEndgames();
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
  if (page === 'tactics') page = 'puzzles';
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
