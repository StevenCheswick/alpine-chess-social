// ══════════════════════════════════════════
// TRAINER PAGE INIT
// ══════════════════════════════════════════
const PIECE_URL = 'https://lichess1.org/assets/piece/cburnett/';
const fenPieceMap = {
  K:'wK', Q:'wQ', R:'wR', B:'wB', N:'wN', P:'wP',
  k:'bK', q:'bQ', r:'bR', b:'bB', n:'bN', p:'bP',
};

function fenToMiniBoard(fen, flip) {
  let html = '<div class="mini-board grid grid-cols-8 aspect-square rounded overflow-hidden">';
  let rows = fen.split(' ')[0].split('/');
  if (flip) rows = rows.reverse().map(r => r.split('').reverse().join(''));
  rows.forEach((row, ri) => {
    let ci = 0;
    for (const ch of row) {
      if (ch >= '1' && ch <= '8') {
        for (let e = 0; e < parseInt(ch); e++) {
          const light = (ri + ci) % 2 === 0;
          html += `<div class="${light ? 'sq-light' : 'sq-dark'}"></div>`;
          ci++;
        }
      } else {
        const light = (ri + ci) % 2 === 0;
        const piece = fenPieceMap[ch];
        html += `<div class="${light ? 'sq-light' : 'sq-dark'} sq-piece"><img src="${PIECE_URL}${piece}.svg" alt="${ch}"></div>`;
        ci++;
      }
    }
  });
  return html + '</div>';
}

// Replay SAN moves to get final FEN (same logic as React MiniChessBoard)
function getFinalFen(moves) {
  if (!Chess || !moves || moves.length === 0) return null;
  try {
    const g = new Chess();
    for (const m of moves) {
      const clean = m.replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
      if (clean && clean !== '1-0' && clean !== '0-1' && clean !== '1/2-1/2') {
        g.move(clean);
      }
    }
    return g.fen();
  } catch { return null; }
}

async function initTrainer() {
  window._trainerInit = true;
  const token = localStorage.getItem('alpine_token');
  if (!token) return;

  const grid = document.getElementById('trainerGrid');

  // Fetch both opening types in parallel
  const [puzzleRes, hmRes] = await Promise.allSettled([
    fetch(API_URL + '/api/trainer/openings', { headers: { 'Authorization': 'Bearer ' + token } }).then(r => r.ok ? r.json() : []),
    fetch(API_URL + '/api/trainer/hard-moves/openings', { headers: { 'Authorization': 'Bearer ' + token } }).then(r => r.ok ? r.json() : []),
  ]);

  const puzzleOpenings = puzzleRes.status === 'fulfilled' ? (puzzleRes.value || []) : [];
  const hardMoveOpenings = hmRes.status === 'fulfilled' ? (hmRes.value || []) : [];

  if (puzzleOpenings.length === 0 && hardMoveOpenings.length === 0) {
    grid.innerHTML = '<div class="col-span-3 text-center text-label text-muted py-8">No trainer openings available yet.</div>';
    return;
  }

  function trainerCard(o, type) {
    const count = type === 'puzzle' ? o.puzzle_count : o.count;
    const pct = count > 0 ? Math.round((o.completed_count / count) * 100) : 0;
    const done = pct === 100;
    const boardHtml = o.sample_fen ? fenToMiniBoard(o.sample_fen) : '<div class="aspect-square bg-slate-900/50 rounded"></div>';
    const isPuzzle = type === 'puzzle';
    const onclick = isPuzzle
      ? `openTrainerOpening('${o.opening_name.replace(/'/g, "\\'")}')`
      : `openHardMoveOpening('${o.opening_name.replace(/'/g, "\\'")}')`;
    const hoverBorder = isPuzzle ? 'hover:border-sky-400/40' : 'hover:border-amber-400/40';
    const badge = isPuzzle
      ? '<span class="inline-block text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded bg-red-500/15 text-red-400 mb-1.5">Punish Mistakes</span>'
      : '<span class="inline-block text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400 mb-1.5">Find the Move</span>';
    const barColor = done ? 'var(--good)' : (isPuzzle ? 'var(--accent)' : '#f59e0b');

    return `
    <div class="card p-0 cursor-pointer transition-all ${hoverBorder} group" style="border-radius:10px" onclick="${onclick}">
      ${boardHtml}
      <div class="p-3">
        <div class="flex items-center gap-1.5 mb-0.5">
          <span class="text-body text-white font-medium group-hover:text-white transition-colors">${o.opening_name}</span>
          ${done ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--good)" stroke-width="2.5" stroke-linecap="round"><path d="M5 13l4 4L19 7"/></svg>' : ''}
        </div>
        ${badge}
        <div class="flex items-center justify-between mb-1.5">
          <span class="text-meta text-muted">${o.completed_count}/${count} completed</span>
          <span class="text-meta font-mono ${done ? 'text-good' : 'text-muted'}">${pct}%</span>
        </div>
        <div class="h-1.5 rounded-full bg-slate-800/60 overflow-hidden">
          <div class="h-full rounded-full transition-all" style="width:${pct}%; background: ${barColor}"></div>
        </div>
      </div>
    </div>`;
  }

  grid.innerHTML =
    puzzleOpenings.map(o => trainerCard(o, 'puzzle')).join('') +
    hardMoveOpenings.map(o => trainerCard(o, 'hard-move')).join('');
}

// ══════════════════════════════════════════
// TRAINER DRILL ENGINE
// ══════════════════════════════════════════
let _trainerPuzzles = [];
let _trainerCompletedIds = new Set();
let _trainerPuzzleIdx = 0;
let _trainerNode = null;
let _trainerGame = null;
let _trainerCgInstance = null;
let _trainerPhase = 'idle';
let _trainerSolverColor = 'white';
let _trainerMoveHistory = [];
let _trainerMistakeThisRun = false;
let _trainerOpeningName = '';
// Variation drilling state
let _trainerVisitedLeaves = new Set();
let _trainerDrillMode = 'main'; // 'main' | 'deep'
let _trainerTotalLeaves = 1;
let _trainerVariationsCompleted = 0;
let _trainerIsFirstAttempt = true;
let _trainerOpponentOrder = new Map();
let _trainerRestartTimer = null;
let _trainerAnimTimer = null;
let _trainerGen = 0;
let _trainerDeepStartVisited = 0;

