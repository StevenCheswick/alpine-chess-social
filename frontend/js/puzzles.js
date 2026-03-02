// ══════════════════════════════════════════
// PUZZLES PAGE INIT
// ══════════════════════════════════════════
async function initPuzzles() {
  window._puzzlesInit = true;
  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  const headers = { 'Authorization': 'Bearer ' + token };

  let stats;
  try {
    const res = await fetch(API_URL + '/api/puzzles/stats', { headers });
    if (!res.ok) throw new Error('Failed');
    stats = await res.json();
  } catch (err) {
    document.getElementById('puzzleGauges').innerHTML = '<div class="col-span-3 text-center text-label text-muted py-4">No puzzle data yet. Analyze some games first!</div>';
    return;
  }

  const GAUGE_C = 2 * Math.PI * 50;
  const user = stats.user || {};
  const opp = stats.opponent || {};
  const userRate = Math.round(user.rate || 0);
  const oppRate = Math.round(opp.rate || 0);
  const edge = userRate - oppRate;

  // Gauges
  document.getElementById('puzzleGauges').innerHTML = `
    <div class="flex flex-col items-center">
      <div class="relative w-28 h-28">
        <svg class="gauge-ring w-full h-full" viewBox="0 0 120 120">
          <circle class="gauge-track" cx="60" cy="60" r="50" fill="none" stroke-width="8" />
          <circle class="gauge-fill" cx="60" cy="60" r="50" fill="none"
            stroke="url(#gGlacierPuzzle)" stroke-width="8"
            stroke-dasharray="${GAUGE_C}" stroke-dashoffset="${GAUGE_C * (1 - userRate / 100)}" />
          <defs><linearGradient id="gGlacierPuzzle" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="var(--good)" /><stop offset="100%" stop-color="var(--accent-bright)" />
          </linearGradient></defs>
        </svg>
        <div class="absolute inset-0 flex flex-col items-center justify-center">
          <span class="text-xl font-bold text-good font-mono">${userRate}%</span>
        </div>
      </div>
      <p class="text-label text-secondary font-medium mt-2">Your Find Rate</p>
      <p class="text-meta text-muted font-mono">${user.found || 0} of ${user.total || 0} found</p>
    </div>
    <div class="flex flex-col items-center">
      <div class="relative w-28 h-28">
        <svg class="gauge-ring w-full h-full" viewBox="0 0 120 120">
          <circle class="gauge-track" cx="60" cy="60" r="50" fill="none" stroke-width="8" />
          <circle class="gauge-fill" cx="60" cy="60" r="50" fill="none"
            stroke="var(--text-dim)" stroke-width="8"
            stroke-dasharray="${GAUGE_C}" stroke-dashoffset="${GAUGE_C * (1 - oppRate / 100)}" />
        </svg>
        <div class="absolute inset-0 flex flex-col items-center justify-center">
          <span class="text-xl font-bold text-secondary font-mono">${oppRate}%</span>
        </div>
      </div>
      <p class="text-label text-secondary font-medium mt-2">Opponent Find Rate</p>
      <p class="text-meta text-muted font-mono">${opp.found || 0} of ${opp.total || 0} found</p>
    </div>
    <div class="flex flex-col items-center justify-center">
      <div class="card-neutral p-5 w-full text-center">
        <p class="text-3xl font-bold ${edge >= 0 ? 'text-good' : 'text-bad'} font-mono leading-none">${edge >= 0 ? '+' : ''}${edge}%</p>
        <p class="text-label text-secondary font-medium mt-2">Tactical Edge</p>
        <p class="text-meta ${edge >= 0 ? 'text-good-dim' : 'text-bad'} mt-1">${edge >= 0 ? 'You outperform your opponents' : 'Opponents have the edge'}</p>
      </div>
    </div>`;

  // Position breakdown
  const positions = stats.byPosition || [];
  const posLabels = { winning: 'Winning Positions', equal: 'Equal Positions', losing: 'Losing Positions' };
  document.getElementById('puzzlePositions').innerHTML = positions.map(p => {
    const uRate = Math.round(p.user?.rate || 0);
    const oRate = Math.round(p.opponent?.rate || 0);
    return `<div>
      <div class="flex items-center justify-between mb-1.5">
        <span class="text-label text-white font-medium">${posLabels[p.position] || p.position}</span>
        <span class="text-meta text-muted font-mono">${p.user?.total || 0} puzzles</span>
      </div>
      <div class="flex items-center gap-2 mb-1">
        <span class="text-meta text-muted w-8">You</span>
        <div class="flex-1 h-3 rounded-full bg-slate-800/60 overflow-hidden"><div class="h-full rounded-full bg-good" style="width:${uRate}%"></div></div>
        <span class="text-meta font-mono text-good w-8 text-right">${uRate}%</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-meta text-muted w-8">Opp</span>
        <div class="flex-1 h-3 rounded-full bg-slate-800/60 overflow-hidden"><div class="h-full rounded-full bg-slate-600/80" style="width:${oRate}%"></div></div>
        <span class="text-meta font-mono text-muted w-8 text-right">${oRate}%</span>
      </div>
    </div>`;
  }).join('');

  // Theme tables
  const themes = (stats.byTheme || []).filter(t => (t.user?.total || 0) >= 10 && isVisibleTag(t.theme));
  const alphaSort = (a, b) => tagDisplayName(a.theme).localeCompare(tagDisplayName(b.theme));
  const mateThemes = themes.filter(t => t.theme.toLowerCase().includes('mate')).sort(alphaSort);
  const tacticThemes = themes.filter(t => !t.theme.toLowerCase().includes('mate')).sort(alphaSort);

  function themeTable(title, items) {
    if (items.length === 0) return '';
    const rows = items.slice(0, 8).map((t, i) => {
      const uR = Math.round(t.user?.rate || 0);
      const oR = Math.round(t.opponent?.rate || 0);
      const edge = uR - oR;
      const edgeColor = edge > 0 ? 'text-good' : edge < 0 ? 'text-bad' : 'text-muted';
      return `<div class="grid grid-cols-[1fr_40px_40px_44px_40px] gap-x-2 px-1 py-1.5 text-secondary ${i % 2 === 1 ? 'bg-slate-800/20 rounded' : ''}">
        <span>${tagDisplayName(t.theme)}</span>
        <span class="text-right font-mono text-white">${uR}%</span>
        <span class="text-right font-mono text-muted">${oR}%</span>
        <span class="text-right font-mono ${edgeColor}">${edge > 0 ? '+' : ''}${edge}%</span>
        <span class="text-right font-mono text-muted">${t.user?.total || 0}</span>
      </div>`;
    }).join('');

    return `<div class="card p-4 fade-up">
      <h2 class="text-xs font-semibold text-white mb-3">${title}</h2>
      <div class="text-meta">
        <div class="grid grid-cols-[1fr_40px_40px_44px_40px] gap-x-2 px-1 py-1.5 text-secondary uppercase tracking-wider font-medium border-b border-slate-800/50">
          <span>Theme</span><span class="text-right">You</span><span class="text-right">Opp</span><span class="text-right">Edge</span><span class="text-right">Total</span>
        </div>
        ${rows}
      </div>
    </div>`;
  }

  document.getElementById('puzzleThemeTables').innerHTML =
    themeTable('By Tactic', tacticThemes) + themeTable('By Checkmate Pattern', mateThemes);

  // Now load the actual puzzles for the grid
  await loadPuzzleGrid();
}

