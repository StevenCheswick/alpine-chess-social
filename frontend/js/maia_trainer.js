// ══════════════════════════════════════════
// MAIA PLAY-VS-BOT TRAINER CARDS
// Lives alongside the existing opening trainer cards in #trainerGrid.
// Drill view: play out a position from a fixed FEN against Maia at a chosen rating.
// ══════════════════════════════════════════

let _maiaPositions = [];
let _maiaEngine = null;
let _maiaCurrent = null;       // { id, title, fen, user_side, notes }
let _maiaChess = null;         // chess.js game
let _maiaCg = null;            // chessground instance
let _maiaGameActive = false;
let _maiaFenHistory = [];
let _maiaRating = 1500;
let _maiaPositionIdx = 0;

async function fetchMaiaPositions() {
  try {
    const r = await fetch(API_URL + '/api/trainer/maia-positions');
    if (!r.ok) return [];
    return await r.json();
  } catch { return []; }
}

function maiaCardHtml(p) {
  const flip = p.user_side === 'black';
  const boardHtml = fenToMiniBoard(p.fen, flip);
  const sideLabel = p.user_side === 'white' ? 'Play White' : 'Play Black';
  return `
    <div class="card p-0 cursor-pointer transition-all hover:border-rose-400/40 group"
         style="border-radius:10px"
         onclick="openMaiaPosition('${p.id.replace(/'/g, "\\'")}')">
      ${boardHtml}
      <div class="p-3">
        <div class="flex items-center gap-1.5 mb-0.5">
          <span class="text-body text-white font-medium">${p.title}</span>
        </div>
        <span class="inline-block text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-400 mb-1.5">
          Critical Position
        </span>
        <div class="flex items-center justify-between">
          <span class="text-meta text-muted">${sideLabel}</span>
          <span class="text-meta text-muted">1100–1900</span>
        </div>
      </div>
    </div>`;
}

function renderMaiaCards() {
  if (!_maiaPositions.length) return '';
  return _maiaPositions.map(maiaCardHtml).join('');
}

async function openMaiaPosition(id) {
  const pos = _maiaPositions.find(p => p.id === id);
  if (!pos) return;
  _maiaCurrent = pos;
  _maiaPositionIdx = _maiaPositions.indexOf(pos);
  if (_maiaPositionIdx < 0) _maiaPositionIdx = 0;
  updateMaiaCounter();

  document.getElementById('maiaDrillTitle').textContent = pos.title;
  document.getElementById('maiaDrillNotes').textContent = pos.notes || '';
  setMaiaStatus('Loading engine…', 'text-muted');

  // Lazy-init engine + ort script
  await ensureOrtLoaded();
  if (!_maiaEngine) {
    const mod = await import('./maia.js?v=2');
    _maiaEngine = new mod.MaiaEngine({
      modelsBaseUrl: 'models/maia',
      onProgress: ({ rating, loaded, total, phase }) => {
        if (phase === 'download' && total) {
          const pct = Math.round(loaded / total * 100);
          setMaiaStatus(`Loading Maia-${rating}… ${pct}%`, 'text-muted');
        } else if (phase === 'init') {
          setMaiaStatus(`Initializing Maia-${rating}…`, 'text-muted');
        }
      },
    });
  }

  _maiaRating = parseInt(document.getElementById('maiaRatingSelect').value, 10) || 1500;
  await _maiaEngine.loadModel(_maiaRating);

  startMaiaGame();
}

function exitMaiaDrill() {
  cleanupMaiaDrill();
  switchTreeTab('tree');
}

function cleanupMaiaDrill() {
  _maiaGameActive = false;
  if (_maiaCg) { try { _maiaCg.destroy(); } catch {} _maiaCg = null; }
}

function setMaiaStatus(text, cls) {
  const el = document.getElementById('maiaDrillStatus');
  el.textContent = text;
  el.className = 'text-sm ' + (cls || 'text-muted');
}