// ── Tree utility functions (match React TrainerBoard.tsx) ──

/** Count all variations (leaf paths). Opponent=sum children, Solver=max accepted. */
function countLeaves(node) {
  if (node.type === 'cutoff' || node.type === 'terminal') return 1;
  if (!node.moves) return 1;
  const entries = Object.values(node.moves);
  if (entries.length === 0) return 1;
  if (node.type === 'opponent') {
    let total = 0;
    for (const m of entries) { if (m.result) total += countLeaves(m.result); }
    return total || 1;
  }
  let best = 0;
  for (const m of entries) {
    if (!m.accepted) continue;
    const n = m.result ? countLeaves(m.result) : 1;
    if (n > best) best = n;
  }
  return best || 1;
}

/** Count visited variations using same max/sum logic as countLeaves. Prevents Set.size mismatch. */
function countVisitedLeaves(node, visited) {
  if (node.type === 'cutoff' || node.type === 'terminal') return visited.has(node) ? 1 : 0;
  if (!node.moves) return visited.has(node) ? 1 : 0;
  const entries = Object.values(node.moves);
  if (entries.length === 0) return visited.has(node) ? 1 : 0;
  if (node.type === 'opponent') {
    let total = 0;
    for (const m of entries) { if (m.result) total += countVisitedLeaves(m.result, visited); }
    return total;
  }
  let best = 0;
  for (const m of entries) {
    if (!m.accepted) continue;
    const n = m.result ? countVisitedLeaves(m.result, visited) : (visited.has(node) ? 1 : 0);
    if (n > best) best = n;
  }
  return best;
}

/** Count variations following only the main line (best opponent move at each node) */
function countMainLineLeaves(node) {
  if (node.type === 'cutoff' || node.type === 'terminal') return 1;
  if (!node.moves) return 1;
  const entries = Object.values(node.moves);
  if (entries.length === 0) return 1;
  if (node.type === 'opponent') {
    const computed = entries.filter(m => m.result);
    if (computed.length === 0) return 1;
    const best = computed.reduce((a, b) => ((a.games ?? 0) >= (b.games ?? 0) ? a : b));
    return best.result ? countMainLineLeaves(best.result) : 1;
  }
  let best = 0;
  for (const m of entries) {
    if (!m.accepted) continue;
    const n = m.result ? countMainLineLeaves(m.result) : 1;
    if (n > best) best = n;
  }
  return best || 1;
}

/** Check if tree has opponent nodes with more than one computed move */
function treeHasDeepVariations(node) {
  if (node.type === 'cutoff' || node.type === 'terminal' || !node.moves) return false;
  const entries = Object.entries(node.moves);
  if (node.type === 'opponent') {
    const computed = entries.filter(([, m]) => m.result);
    if (computed.length > 1) return true;
    for (const [, m] of computed) {
      if (m.result && treeHasDeepVariations(m.result)) return true;
    }
    return false;
  }
  for (const [, m] of entries) {
    if (!m.accepted) continue;
    if (m.result && treeHasDeepVariations(m.result)) return true;
  }
  return false;
}

/** Check if a subtree has any unvisited leaf nodes */
function hasUnvisitedLeaves(node, visited) {
  if (node.type === 'cutoff' || node.type === 'terminal' || !node.moves) {
    return !visited.has(node);
  }
  const entries = Object.values(node.moves);
  if (entries.length === 0) return !visited.has(node);
  if (node.type === 'opponent') {
    for (const m of entries) {
      if (m.result && hasUnvisitedLeaves(m.result, visited)) return true;
    }
    return false;
  }
  for (const m of entries) {
    if (!m.accepted) continue;
    if (!m.result) { if (!visited.has(node)) return true; continue; }
    if (hasUnvisitedLeaves(m.result, visited)) return true;
  }
  return false;
}

/** Mark all direct cutoff/terminal results of accepted moves at a solver node as visited */
function markSiblingLeaves(solverNode, visited) {
  if (!solverNode.moves) return;
  for (const m of Object.values(solverNode.moves)) {
    if (!m.accepted) continue;
    if (m.result && (m.result.type === 'cutoff' || m.result.type === 'terminal')) {
      visited.add(m.result);
    }
    if (!m.result) visited.add(solverNode);
  }
}

function setTrainerStatus(title, msg, type) {
  // type: 'success' | 'error' | 'info'
  const titleEl = document.getElementById('trainerStatusTitle');
  const msgEl = document.getElementById('trainerStatusMsg');
  titleEl.textContent = title;
  msgEl.textContent = msg || '';
  titleEl.className = 'text-sm font-semibold' + (
    type === 'success' ? ' text-good' :
    type === 'error' ? ' text-bad' :
    ' text-slate-300'
  );
}

function setTrainerBoard(fen, orientation, movable, boardId) {
  if (!Chessground) return;
  const el = document.getElementById(boardId || 'trainerBoard');
  const dests = movable ? getTrainerDests() : new Map();
  const turnColor = fen.split(' ')[1] === 'w' ? 'white' : 'black';
  // Always recreate to ensure events are bound correctly
  if (_trainerCgInstance) { _trainerCgInstance.destroy(); _trainerCgInstance = null; }
  el.innerHTML = '';
  _trainerCgInstance = Chessground(el, {
    fen, orientation, turnColor,
    viewOnly: false,
    coordinates: true,
    animation: { duration: 250 },
    movable: {
      free: false,
      color: movable ? turnColor : undefined,
      dests,
      showDests: true,
      events: { after: (orig, dest) => _trainerIsHardMoveMode ? hmOnMove(orig, dest) : trainerOnMove(orig, dest) },
    },
    draggable: { enabled: true },
  });
  setTimeout(resizeBoards, 50);
}