// ══════════════════════════════════════════
// PUZZLE BROWSING + SOLVE MODE
// ══════════════════════════════════════════
const PUZZLES_PER_PAGE = 9;
let _allPuzzles = [];
let _puzzleThemes = {};
let _puzzleSelectedTheme = null;
let _puzzlePage = 1;
// Solve mode state
let _solvePuzzle = null;
let _solveGame = null;       // chess.js instance
let _solveMoveIndex = 0;
let _solveStatus = 'solving'; // 'solving' | 'solved' | 'failed'
var _solveCg = null;          // Chessground instance for solve board (var for hoisting)
let _solveShowSolution = false;
let _solveSolverColor = 'white';

async function loadPuzzleGrid(theme) {
  const token = localStorage.getItem('alpine_token');
  if (!token) return;

  let url = API_URL + '/api/puzzles';
  if (theme) url += '?theme=' + encodeURIComponent(theme);

  try {
    const res = await fetch(url, { headers: { 'Authorization': 'Bearer ' + token } });
    if (!res.ok) throw new Error('Failed');
    const data = await res.json();
    _allPuzzles = (data.puzzles || []).filter(p => (p.themes || []).some(isVisibleTag));
    _puzzleThemes = data.themes || {};
  } catch {
    _allPuzzles = [];
    _puzzleThemes = {};
  }

  _puzzlePage = 1;
  renderPuzzleFilters();
  renderPuzzleGrid();
}