function startMaiaGame() {
  if (!Chess || !Chessground) {
    setMaiaStatus('Chess libraries still loading — try again in a moment', 'text-red-400');
    return;
  }
  const { fen, user_side } = _maiaCurrent;
  try {
    _maiaChess = new Chess(fen);
  } catch (e) {
    setMaiaStatus('Invalid FEN for this position', 'text-red-400');
    return;
  }
  _maiaFenHistory = [];
  _maiaGameActive = true;

  if (_maiaCg) { try { _maiaCg.destroy(); } catch {} }
  const boardEl = document.getElementById('maiaBoard');
  const turnColor = _maiaChess.turn() === 'w' ? 'white' : 'black';
  const isPlayerTurn = turnColor === user_side;

  _maiaCg = Chessground(boardEl, {
    fen: _maiaChess.fen(),
    orientation: user_side,
    turnColor,
    movable: {
      color: isPlayerTurn ? user_side : undefined,
      free: false,
      dests: isPlayerTurn ? maiaDests(_maiaChess) : new Map(),
      events: { after: onMaiaPlayerMove },
    },
    draggable: { showGhost: true },
    animation: { enabled: true, duration: 180 },
    check: _maiaChess.isCheck(),
  });

  if (isPlayerTurn) setMaiaStatus('Your turn', 'text-secondary');
  else maiaReply();
}