function getTrainerDests() {
  if (!_trainerGame) return new Map();
  const dests = new Map();
  for (const m of _trainerGame.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  return dests;
}

async function openTrainerOpening(name) {
  _trainerOpeningName = name;
  const token = localStorage.getItem('alpine_token');
  if (!token || !Chess) return;

  document.getElementById('trainerSelectView').style.display = 'none';
  document.getElementById('trainerDrillView').style.display = '';
  setTrainerStatus('Loading puzzles...', '', 'info');
  document.getElementById('trainerMoveList').innerHTML = '';

  try {
    const res = await fetch(API_URL + '/api/trainer/puzzles?opening=' + encodeURIComponent(name), {
      headers: { 'Authorization': 'Bearer ' + token },
    });
    if (!res.ok) throw new Error('Failed to load puzzles');
    const data = await res.json();
    _trainerPuzzles = data.puzzles || [];
    _trainerCompletedIds = new Set(data.completed_ids || []);
  } catch (err) {
    setTrainerStatus('Error', err.message, 'error');
    return;
  }

  if (_trainerPuzzles.length === 0) {
    setTrainerStatus('No puzzles', 'No puzzles available for this opening.', 'info');
    return;
  }

  _trainerPuzzleIdx = _trainerPuzzles.findIndex(p => !_trainerCompletedIds.has(p.id));
  if (_trainerPuzzleIdx < 0) _trainerPuzzleIdx = 0;

  resetTrainerVariationState();
  updateTrainerProgress();
  startTrainerPuzzle();
}

function exitTrainerDrill() {
  document.getElementById('trainerDrillView').style.display = 'none';
  document.getElementById('hmDrillView').style.display = 'none';
  document.getElementById('trainerSelectView').style.display = '';
  clearTimeout(_trainerAnimTimer);
  clearTimeout(_trainerRestartTimer);
  clearTimeout(_hmAnimTimer);
  clearTimeout(_hmRestartTimer);
  ++_trainerGen;
  ++_hmGen;
  _trainerPhase = 'idle';
  _hmPhase = 'idle';
  _trainerIsHardMoveMode = false;
  window._trainerInit = false;
  initTrainer();
}

function updateTrainerProgress() {
  const total = _trainerPuzzles.length;
  const done = _trainerCompletedIds.size;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;
  document.getElementById('trainerProgressLabel').textContent = `${done}/${total} completed`;
  document.getElementById('trainerProgressPct').textContent = `${pct}%`;
  document.getElementById('trainerProgressBar').style.width = `${pct}%`;
  document.getElementById('trainerDrillCounter').textContent = `${_trainerOpeningName} — Puzzle ${_trainerPuzzleIdx + 1} / ${total}`;
}

function startTrainerPuzzle(fast) {
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  if (!puzzle) return;

  const isFast = fast || !_trainerIsFirstAttempt;
  console.log(`[TRAINER] startTrainerPuzzle | puzzle=${puzzle.id} idx=${_trainerPuzzleIdx} fast=${!!fast} isFast=${isFast} drillMode=${_trainerDrillMode} visited=${_trainerVisitedLeaves.size} total=${_trainerTotalLeaves} completed=${_trainerVariationsCompleted}`);
  clearTimeout(_trainerAnimTimer);
  const gen = ++_trainerGen;
  _trainerSolverColor = puzzle.solver_color === 'w' ? 'white' : 'black';
  _trainerMoveHistory = [];
  _trainerMistakeThisRun = false;
  _trainerPhase = 'show_mistake';

  _trainerGame = new Chess(puzzle.pre_mistake_fen);
  setTrainerBoard(puzzle.pre_mistake_fen, _trainerSolverColor, false);
  renderTrainerMoves();
  updateTrainerProgress();
  updateTrainerButtons();

  if (!isFast) {
    setTrainerStatus('Watch...', 'Your opponent is about to blunder.', 'info');
  } else {
    setTrainerStatus('Next variation', `Opponent plays ${puzzle.mistake_san}...`, 'info');
  }

  const delay = isFast ? 600 : 2000;
  _trainerAnimTimer = setTimeout(() => {
    if (gen !== _trainerGen || _trainerPhase !== 'show_mistake') return;
    const uci = puzzle.mistake_uci;
    try {
      const moveResult = _trainerGame.move({ from: uci.slice(0, 2), to: uci.slice(2, 4), promotion: uci[4] || undefined });
      if (moveResult) {
        _trainerMoveHistory.push({ san: moveResult.san, type: 'mistake' });
        renderTrainerMoves();
      }
    } catch (e) {
      console.warn(`[TRAINER] mistake move failed (stale timeout?): ${uci}`, e);
      return;
    }
    _trainerNode = puzzle.tree;
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, true);
    _trainerPhase = 'solver_turn';
    setTrainerStatus('Punish the mistake!', `They played ${puzzle.mistake_san}. Find the best response!`, 'info');
    _trainerIsFirstAttempt = false;
    updateTrainerButtons();
  }, delay);
}