function renderPuzzleFilters() {
  const sortedThemes = Object.entries(_puzzleThemes)
    .filter(([t]) => isVisibleTag(t))
    .sort((a, b) => b[1] - a[1])
    .map(([t]) => t);

  if (sortedThemes.length === 0) {
    document.getElementById('puzzleFilters').innerHTML = '';
    return;
  }

  document.getElementById('puzzleFilters').innerHTML = `
    <div class="flex items-center gap-1.5 flex-wrap">
      <span class="text-label text-muted mr-1">Filter:</span>
      ${sortedThemes.map(t => {
        const sel = _puzzleSelectedTheme === t;
        return `<button onclick="selectPuzzleTheme('${t}')"
          class="px-2 py-1 text-meta font-medium rounded-md border transition-colors ${
            sel ? 'bg-accent-dim text-accent border-accent/40' : 'bg-transparent text-secondary border-slate-700 hover:border-slate-500'
          }">${tagDisplayName(t)} <span class="font-mono text-muted">${_puzzleThemes[t]}</span></button>`;
      }).join('')}
    </div>`;
}

function renderPuzzleGrid() {
  const puzzles = _allPuzzles;
  const total = puzzles.length;
  const totalPages = Math.max(1, Math.ceil(total / PUZZLES_PER_PAGE));
  const start = (_puzzlePage - 1) * PUZZLES_PER_PAGE;
  const paginated = puzzles.slice(start, start + PUZZLES_PER_PAGE);

  // Count bar
  const countBar = document.getElementById('puzzleCountBar');
  if (total === 0) {
    countBar.innerHTML = '';
    document.getElementById('puzzleGrid').innerHTML = '';
    document.getElementById('puzzlePagination').innerHTML = '';
    document.getElementById('puzzleEmptyState').classList.remove('hidden');
    document.getElementById('puzzleEmptyState').innerHTML = `
      <div class="card p-8 text-center">
        <div class="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5"><path d="M12 2C9.5 2 8 3.5 8 5.5c0 1.5.5 2 1 2.5L8 10h8l-1-2c.5-.5 1-1 1-2.5C16 3.5 14.5 2 12 2z"/><rect x="7" y="10" width="10" height="2" rx="0.5"/><path d="M8 12v7a3 3 0 003 3h2a3 3 0 003-3v-7"/></svg>
        </div>
        <h2 class="text-lg font-semibold text-white mb-2">No tactics yet</h2>
        <p class="text-body text-muted mb-5">Tactics are automatically extracted when you analyze your games.</p>
        <button onclick="switchPage('games')" class="px-5 py-2.5 rounded-lg font-medium text-white text-body bg-gradient-to-r from-sky-400 to-blue-500 hover:from-sky-300 hover:to-blue-400 shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-all">Go to Games</button>
      </div>`;
    return;
  }

  document.getElementById('puzzleEmptyState').classList.add('hidden');

  countBar.innerHTML = `
    <p class="text-label text-muted">
      <span class="text-white font-mono">${total}</span> tactics from your games
      ${_puzzleSelectedTheme ? `<span class="text-accent ml-1">— ${tagDisplayName(_puzzleSelectedTheme)} <button onclick="selectPuzzleTheme('${_puzzleSelectedTheme}')" class="ml-1 text-muted hover:text-white transition-colors">&times;</button></span>` : ''}
    </p>
    ${totalPages > 1 ? `<p class="text-label text-muted">Showing ${start + 1}–${Math.min(start + PUZZLES_PER_PAGE, total)}</p>` : ''}`;

  // List (games-page style)
  const grid = document.getElementById('puzzleGrid');
  grid.innerHTML = paginated.map((p, i) => {
    const flip = p.fen.split(' ')[1] === 'w'; // flip if solver is black
    const visibleTags = (p.themes || []).filter(isVisibleTag);
    const solverColor = p.fen.split(' ')[1] === 'w' ? 'Black' : 'White';
    const moveCount = Math.floor(p.moves.length / 2);
    const source = p.source === 'chess_com' ? 'C' : 'L';
    const sourceBg = p.source === 'chess_com' ? 'bg-good/20 text-good' : 'bg-white/20 text-white';
    return `
    <div class="card p-4 cursor-pointer transition-all hover:border-sky-400/40 fade-up" onclick="openPuzzleSolve(${i + start})" style="animation-delay:${0.05 + i * 0.03}s">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <div class="w-48 h-48 rounded overflow-hidden shrink-0">${fenToMiniBoard(p.fen, flip)}</div>
          <div>
            <div class="flex items-center gap-2">
              <span class="w-5 h-5 rounded text-meta font-bold flex items-center justify-center ${sourceBg}">${source}</span>
              <span class="text-sm text-white font-medium">vs ${esc(p.opponent)}</span>
            </div>
            <div class="flex items-center gap-2 flex-wrap mt-1 text-label text-muted">
              <span class="font-medium">${solverColor} to move</span>
              <span class="text-slate-700">&middot;</span>
              <span>${moveCount} move${moveCount !== 1 ? 's' : ''}</span>
              <span class="text-slate-700">&middot;</span>
              <span>as ${esc(p.userColor || 'white')}</span>
              ${visibleTags.slice(0, 3).map(t => `<span class="px-1.5 py-0.5 text-label rounded border text-good whitespace-nowrap" style="border-color:var(--accent-dim)">${tagDisplayName(t)}</span>`).join('')}
              ${visibleTags.length > 3 ? `<span class="px-1.5 py-0.5 text-label rounded text-muted">+${visibleTags.length - 3}</span>` : ''}
            </div>
          </div>
        </div>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-muted shrink-0"><path d="M9 18l6-6-6-6"/></svg>
      </div>
    </div>`;
  }).join('');

  // Pagination
  const pag = document.getElementById('puzzlePagination');
  if (totalPages <= 1) { pag.innerHTML = ''; return; }
  const btnCls = 'px-2 py-1 text-meta text-muted rounded border border-slate-700 hover:border-slate-500 transition-colors';
  const disCls = 'px-2 py-1 text-meta text-muted rounded border border-slate-800 opacity-40 cursor-default';
  pag.innerHTML = `
    <button onclick="puzzleGoPage(1)" class="${_puzzlePage === 1 ? disCls : btnCls}" ${_puzzlePage === 1 ? 'disabled' : ''}>First</button>
    <button onclick="puzzleGoPage(${_puzzlePage - 1})" class="${_puzzlePage === 1 ? disCls : btnCls}" ${_puzzlePage === 1 ? 'disabled' : ''}>Prev</button>
    <span class="px-3 py-1 text-label text-muted font-mono">Page ${_puzzlePage} of ${totalPages}</span>
    <button onclick="puzzleGoPage(${_puzzlePage + 1})" class="${_puzzlePage === totalPages ? disCls : btnCls}" ${_puzzlePage === totalPages ? 'disabled' : ''}>Next</button>
    <button onclick="puzzleGoPage(${totalPages})" class="${_puzzlePage === totalPages ? disCls : btnCls}" ${_puzzlePage === totalPages ? 'disabled' : ''}>Last</button>`;
}

function selectPuzzleTheme(theme) {
  if (_puzzleSelectedTheme === theme) {
    _puzzleSelectedTheme = null;
  } else {
    _puzzleSelectedTheme = theme;
  }
  loadPuzzleGrid(_puzzleSelectedTheme);
}

function puzzleGoPage(p) {
  const totalPages = Math.max(1, Math.ceil(_allPuzzles.length / PUZZLES_PER_PAGE));
  _puzzlePage = Math.max(1, Math.min(totalPages, p));
  renderPuzzleGrid();
}

// ── Puzzle Solve Mode ──
function openPuzzleSolve(idx) {
  const puzzle = _allPuzzles[idx];
  if (!puzzle) return;
  _solvePuzzle = puzzle;
  _solvePuzzleIdx = idx;
  _solveShowSolution = false;
  _solveStatus = 'solving';

  // Solver color: opposite of side to move in FEN (FEN is before opponent's blunder)
  _solveSolverColor = puzzle.fen.split(' ')[1] === 'w' ? 'black' : 'white';

  // Show solve view, hide puzzle list
  document.getElementById('page-puzzles').classList.remove('active');
  document.getElementById('puzzleSolveView').classList.add('active');

  // Populate info panels
  const visibleTags = (puzzle.themes || []).filter(isVisibleTag);
  document.getElementById('puzzleSolveThemes').innerHTML = `
    <h3 class="text-label font-medium text-muted mb-2">Themes</h3>
    <div class="flex flex-wrap gap-2">
      ${visibleTags.map(t => `<span class="px-1.5 py-0.5 text-meta rounded border text-good" style="border-color:var(--accent-dim)">${tagDisplayName(t)}</span>`).join('')}
    </div>`;

  const srcBadge = puzzle.source === 'chess_com'
    ? '<span class="w-4 h-4 rounded flex items-center justify-center text-[9px] font-bold bg-green-600 text-white">C</span>'
    : '<span class="w-4 h-4 rounded flex items-center justify-center text-[9px] font-bold bg-white text-black">L</span>';
  document.getElementById('puzzleSolveSource').innerHTML = `
    <h3 class="text-label font-medium text-muted mb-2">Source Game</h3>
    <button onclick="switchPage('analysis'); loadGameAnalysis(${puzzle.gameId})" class="text-accent hover:text-accent-bright text-label transition-colors">
      vs ${esc(puzzle.opponent)} ${puzzle.date ? `<span class="text-muted ml-2">${esc(puzzle.date)}</span>` : ''}
    </button>
    <div class="flex items-center gap-2 mt-1">
      ${srcBadge}
      <span class="text-meta text-muted capitalize">as ${puzzle.userColor}</span>
    </div>`;

  const moveCount = Math.floor(puzzle.moves.length / 2);
  const solverLabel = _solveSolverColor === 'white' ? 'White' : 'Black';
  document.getElementById('puzzleSolveInfo').innerHTML = `
    <h3 class="text-label font-medium text-muted mb-2">Puzzle Info</h3>
    <p class="text-body text-secondary">${solverLabel} to move — ${moveCount} move${moveCount !== 1 ? 's' : ''} to find</p>`;

  initSolveBoard(puzzle);
}

function closePuzzleSolve() {
  document.getElementById('puzzleSolveView').classList.remove('active');
  document.getElementById('page-puzzles').classList.add('active');
  if (_solveCg) { _solveCg.destroy(); _solveCg = null; }
  _solvePuzzle = null;
  _solveGame = null;
}

function initSolveBoard(puzzle) {
  if (!Chess || !Chessground) {
    setTimeout(() => initSolveBoard(puzzle), 200);
    return;
  }

  _solveGame = new Chess(puzzle.fen);
  _solveMoveIndex = 0;
  _solveStatus = 'solving';
  _solveShowSolution = false;

  setSolveBoard(_solveGame.fen(), _solveSolverColor, false);

  updateSolveActions();
  updateSolveStatus();

  // Auto-play opponent's blunder (first move) after 600ms
  if (puzzle.moves.length > 0) {
    setTimeout(() => {
      const move = uciToChessJs(puzzle.moves[0]);
      try { _solveGame.move(move); } catch { return; }
      _solveMoveIndex = 1;
      setSolveBoard(_solveGame.fen(), _solveSolverColor, true);
      updateSolveStatus();
    }, 600);
  }
}

function setSolveBoard(fen, orientation, movable) {
  const el = document.getElementById('puzzleSolveBoard');
  const turnColor = fen.split(' ')[1] === 'w' ? 'white' : 'black';
  const dests = movable ? getSolveDests() : new Map();
  if (_solveCg) { _solveCg.destroy(); _solveCg = null; }
  el.innerHTML = '';
  _solveCg = Chessground(el, {
    fen, orientation, turnColor,
    viewOnly: false,
    coordinates: true,
    animation: { duration: 250 },
    movable: {
      free: false,
      color: movable ? orientation : undefined,
      dests,
      showDests: true,
      events: { after: (orig, dest) => onSolverMove(orig, dest) },
    },
    draggable: { enabled: true },
  });
}

function getSolveDests() {
  if (!_solveGame) return new Map();
  const dests = new Map();
  for (const m of _solveGame.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  return dests;
}

function uciToChessJs(uci) {
  return {
    from: uci.slice(0, 2),
    to: uci.slice(2, 4),
    promotion: uci.length === 5 ? uci[4] : undefined,
  };
}

function enableSolverMoves() {
  if (!_solveGame || _solveStatus !== 'solving') return;
  setSolveBoard(_solveGame.fen(), _solveSolverColor, true);
}

function onSolverMove(orig, dest) {
  if (_solveStatus !== 'solving' || _solveMoveIndex >= _solvePuzzle.moves.length) return;

  const expectedUci = _solvePuzzle.moves[_solveMoveIndex];
  const expected = uciToChessJs(expectedUci);

  // Build attempted UCI
  let attemptedUci = orig + dest;
  // Handle promotion
  const piece = _solveGame.get(orig);
  if (expected.promotion) {
    attemptedUci += expected.promotion;
  } else if (piece && piece.type === 'p' && (dest[1] === '8' || dest[1] === '1')) {
    attemptedUci += 'q';
  }

  if (attemptedUci === expectedUci) {
    // Correct!
    const moveObj = { from: orig, to: dest, promotion: expected.promotion || (piece && piece.type === 'p' && (dest[1] === '8' || dest[1] === '1') ? 'q' : undefined) };
    try { _solveGame.move(moveObj); } catch { return; }

    _solveMoveIndex++;
    setSolveBoard(_solveGame.fen(), _solveSolverColor, false);

    // Check if solved
    if (_solveMoveIndex >= _solvePuzzle.moves.length) {
      _solveStatus = 'solved';
      updateSolveStatus();
      updateSolveActions();
      return;
    }

    // Auto-play opponent reply
    setTimeout(() => {
      const oppUci = _solvePuzzle.moves[_solveMoveIndex];
      const oppMove = uciToChessJs(oppUci);
      try { _solveGame.move(oppMove); } catch { return; }
      _solveMoveIndex++;
      setSolveBoard(_solveGame.fen(), _solveSolverColor, false);

      if (_solveMoveIndex >= _solvePuzzle.moves.length) {
        _solveStatus = 'solved';
        updateSolveStatus();
        updateSolveActions();
      } else {
        enableSolverMoves();
        updateSolveStatus();
      }
    }, 400);
  } else {
    // Wrong move - revert
    _solveStatus = 'failed';
    setSolveBoard(_solveGame.fen(), _solveSolverColor, false);
    updateSolveStatus();
    updateSolveActions();
  }
}

function flashSquare(sq, color) {
  // Brief highlight via Chessground drawable shapes
  // Since CG doesn't natively do this well, we'll use the DOM
  const boardEl = document.getElementById('puzzleSolveBoard');
  const flash = document.createElement('div');
  flash.style.cssText = `position:absolute;top:0;left:0;right:0;bottom:0;background:${color};pointer-events:none;z-index:10;opacity:1;transition:opacity 0.6s`;
  // Find the square element
  const cgEl = boardEl.querySelector('cg-board');
  if (cgEl) {
    flash.style.position = 'absolute';
    flash.style.borderRadius = '0';
    cgEl.appendChild(flash);
    requestAnimationFrame(() => { flash.style.opacity = '0'; });
    setTimeout(() => flash.remove(), 700);
  }
}

function updateSolveStatus() {
  const el = document.getElementById('puzzleSolveStatus');
  if (_solveStatus === 'solved') {
    el.innerHTML = '<p class="text-body font-semibold" style="color:var(--good)">Puzzle solved!</p>';
  } else if (_solveStatus === 'failed') {
    el.innerHTML = '<p class="text-body font-semibold" style="color:var(--bad)">Incorrect — puzzle failed</p>';
  } else {
    const remaining = Math.ceil((_solvePuzzle.moves.length - _solveMoveIndex) / 2);
    if (_solveMoveIndex === 0) {
      el.innerHTML = '<p class="text-body text-secondary">Find the best move</p>';
    } else {
      el.innerHTML = `<p class="text-body text-secondary">${remaining} move${remaining !== 1 ? 's' : ''} to find</p>`;
    }
  }
}

function updateSolveActions() {
  const el = document.getElementById('puzzleSolveActions');
  const hasNext = _solvePuzzleIdx < _allPuzzles.length - 1;
  const btnSecondary = 'px-4 py-2 rounded-lg text-label font-medium text-muted border border-slate-700 hover:bg-slate-800 hover:text-white transition-colors';
  const btnPrimary = 'px-4 py-2 rounded-lg font-medium text-white text-body bg-gradient-to-r from-sky-400 to-blue-500 hover:from-sky-300 hover:to-blue-400 shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-all';
  let html = '';

  if (_solveStatus === 'solving' && !_solveShowSolution) {
    html += `<button onclick="showPuzzleSolution()" class="${btnSecondary}">Show Solution</button>`;
  }
  if (_solveStatus === 'failed' || _solveStatus === 'solved') {
    html += `<button onclick="retryPuzzleSolve()" class="${btnSecondary}">Retry Puzzle</button>`;
  }
  if (_solveStatus === 'failed' && !_solveShowSolution) {
    html += `<button onclick="showPuzzleSolution()" class="${btnSecondary}">Show Solution</button>`;
  }
  if (hasNext) {
    html += `<button onclick="nextPuzzleSolve()" class="${btnPrimary}">Next Puzzle</button>`;
  }

  el.innerHTML = html;
}

function showPuzzleSolution() {
  _solveShowSolution = true;
  if (_solveCg && _solveMoveIndex < _solvePuzzle.moves.length) {
    const m = _solvePuzzle.moves[_solveMoveIndex];
    _solveCg.set({
      drawable: {
        autoShapes: [{
          orig: m.slice(0, 2),
          dest: m.slice(2, 4),
          brush: 'green',
        }],
      },
    });
  }
  updateSolveActions();
}

function retryPuzzleSolve() {
  if (_solvePuzzle) initSolveBoard(_solvePuzzle);
}

function nextPuzzleSolve() {
  const nextIdx = _solvePuzzleIdx + 1;
  if (nextIdx < _allPuzzles.length) {
    if (_solveCg) { _solveCg.destroy(); _solveCg = null; }
    openPuzzleSolve(nextIdx);
  }
}

let _solvePuzzleIdx = 0;