function maiaDests(game) {
  const dests = new Map();
  for (const m of game.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  return dests;
}

function maiaUpdateBoard() {
  if (!_maiaCg) return;
  const turnColor = _maiaChess.turn() === 'w' ? 'white' : 'black';
  const isPlayerTurn = turnColor === _maiaCurrent.user_side && _maiaGameActive;
  _maiaCg.set({
    fen: _maiaChess.fen(),
    turnColor,
    movable: {
      color: isPlayerTurn ? _maiaCurrent.user_side : undefined,
      free: false,
      dests: isPlayerTurn ? maiaDests(_maiaChess) : new Map(),
    },
    check: _maiaChess.isCheck(),
  });
}

function maiaCheckGameOver() {
  if (_maiaChess.isCheckmate()) {
    const loser = _maiaChess.turn(); // side to move lost
    const youLost = (loser === 'w' && _maiaCurrent.user_side === 'white')
                 || (loser === 'b' && _maiaCurrent.user_side === 'black');
    setMaiaStatus(youLost ? 'Checkmate — Maia wins' : 'Checkmate — you win', youLost ? 'text-red-400' : 'text-good');
    _maiaGameActive = false;
    return true;
  }
  if (_maiaChess.isStalemate()) { setMaiaStatus('Draw — stalemate', 'text-yellow-400'); _maiaGameActive = false; return true; }
  if (_maiaChess.isThreefoldRepetition()) { setMaiaStatus('Draw — threefold repetition', 'text-yellow-400'); _maiaGameActive = false; return true; }
  if (_maiaChess.isInsufficientMaterial()) { setMaiaStatus('Draw — insufficient material', 'text-yellow-400'); _maiaGameActive = false; return true; }
  if (_maiaChess.isDraw()) { setMaiaStatus('Draw', 'text-yellow-400'); _maiaGameActive = false; return true; }
  return false;
}

async function onMaiaPlayerMove(from, to) {
  if (!_maiaGameActive) return;
  let promotion;
  const piece = _maiaChess.get(from);
  if (piece && piece.type === 'p') {
    const destRank = parseInt(to[1], 10);
    if ((piece.color === 'w' && destRank === 8) || (piece.color === 'b' && destRank === 1)) promotion = 'q';
  }
  _maiaFenHistory.push(_maiaChess.fen());
  const mv = _maiaChess.move({ from, to, promotion });
  if (!mv) { maiaUpdateBoard(); return; }
  playSound(soundForMove(_maiaChess, mv));
  maiaUpdateBoard();
  if (maiaCheckGameOver()) return;
  await maiaReply();
}

async function maiaReply() {
  setMaiaStatus(`Maia-${_maiaRating} is thinking…`, 'text-muted');
  await new Promise(r => setTimeout(r, 60));
  try {
    const result = await _maiaEngine.pickMove({
      chess: _maiaChess,
      historyFens: _maiaFenHistory.slice(-7).reverse(),
      rating: _maiaRating,
    });
    if (!result) { maiaCheckGameOver(); return; }
    const m = result.chosen.move;
    _maiaFenHistory.push(_maiaChess.fen());
    const mv = _maiaChess.move({ from: m.from, to: m.to, promotion: m.promotion });
    if (!mv) { setMaiaStatus('Maia returned an illegal move', 'text-red-400'); return; }
    playSound(soundForMove(_maiaChess, mv));
    if (_maiaCg) _maiaCg.move(m.from, m.to);
    maiaUpdateBoard();
    if (maiaCheckGameOver()) return;
    setMaiaStatus('Your turn', 'text-secondary');
  } catch (err) {
    console.error('Maia inference error:', err);
    setMaiaStatus('Engine error: ' + err.message, 'text-red-400');
  }
}

async function onMaiaRatingChange(e) {
  _maiaRating = parseInt(e.target.value, 10) || 1500;
  if (!_maiaEngine) return;
  setMaiaStatus(`Loading Maia-${_maiaRating}…`, 'text-muted');
  try {
    await _maiaEngine.loadModel(_maiaRating);
    setMaiaStatus(`Maia-${_maiaRating} ready`, 'text-muted');
  } catch (err) {
    setMaiaStatus('Failed to load model: ' + err.message, 'text-red-400');
  }
}

function ensureOrtLoaded() {
  if (typeof ort !== 'undefined') return Promise.resolve();
  return new Promise((resolve, reject) => {
    const s = document.createElement('script');
    s.src = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.21.0/dist/ort.min.js';
    s.onload = resolve;
    s.onerror = () => reject(new Error('Failed to load onnxruntime-web'));
    document.head.appendChild(s);
  });
}

// Hook into the existing initTrainer: after it renders its own cards,
// fetch maia positions and append.
async function initMaiaTrainerCards() {
  _maiaPositions = await fetchMaiaPositions();
  if (!_maiaPositions.length) return;
  const grid = document.getElementById('trainerGrid');
  if (!grid) return;
  // Remove the "no trainers" placeholder if it exists (empty first-load case).
  if (grid.querySelector('.col-span-3')) grid.innerHTML = '';
  grid.insertAdjacentHTML('beforeend', renderMaiaCards());
}

function updateMaiaCounter() {
  const el = document.getElementById('maiaDrillCounter');
  if (el) el.textContent = `Position ${_maiaPositionIdx + 1} / ${_maiaPositions.length}`;
}

function maiaPrevPosition() {
  if (_maiaPositions.length <= 1) return;
  cleanupMaiaDrill();
  _maiaPositionIdx = (_maiaPositionIdx - 1 + _maiaPositions.length) % _maiaPositions.length;
  openMaiaPosition(_maiaPositions[_maiaPositionIdx].id);
}

function maiaNextPosition() {
  if (_maiaPositions.length <= 1) return;
  cleanupMaiaDrill();
  _maiaPositionIdx = (_maiaPositionIdx + 1) % _maiaPositions.length;
  openMaiaPosition(_maiaPositions[_maiaPositionIdx].id);
}

// Expose to window for onclick= handlers and for trainer.js to invoke.
window.openMaiaPosition = openMaiaPosition;
window.exitMaiaDrill = exitMaiaDrill;
window.onMaiaRatingChange = onMaiaRatingChange;
window.initMaiaTrainerCards = initMaiaTrainerCards;
window.maiaPrevPosition = maiaPrevPosition;
window.maiaNextPosition = maiaNextPosition;
window.cleanupMaiaDrill = cleanupMaiaDrill;