function trainerOnMove(orig, dest) {
  if (_trainerPhase !== 'solver_turn' || !_trainerNode || !_trainerNode.moves) return;

  let uci = orig + dest;
  let moveData = _trainerNode.moves[uci];
  let promoChar = '';
  if (!moveData) {
    for (const p of ['q', 'r', 'b', 'n']) {
      if (_trainerNode.moves[uci + p]) { moveData = _trainerNode.moves[uci + p]; promoChar = p; break; }
    }
  }

  if (!moveData || !moveData.accepted) {
    // Wrong move — Chessground already moved the piece visually
    console.log(`[TRAINER] WRONG MOVE | uci=${uci} accepted=${moveData?.accepted} phase=${_trainerPhase}`);
    _trainerMistakeThisRun = true;
    _trainerPhase = 'showing_correction';

    // Reset board to actual position (undo visual move)
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);
    setTrainerStatus('Wrong move', 'That\'s not the best response.', 'error');

    const corrGen = _trainerGen;
    setTimeout(() => {
      if (corrGen !== _trainerGen || _trainerPhase !== 'showing_correction') return;
      // Show the best accepted move on the board
      const bestUci = Object.keys(_trainerNode.moves).find(k => _trainerNode.moves[k].accepted);
      if (bestUci) {
        const bestData = _trainerNode.moves[bestUci];
        const showGame = new Chess(_trainerGame.fen());
        try {
          showGame.move({ from: bestUci.slice(0, 2), to: bestUci.slice(2, 4), promotion: bestUci[4] || undefined });
          setTrainerBoard(showGame.fen(), _trainerSolverColor, false);
        } catch {}
        setTrainerStatus('Wrong move', `The best move was ${bestData.san}. Now play it.`, 'error');
      }

      // Reset back so user can play the correct move
      setTimeout(() => {
        if (corrGen !== _trainerGen || _trainerPhase !== 'showing_correction') return;
        _trainerPhase = 'solver_turn';
        setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, true);
        setTrainerStatus('Try again', 'Play the correct move.', 'info');
        updateTrainerButtons();
      }, 3000);
    }, 1600);
    return;
  }

  // Correct move — apply to chess.js game state
  console.log(`[TRAINER] CORRECT MOVE | uci=${uci}${promoChar} san=${moveData.san} accepted=${moveData.accepted}`);
  const moveResult = _trainerGame.move({ from: orig, to: dest, promotion: promoChar || undefined });
  if (!moveResult) { setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, true); return; }

  _trainerMoveHistory.push({ san: moveResult.san, type: 'solver' });
  renderTrainerMoves();
  setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);
  const accepted = Object.values(_trainerNode.moves).filter(m => m.accepted);
  if (accepted.length === 1) {
    setTrainerStatus('Correct!', `${moveResult.san} — the only winning move.`, 'success');
  } else {
    const others = accepted.filter(m => m.san !== moveResult.san).map(m => m.san).join(', ');
    setTrainerStatus('Correct!', `${moveResult.san} — correct!${others ? ' Also good: ' + others : ''}`, 'success');
  }

  const nextNode = moveData.result;
  if (!nextNode || nextNode.type === 'cutoff') {
    console.log(`[TRAINER] REACHED CUTOFF | visited=${_trainerVisitedLeaves.size}`);
    const cGen = _trainerGen;
    setTimeout(() => { if (cGen === _trainerGen) trainerPuzzleComplete(nextNode ?? _trainerNode, 'Position won! Advantage secured.'); }, 1000);
    return;
  }
  if (nextNode.type === 'terminal') {
    console.log(`[TRAINER] REACHED TERMINAL (checkmate) | visited=${_trainerVisitedLeaves.size}`);
    const cGen = _trainerGen;
    setTimeout(() => { if (cGen === _trainerGen) trainerPuzzleComplete(nextNode, 'Checkmate! Brilliant!'); }, 1000);
    return;
  }

  if (nextNode.type === 'opponent') {
    _trainerPhase = 'opponent_thinking';
    playTrainerOpponentMove(nextNode);
  } else {
    _trainerNode = nextNode;
    _trainerPhase = 'solver_turn';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, true);
    setTrainerStatus('Your turn', 'Find the best move.', 'info');
    updateTrainerButtons();
  }
}

function playTrainerOpponentMove(node) {
  if (!node.moves) { trainerPuzzleComplete(node, 'Position won!'); return; }
  const computed = Object.entries(node.moves).filter(([, m]) => m.result);
  if (computed.length === 0) { trainerPuzzleComplete(node, 'Position won!'); return; }

  let pick;
  if (_trainerDrillMode === 'main') {
    // Main line: always pick the most popular move
    pick = computed.reduce((a, b) => ((a[1].games ?? 0) >= (b[1].games ?? 0) ? a : b));
  } else {
    // Deep mode: shuffled order, prefer unvisited branches
    let order = _trainerOpponentOrder.get(node);
    if (!order) {
      order = Object.keys(node.moves).slice();
      for (let i = order.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [order[i], order[j]] = [order[j], order[i]];
      }
      _trainerOpponentOrder.set(node, order);
    }
    const computedSet = new Set(computed.map(([u]) => u));
    pick = order
      .filter(u => computedSet.has(u))
      .map(u => [u, node.moves[u]])
      .find(([, m]) => hasUnvisitedLeaves(m.result, _trainerVisitedLeaves))
      ?? computed[0];
  }

  const [uci, moveData] = pick;
  console.log(`[TRAINER] OPPONENT PICKS | san=${moveData.san} uci=${uci} games=${moveData.games ?? '?'} mode=${_trainerDrillMode} computed=${computed.length} options=[${computed.map(([u,m]) => m.san).join(',')}]`);

  const oppGen = _trainerGen;
  setTimeout(() => {
    if (oppGen !== _trainerGen) return;
    let moveResult;
    try {
      moveResult = _trainerGame.move({ from: uci.slice(0, 2), to: uci.slice(2, 4), promotion: uci[4] || undefined });
    } catch (e) {
      console.warn(`[TRAINER] opponent move failed (stale timeout?): ${uci}`, e);
      return;
    }
    if (!moveResult) { trainerPuzzleComplete(node, 'Position won!'); return; }

    _trainerMoveHistory.push({ san: moveResult.san, type: 'opponent' });
    renderTrainerMoves();

    const result = moveData.result;
    if (result.type === 'cutoff') {
      trainerPuzzleComplete(result, `Opponent played ${moveData.san}. Advantage secured!`);
      return;
    }
    if (result.type === 'terminal') {
      trainerPuzzleComplete(result, `Opponent played ${moveData.san}. Game over!`);
      return;
    }

    _trainerNode = result;
    _trainerPhase = 'solver_turn';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, true);
    const gamesNote = moveData.games && moveData.games > 0 ? ` (${moveData.games} games)` : ' (engine)';
    setTrainerStatus('Your turn', `Opponent played ${moveData.san}${gamesNote}. Find the best response.`, 'info');
    updateTrainerButtons();
  }, 1400);
}

function trainerPuzzleComplete(leaf, message) {
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  const total = _trainerTotalLeaves;
  console.log(`[TRAINER] puzzleComplete | mistake=${_trainerMistakeThisRun} visited=${_trainerVisitedLeaves.size} total=${total} msg="${message}"`);

  // If mistake was made this run, DON'T mark leaf as visited — it stays unvisited for retry
  if (_trainerMistakeThisRun) {
    console.log(`[TRAINER] MISTAKE — not marking leaf, will retry this variation`);
    _trainerPhase = 'done';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);
    setTrainerStatus('Mistake — retrying this line', 'You made a mistake. Let\'s retry just this variation.', 'info');
    clearTimeout(_trainerRestartTimer);
    _trainerRestartTimer = setTimeout(() => startTrainerPuzzle(true), 3000);
    updateTrainerButtons();
    return;
  }

  // No mistake — mark leaf + sibling leaves as visited
  markSiblingLeaves(_trainerNode, _trainerVisitedLeaves);
  if (leaf) _trainerVisitedLeaves.add(leaf);
  const completed = puzzle ? countVisitedLeaves(puzzle.tree, _trainerVisitedLeaves) : _trainerVisitedLeaves.size;
  _trainerVariationsCompleted = completed;
  console.log(`[TRAINER] leaf marked visited | setSize=${_trainerVisitedLeaves.size} countedVisited=${completed} total=${total}`);

  if (completed >= total) {
    // Truly done — fire completion
    _trainerPhase = 'done';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);

    if (_trainerDrillMode === 'main') {
      // Mark puzzle complete on main line finish
      if (puzzle && !_trainerCompletedIds.has(puzzle.id)) {
        _trainerCompletedIds.add(puzzle.id);
        updateTrainerProgress();
        const token = localStorage.getItem('alpine_token');
        if (token) {
          fetch(API_URL + '/api/trainer/progress', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
            body: JSON.stringify({ puzzle_id: puzzle.id }),
          }).catch(() => {});
        }
      }
    }

    const deepCount = _trainerDrillMode === 'deep' ? total - _trainerDeepStartVisited : total;
    const doneMsg = _trainerDrillMode === 'main' ? 'Main line complete!' : `Completed all ${deepCount} variation${deepCount !== 1 ? 's' : ''}.`;
    setTrainerStatus(doneMsg, message || 'Well done!', 'success');
    updateTrainerButtons();
  } else {
    // More variations remain — show status, then auto-restart
    _trainerPhase = 'done';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);
    setTrainerStatus(`Variation ${completed}/${total} complete!`, message || '', 'success');
    clearTimeout(_trainerRestartTimer);
    _trainerRestartTimer = setTimeout(() => startTrainerPuzzle(true), 3000);
    updateTrainerButtons();
  }
}

function renderTrainerMoves() {
  const ml = document.getElementById('trainerMoveList');
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  if (!puzzle) { ml.innerHTML = ''; return; }

  const parts = puzzle.pre_mistake_fen.split(' ');
  const turnAtMistake = parts[1]; // 'w' or 'b'
  let startMoveNum = parseInt(parts[5]) || 1;

  let html = '';
  for (let i = 0; i < _trainerMoveHistory.length; i++) {
    const m = _trainerMoveHistory[i];
    const cls = m.type === 'mistake' ? 'text-bad' : m.type === 'solver' ? 'text-good' : 'text-move-inaccuracy';
    const isWhite = (turnAtMistake === 'w') ? (i % 2 === 0) : (i % 2 === 1);
    const mn = startMoveNum + Math.floor((i + (turnAtMistake === 'b' ? 1 : 0)) / 2);

    if (isWhite) html += `<span class="text-secondary mr-1">${mn}.</span>`;
    else if (i === 0) html += `<span class="text-secondary mr-1">${mn}...</span>`;
    html += `<span class="${cls} font-semibold">${m.san}</span> `;
  }
  ml.innerHTML = html;
  ml.scrollTop = ml.scrollHeight;
}

function trainerHint() {
  if (_trainerIsHardMoveMode) return hmHint();
  if (_trainerPhase !== 'solver_turn' || !_trainerNode || !_trainerNode.moves) return;
  const bestUci = Object.keys(_trainerNode.moves).find(k => _trainerNode.moves[k].accepted);
  if (!bestUci) return;
  const from = bestUci.slice(0, 2);
  // Find the square element in the board and highlight it
  const boardEl = document.getElementById('trainerBoard');
  if (!boardEl) return;
  const sq = boardEl.querySelector(`square[class*="${from}"]`) || boardEl.querySelector(`.${from}`);
  // Use overlay div positioned over the square
  const existing = boardEl.querySelector('.trainer-hint-highlight');
  if (existing) existing.remove();
  const file = from.charCodeAt(0) - 97; // a=0, b=1, ...
  const rank = parseInt(from[1]) - 1;   // 1=0, 2=1, ...
  const isFlipped = _trainerSolverColor === 'black';
  const x = isFlipped ? (7 - file) : file;
  const y = isFlipped ? rank : (7 - rank);
  const highlight = document.createElement('div');
  highlight.className = 'trainer-hint-highlight';
  highlight.style.cssText = `position:absolute; left:${x * 12.5}%; top:${y * 12.5}%; width:12.5%; height:12.5%; background:rgba(21,189,89,0.45); border-radius:50%; z-index:10; pointer-events:none; transition:opacity 0.5s;`;
  const cgWrap = boardEl.querySelector('cg-container') || boardEl;
  cgWrap.style.position = 'relative';
  cgWrap.appendChild(highlight);
  setTimeout(() => {
    highlight.style.opacity = '0';
    setTimeout(() => highlight.remove(), 500);
  }, 5000);
}

function trainerSkip() {
  console.log(`[TRAINER] trainerSkip | mode=${_trainerDrillMode} total=${_trainerTotalLeaves} visited=${_trainerVisitedLeaves.size}`);
  if (_trainerDrillMode !== 'deep' || _trainerTotalLeaves <= 1) return;
  // Find current leaf by walking the tree along visited path, then mark it visited
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  if (!puzzle) return;
  // Walk to find current leaf and mark it + siblings
  function findAndMarkLeaf(node) {
    if (!node || node.type === 'cutoff' || node.type === 'terminal' || !node.moves) {
      if (node) _trainerVisitedLeaves.add(node);
      return;
    }
    const entries = Object.values(node.moves);
    if (entries.length === 0) { _trainerVisitedLeaves.add(node); return; }
    if (node.type === 'opponent') {
      // Find the unvisited branch to mark
      for (const m of entries) {
        if (m.result && hasUnvisitedLeaves(m.result, _trainerVisitedLeaves)) {
          findAndMarkLeaf(m.result);
          return;
        }
      }
      // All visited — mark first
      if (entries[0] && entries[0].result) findAndMarkLeaf(entries[0].result);
    } else {
      // Solver node — mark sibling leaves + walk deeper
      markSiblingLeaves(node, _trainerVisitedLeaves);
      for (const m of entries) {
        if (!m.accepted) continue;
        if (m.result && hasUnvisitedLeaves(m.result, _trainerVisitedLeaves)) {
          findAndMarkLeaf(m.result);
          return;
        }
      }
    }
  }
  findAndMarkLeaf(puzzle.tree);

  const completed = countVisitedLeaves(puzzle.tree, _trainerVisitedLeaves);
  _trainerVariationsCompleted = completed;

  if (completed >= _trainerTotalLeaves) {
    _trainerPhase = 'done';
    setTrainerBoard(_trainerGame.fen(), _trainerSolverColor, false);
    setTrainerStatus(`Completed all ${_trainerTotalLeaves} variations`, 'All variations done!', 'success');
    updateTrainerButtons();
  } else {
    clearTimeout(_trainerRestartTimer);
    startTrainerPuzzle(true);
  }
}

function trainerPrevPuzzle() {
  if (_trainerIsHardMoveMode) return hmPrevPuzzle();
  clearTimeout(_trainerRestartTimer);
  _trainerPuzzleIdx = (_trainerPuzzleIdx - 1 + _trainerPuzzles.length) % _trainerPuzzles.length;
  resetTrainerVariationState();
  startTrainerPuzzle();
}

function trainerNextPuzzle() {
  if (_trainerIsHardMoveMode) return hmNextPuzzle();
  clearTimeout(_trainerRestartTimer);
  _trainerPuzzleIdx = (_trainerPuzzleIdx + 1) % _trainerPuzzles.length;
  resetTrainerVariationState();
  startTrainerPuzzle();
}

function trainerRetry() {
  if (_trainerIsHardMoveMode) return hmRetry();
  resetTrainerVariationState();
  startTrainerPuzzle();
}

function trainerNext() {
  if (_trainerIsHardMoveMode) return hmNext();
  let next = -1;
  for (let i = _trainerPuzzleIdx + 1; i < _trainerPuzzles.length; i++) {
    if (!_trainerCompletedIds.has(_trainerPuzzles[i].id)) { next = i; break; }
  }
  if (next < 0) {
    for (let i = 0; i < _trainerPuzzleIdx; i++) {
      if (!_trainerCompletedIds.has(_trainerPuzzles[i].id)) { next = i; break; }
    }
  }
  if (next < 0) next = (_trainerPuzzleIdx + 1) % _trainerPuzzles.length;
  _trainerPuzzleIdx = next;
  resetTrainerVariationState();
  startTrainerPuzzle();
}

function resetTrainerVariationState() {
  console.log(`[TRAINER] resetTrainerVariationState`);
  clearTimeout(_trainerRestartTimer);
  _trainerMistakeThisRun = false;
  _trainerVisitedLeaves.clear();
  _trainerOpponentOrder.clear();
  _trainerDrillMode = 'main';
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  _trainerTotalLeaves = puzzle ? countMainLineLeaves(puzzle.tree) : 1;
  _trainerVariationsCompleted = 0;
  _trainerDeepStartVisited = 0;
  _trainerIsFirstAttempt = true;
}

function startDeepDrill() {
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  if (!puzzle) return;
  // Keep visited leaves from main line — don't replay what's already done
  console.log(`[TRAINER] startDeepDrill | puzzle=${puzzle.id} keepingVisited=${_trainerVisitedLeaves.size} totalLeaves=${countLeaves(puzzle.tree)}`);
  _trainerOpponentOrder.clear();
  _trainerDrillMode = 'deep';
  _trainerTotalLeaves = countLeaves(puzzle.tree);
  _trainerVariationsCompleted = countVisitedLeaves(puzzle.tree, _trainerVisitedLeaves);
  _trainerDeepStartVisited = _trainerVariationsCompleted;
  _trainerIsFirstAttempt = true;
  _trainerMistakeThisRun = false;
  setTimeout(() => startTrainerPuzzle(), 600);
}

// ══════════════════════════════════════════
// HARD MOVE DRILL ENGINE
// ══════════════════════════════════════════
let _hmMoves = [];
let _hmCompletedIds = new Set();
let _hmIdx = 0;
let _hmPhase = 'idle';
let _hmOpeningName = '';
let _hmSolverColor = 'white';
let _hmMistakeThisRun = false;
let _hmAnimTimer = null;
let _hmRestartTimer = null;
let _hmGen = 0;
let _trainerIsHardMoveMode = false;

function _formatCp(cp) {
  return (cp >= 0 ? '+' : '') + (cp / 100).toFixed(1);
}

function setHmStatus(title, msg, type) {
  const titleEl = document.getElementById('hmStatusTitle');
  const msgEl = document.getElementById('hmStatusMsg');
  titleEl.textContent = title;
  msgEl.textContent = msg || '';
  titleEl.className = 'text-sm font-semibold' + (
    type === 'success' ? ' text-good' :
    type === 'error' ? ' text-bad' :
    ' text-slate-300'
  );
}

async function openHardMoveOpening(name) {
  _hmOpeningName = name;
  _trainerIsHardMoveMode = true;
  const token = localStorage.getItem('alpine_token');
  if (!token || !Chess) return;

  document.getElementById('trainerSelectView').style.display = 'none';
  document.getElementById('hmDrillView').style.display = '';
  setHmStatus('Loading...', '', 'info');

  try {
    const res = await fetch(API_URL + '/api/trainer/hard-moves?opening=' + encodeURIComponent(name), {
      headers: { 'Authorization': 'Bearer ' + token },
    });
    if (!res.ok) throw new Error('Failed to load');
    const data = await res.json();
    _hmMoves = data.moves || [];
    _hmCompletedIds = new Set(data.completed_ids || []);
  } catch (err) {
    setHmStatus('Error', err.message, 'error');
    return;
  }

  if (_hmMoves.length === 0) {
    setHmStatus('No positions', 'No positions available for this opening.', 'info');
    return;
  }

  _hmIdx = _hmMoves.findIndex(m => !_hmCompletedIds.has(m.id));
  if (_hmIdx < 0) _hmIdx = 0;
  updateHmProgress();
  startHardMove();
}

function updateHmProgress() {
  const total = _hmMoves.length;
  const done = _hmCompletedIds.size;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;
  document.getElementById('hmProgressLabel').textContent = `${done}/${total} completed`;
  document.getElementById('hmProgressPct').textContent = `${pct}%`;
  document.getElementById('hmProgressBar').style.width = `${pct}%`;
  document.getElementById('hmDrillCounter').textContent = `${_hmOpeningName} — Position ${_hmIdx + 1} / ${total}`;
}

function startHardMove() {
  const hm = _hmMoves[_hmIdx];
  if (!hm) return;

  clearTimeout(_hmAnimTimer);
  clearTimeout(_hmRestartTimer);
  const gen = ++_hmGen;
  _hmSolverColor = hm.side === 'w' ? 'white' : 'black';
  _hmMistakeThisRun = false;
  _hmPhase = 'show_mistake';

  _trainerGame = new Chess(hm.fen);
  setTrainerBoard(hm.fen, _hmSolverColor, false, 'hmBoard');
  updateHmProgress();
  updateHmButtons();

  const mistakePct = hm.mistake_maia_pct ? `${Math.round(hm.mistake_maia_pct)}%` : 'Many';
  const bestEval = _formatCp(hm.best_eval_cp);
  const mistakeEval = _formatCp(hm.mistake_eval_cp);
  setHmStatus('Common mistake...', `${mistakePct} of players play ${hm.mistake_move} here.`, 'info');

  // Pause to let user see the position, then animate the mistake
  _hmAnimTimer = setTimeout(() => {
    if (gen !== _hmGen || _hmPhase !== 'show_mistake') return;
    let mistakeResult;
    try {
      mistakeResult = _trainerGame.move(hm.mistake_move);
    } catch (e) {
      console.warn('[HM] mistake move failed:', hm.mistake_move, e);
      return;
    }
    if (!mistakeResult) return;

    // Show the mistake on the board
    setTrainerBoard(_trainerGame.fen(), _hmSolverColor, false, 'hmBoard');

    setHmStatus('Common mistake', `${hm.mistake_move} shifts the eval from ${bestEval} to ${mistakeEval}.`, 'error');

    // Highlight the mistake squares AFTER board is rendered
    setTimeout(() => {
      const boardEl = document.getElementById('hmBoard');
      if (!boardEl) return;
      const cgWrap = boardEl.querySelector('cg-container') || boardEl;
      cgWrap.style.position = 'relative';
      for (const sq of [mistakeResult.from, mistakeResult.to]) {
        const file = sq.charCodeAt(0) - 97;
        const rank = parseInt(sq[1]) - 1;
        const isFlipped = _hmSolverColor === 'black';
        const x = isFlipped ? (7 - file) : file;
        const y = isFlipped ? rank : (7 - rank);
        const hl = document.createElement('div');
        hl.className = 'hm-mistake-highlight';
        hl.style.cssText = `position:absolute; left:${x * 12.5}%; top:${y * 12.5}%; width:12.5%; height:12.5%; background:rgba(220,38,38,0.75); box-shadow: inset 0 0 8px rgba(239,68,68,0.9); z-index:10; pointer-events:none; animation: hm-pulse 1s ease-in-out infinite;`;
        cgWrap.appendChild(hl);
      }
    }, 100);

    // Hold the mistake on screen, then undo and prompt user
    _hmAnimTimer = setTimeout(() => {
      if (gen !== _hmGen || _hmPhase !== 'show_mistake') return;
      _trainerGame.undo();
      _hmPhase = 'solver_turn';
      setTrainerBoard(hm.fen, _hmSolverColor, true, 'hmBoard');
      setHmStatus('Find the better move!', `${hm.mistake_move} shifts eval to ${mistakeEval}. Find the move that keeps it at ${bestEval}.`, 'info');
      updateHmButtons();
    }, 3500);
  }, 2000);
}

function renderHmMoves(hm, currentSan, isCorrect) {
  const ml = document.getElementById('trainerMoveList');
  if (!hm) { ml.innerHTML = ''; return; }

  // Show opening context from sequence (excluding last move = mistake)
  const allMoves = hm.sequence ? hm.sequence.split('|') : [];
  const contextMoves = allMoves.length > 0 ? allMoves.slice(0, -1) : [];

  let html = '';
  for (let i = 0; i < contextMoves.length; i++) {
    if (i % 2 === 0) html += `<span class="text-secondary mr-1">${Math.floor(i / 2) + 1}.</span>`;
    html += `<span class="text-slate-500">${contextMoves[i]}</span> `;
  }

  if (currentSan) {
    const moveNum = Math.floor(contextMoves.length / 2) + 1;
    if (contextMoves.length % 2 === 0) {
      html += `<span class="text-secondary mr-1">${moveNum}.</span>`;
    } else {
      html += `<span class="text-secondary mr-1">${moveNum}...</span>`;
    }
    const cls = isCorrect ? 'text-good font-semibold' : 'text-bad font-semibold';
    html += `<span class="${cls}">${currentSan}</span>`;
  }

  ml.innerHTML = html;
  ml.scrollTop = ml.scrollHeight;
}

function hmOnMove(orig, dest) {
  if (_hmPhase !== 'solver_turn') return;
  const hm = _hmMoves[_hmIdx];
  if (!hm) return;

  // Try to make the move in chess.js
  let moveResult;
  for (const promo of [undefined, 'q', 'r', 'b', 'n']) {
    try {
      moveResult = _trainerGame.move({ from: orig, to: dest, promotion: promo });
      if (moveResult) break;
    } catch {}
  }
  if (!moveResult) {
    setTrainerBoard(hm.fen, _hmSolverColor, true, 'hmBoard');
    return;
  }

  const isCorrect = moveResult.san === hm.best_move;
  _trainerGame.undo();

  if (isCorrect) {
    _hmPhase = 'done';
    _trainerGame.move(hm.best_move);
    setTrainerBoard(_trainerGame.fen(), _hmSolverColor, false, 'hmBoard');

    const maiaPct = hm.best_maia_pct ? ` Only ${hm.best_maia_pct}% of players find this move.` : '';
    setHmStatus('Correct!', `${hm.best_move} is the best move!${maiaPct}`, 'success');

    // Mark complete
    if (!_hmCompletedIds.has(hm.id)) {
      _hmCompletedIds.add(hm.id);
      updateHmProgress();
      const token = localStorage.getItem('alpine_token');
      if (token) {
        fetch(API_URL + '/api/trainer/hard-moves/progress', {
          method: 'POST',
          headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
          body: JSON.stringify({ puzzle_id: hm.id }),
        }).catch(() => {});
      }
    }
    updateHmButtons();
  } else {
    // Wrong move
    _hmMistakeThisRun = true;
    _hmPhase = 'showing_correction';
    setTrainerBoard(hm.fen, _hmSolverColor, false, 'hmBoard');
    setHmStatus('Wrong move', `You played ${moveResult.san}. That's not the best move.`, 'error');

    const corrGen = _hmGen;
    _hmAnimTimer = setTimeout(() => {
      if (corrGen !== _hmGen || _hmPhase !== 'showing_correction') return;
      const showGame = new Chess(hm.fen);
      try {
        showGame.move(hm.best_move);
        setTrainerBoard(showGame.fen(), _hmSolverColor, false, 'hmBoard');
      } catch {}
      setHmStatus('Wrong move', `The best move was ${hm.best_move}. Try again.`, 'error');

      _hmAnimTimer = setTimeout(() => {
        if (corrGen !== _hmGen || _hmPhase !== 'showing_correction') return;
        _trainerGame = new Chess(hm.fen);
        _hmPhase = 'solver_turn';
        setTrainerBoard(hm.fen, _hmSolverColor, true, 'hmBoard');
        setHmStatus('Try again', 'Find the best move.', 'info');
        updateHmButtons();
      }, 2500);
    }, 1500);
  }
}

function hmHint() {
  if (_hmPhase !== 'solver_turn') return;
  const hm = _hmMoves[_hmIdx];
  if (!hm) return;

  const game = new Chess(hm.fen);
  let bestResult;
  try { bestResult = game.move(hm.best_move); } catch { return; }
  if (!bestResult) return;

  const from = bestResult.from;
  const boardEl = document.getElementById('hmBoard');
  if (!boardEl) return;

  const existing = boardEl.querySelector('.trainer-hint-highlight');
  if (existing) existing.remove();

  const file = from.charCodeAt(0) - 97;
  const rank = parseInt(from[1]) - 1;
  const isFlipped = _hmSolverColor === 'black';
  const x = isFlipped ? (7 - file) : file;
  const y = isFlipped ? rank : (7 - rank);
  const highlight = document.createElement('div');
  highlight.className = 'trainer-hint-highlight';
  highlight.style.cssText = `position:absolute; left:${x * 12.5}%; top:${y * 12.5}%; width:12.5%; height:12.5%; background:rgba(21,189,89,0.45); border-radius:50%; z-index:10; pointer-events:none; transition:opacity 0.5s;`;
  const cgWrap = boardEl.querySelector('cg-container') || boardEl;
  cgWrap.style.position = 'relative';
  cgWrap.appendChild(highlight);
  setTimeout(() => {
    highlight.style.opacity = '0';
    setTimeout(() => highlight.remove(), 500);
  }, 5000);
}

function hmPrevPuzzle() {
  clearTimeout(_hmAnimTimer);
  clearTimeout(_hmRestartTimer);
  _hmIdx = (_hmIdx - 1 + _hmMoves.length) % _hmMoves.length;
  startHardMove();
}

function hmNextPuzzle() {
  clearTimeout(_hmAnimTimer);
  clearTimeout(_hmRestartTimer);
  _hmIdx = (_hmIdx + 1) % _hmMoves.length;
  startHardMove();
}

function hmRetry() {
  startHardMove();
}

function hmNext() {
  let next = -1;
  for (let i = _hmIdx + 1; i < _hmMoves.length; i++) {
    if (!_hmCompletedIds.has(_hmMoves[i].id)) { next = i; break; }
  }
  if (next < 0) {
    for (let i = 0; i < _hmIdx; i++) {
      if (!_hmCompletedIds.has(_hmMoves[i].id)) { next = i; break; }
    }
  }
  if (next < 0) next = (_hmIdx + 1) % _hmMoves.length;
  _hmIdx = next;
  startHardMove();
}

function updateHmButtons() {
  const allDone = _hmPhase === 'done';
  const hasNext = _hmMoves.length > 1;
  document.getElementById('btnHmHint').style.display = _hmPhase === 'solver_turn' ? '' : 'none';
  document.getElementById('btnHmRetry').style.display = allDone ? '' : 'none';
  document.getElementById('btnHmNext').style.display = (allDone && hasNext) ? '' : 'none';
}

function updateTrainerButtons() {
  const puzzle = _trainerPuzzles[_trainerPuzzleIdx];
  const allDone = _trainerVariationsCompleted >= _trainerTotalLeaves;
  const hasNext = _trainerPuzzles.length > 1;

  // Hint: only during solver_turn
  document.getElementById('btnTrainerHint').style.display = _trainerPhase === 'solver_turn' ? '' : 'none';
  // Skip: only during deep drill when solving and more than 1 variation
  document.getElementById('btnTrainerSkip').style.display = (_trainerDrillMode === 'deep' && _trainerTotalLeaves > 1 && (_trainerPhase === 'solver_turn' || _trainerPhase === 'opponent_thinking')) ? '' : 'none';
  // Retry: only when fully done
  document.getElementById('btnTrainerRetry').style.display = (_trainerPhase === 'done' && allDone) ? '' : 'none';
  // Next: only when fully done and has more puzzles
  document.getElementById('btnTrainerNext').style.display = (_trainerPhase === 'done' && allDone && hasNext) ? '' : 'none';

  // Drill Deeper: only when main line done and tree has deep variations
  const deepBtn = document.getElementById('btnTrainerDeepDrill');
  if (deepBtn) {
    const showDeep = _trainerPhase === 'done' && allDone && _trainerDrillMode === 'main' && puzzle && treeHasDeepVariations(puzzle.tree);
    deepBtn.style.display = showDeep ? '' : 'none';
    if (showDeep) {
      const remaining = countLeaves(puzzle.tree) - countVisitedLeaves(puzzle.tree, _trainerVisitedLeaves);
      deepBtn.textContent = `Drill Deeper (${remaining} variation${remaining !== 1 ? 's' : ''})`;
    }
  }

  // Variation counter
  const varCounter = document.getElementById('trainerVarCounter');
  if (varCounter) {
    if (_trainerDrillMode === 'deep' && _trainerTotalLeaves > 1) {
      const deepTotal = _trainerTotalLeaves - _trainerDeepStartVisited;
      const deepDone = _trainerVariationsCompleted - _trainerDeepStartVisited;
      const current = Math.min(deepDone + (_trainerPhase !== 'done' && _trainerPhase !== 'idle' ? 1 : 0), deepTotal);
      varCounter.textContent = `Variation ${current} / ${deepTotal}`;
      varCounter.style.display = '';
    } else if (_trainerDrillMode === 'main') {
      varCounter.textContent = 'Main line';
      varCounter.style.display = '';
    } else {
      varCounter.style.display = 'none';
    }
  }
}
