// ══════════════════════════════════════════
// OPENING TREE VIEWER (Chessable-style trees)
// ══════════════════════════════════════════
let _treeData = null;
let _treePath = [];          // array of nodes from root to current
let _treeCgInstance = null;
let _treeFlipped = false;
let _treeTab = 'tree';        // 'tree' | 'learn' | 'train'
let _treeLearnedSet = new Set();  // "fen|san" pairs the user has learned

function learnedKey(fen, san) { return fen + '|' + san; }

async function fetchTreeProgress(treeId) {
  const token = localStorage.getItem('alpine_token');
  if (!token) return [];
  try {
    const r = await fetch(API_URL + '/api/trainer/trees/' + encodeURIComponent(treeId) + '/progress', { headers: { 'Authorization': 'Bearer ' + token } });
    if (!r.ok) return [];
    const data = await r.json();
    return data.learned || [];
  } catch { return []; }
}

async function postTreeProgress(treeId, moves) {
  if (!moves.length) return;
  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  try {
    await fetch(API_URL + '/api/trainer/trees/' + encodeURIComponent(treeId) + '/progress', {
      method: 'POST',
      headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
      body: JSON.stringify({ moves }),
    });
  } catch {}
}

function fenWithClocks(fen4) {
  const parts = fen4.split(' ');
  return parts.length >= 6 ? fen4 : [...parts, '0', '1'].join(' ');
}

async function loadTrees() {
  const token = localStorage.getItem('alpine_token');
  if (!token) return [];
  try {
    const r = await fetch(API_URL + '/api/trainer/trees', { headers: { 'Authorization': 'Bearer ' + token } });
    if (!r.ok) return [];
    return await r.json();
  } catch { return []; }
}

function treeCard(t) {
  const boardHtml = t.start_fen ? fenToMiniBoard(t.start_fen, t.color === 'black') :
    '<div class="aspect-square bg-slate-900/50 rounded"></div>';
  const id = (t.id || '').replace(/'/g, "\\'");
  return `
  <div class="card p-0 cursor-pointer transition-all hover:border-emerald-400/40 group" style="border-radius:10px" onclick="openTreeViewer('${id}')">
    ${boardHtml}
    <div class="p-3">
      <div class="flex items-center gap-1.5 mb-0.5">
        <span class="text-body text-white font-medium group-hover:text-white transition-colors">${t.name}</span>
      </div>
      <span class="inline-block text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400 mb-1.5">Opening Tree</span>
      <div class="flex items-center justify-between text-meta text-muted">
        <span>${t.color}</span>
        <span class="font-mono">${(t.lines_count || 0).toLocaleString()} lines</span>
      </div>
    </div>
  </div>`;
}

async function openTreeViewer(id) {
  const token = localStorage.getItem('alpine_token');
  if (!token) return;

  document.getElementById('trainerSelectView').style.display = 'none';
  document.getElementById('trainerOpeningView').style.display = 'none';
  document.getElementById('treeViewerView').style.display = 'block';
  document.getElementById('treeViewerStatus').textContent = 'Loading…';

  try {
    const r = await fetch(API_URL + '/api/trainer/trees/' + encodeURIComponent(id), { headers: { 'Authorization': 'Bearer ' + token } });
    if (!r.ok) throw new Error('HTTP ' + r.status);
    _treeData = await r.json();
    _treePath = [_treeData.tree];
    _treeFlipped = (_treeData.color === 'black');
    const catalogName = (typeof _catalogCurrentOpening !== 'undefined' && _catalogCurrentOpening) ? _catalogCurrentOpening.opening_name : null;
    document.getElementById('treeViewerTitle').textContent = catalogName || _treeData.name;
    document.getElementById('treeViewerStatus').textContent = '';

    // Fetch learned set
    const learned = await fetchTreeProgress(_treeData.id);
    _treeLearnedSet = new Set(learned.map(([fen, san]) => learnedKey(fen, san)));

    switchTreeTab('tree');
    if (typeof injectCatalogTabs === 'function') injectCatalogTabs();
  } catch (e) {
    document.getElementById('treeViewerStatus').textContent = 'Failed to load: ' + e.message;
  }
}

function switchTreeTab(tab) {
  // Stop any in-progress trainer activity
  if (_ttPendingTimer) { clearTimeout(_ttPendingTimer); _ttPendingTimer = null; }
  if (_tmPendingTimer) { clearTimeout(_tmPendingTimer); _tmPendingTimer = null; }
  // Clean up drills when switching away
  if ((_treeTab === 'punish' || _treeTab === 'moves') && tab !== 'punish' && tab !== 'moves') {
    if (typeof cleanupTrainerDrill === 'function') cleanupTrainerDrill();
  }
  if (_treeTab === 'maia' && tab !== 'maia') {
    if (typeof cleanupMaiaDrill === 'function') cleanupMaiaDrill();
  }

  _treeTab = tab;
  document.querySelectorAll('.tree-tab').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === tab);
  });
  const panelForTab = (tab === 'punish' || tab === 'moves') ? 'drill' : tab;
  ['tree','learn','train','drill','maia'].forEach(t => {
    const el = document.getElementById('treeTabContent-' + t);
    if (el) el.style.display = (t === panelForTab) ? '' : 'none';
  });

  // Toggle board wraps
  const isTreeBoard = (tab === 'tree' || tab === 'learn' || tab === 'train');
  document.getElementById('treeBoardWrap').style.display = isTreeBoard ? '' : 'none';
  document.getElementById('trainerBoardWrap').style.display = (tab === 'punish' || tab === 'moves') ? '' : 'none';
  document.getElementById('maiaBoardWrap').style.display = (tab === 'maia') ? '' : 'none';

  if (tab === 'tree') {
    _treePath = [_treeData.tree];
    renderTreeViewer();
  } else if (tab === 'learn') {
    enterLearnMode();
  } else if (tab === 'train') {
    enterTrainMode();
  } else if (tab === 'punish') {
    openTrainerOpening(_catalogCurrentOpening.opening_name);
  } else if (tab === 'moves') {
    openHardMoveOpening(_catalogCurrentOpening.opening_name);
  } else if (tab === 'maia') {
    const mp = _catalogCurrentOpening.maia_positions;
    if (mp && mp.length > 0) openMaiaPosition(mp[0].id);
  }
}

function exitTreeViewer() {
  if (_ttPendingTimer) { clearTimeout(_ttPendingTimer); _ttPendingTimer = null; }
  if (typeof _tmPendingTimer !== 'undefined' && _tmPendingTimer) { clearTimeout(_tmPendingTimer); _tmPendingTimer = null; }
  // Clean up any active drills
  if (typeof cleanupTrainerDrill === 'function') cleanupTrainerDrill();
  if (typeof cleanupMaiaDrill === 'function') cleanupMaiaDrill();
  document.getElementById('treeViewerView').style.display = 'none';
  document.querySelectorAll('.catalog-extra-tab').forEach(el => el.remove());
  document.getElementById('trainerSelectView').style.display = '';
  // Reset board wrap visibility
  document.getElementById('treeBoardWrap').style.display = '';
  document.getElementById('trainerBoardWrap').style.display = 'none';
  document.getElementById('maiaBoardWrap').style.display = 'none';
  if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  _treeData = null;
  _treePath = [];
  _ttMode = 'view';
  _ttGame = null;
  _ttCurNode = null;
  _tmGame = null;
  _tmActive = false;
  _treeLearnedSet = new Set();
  _treeTab = 'tree';
}

function countTreeNodes(node) {
  return 1 + (node.children || []).reduce((s, c) => s + countTreeNodes(c), 0);
}

function renderTreeViewer() {
  const node = _treePath[_treePath.length - 1];
  if (!node) return;

  // Board
  const fenFull = fenWithClocks(node.fen);
  const orientation = _treeFlipped ? 'black' : 'white';
  const boardEl = document.getElementById('treeBoard');
  if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  boardEl.innerHTML = '';
  const lastMove = node.move_uci ? [node.move_uci.slice(0, 2), node.move_uci.slice(2, 4)] : null;
  _treeCgInstance = Chessground(boardEl, {
    fen: fenFull,
    orientation,
    coordinates: true,
    viewOnly: true,
    lastMove,
    animation: { duration: 200 },
  });
  setTimeout(resizeBoards, 50);

  // Move line above continuations: start_moves (static) + tree path (clickable),
  // numbered from move 1 in PGN style.
  const startMovesArr = _treeData.start_moves ? _treeData.start_moves.split(/\s+/).filter(Boolean) : [];
  const moveLineEl = document.getElementById('treeMoveLine');
  if (moveLineEl) {
    const parts = [];
    let ply = 0;
    for (const san of startMovesArr) {
      ply++;
      if (ply % 2 === 1) parts.push(`<span class="text-muted">${Math.ceil(ply/2)}.</span>`);
      parts.push(`<span class="text-secondary">${san}</span>`);
    }
    _treePath.slice(1).forEach((n, i) => {
      ply++;
      const idx = i + 1;
      const isLast = idx === _treePath.length - 1;
      if (ply % 2 === 1) parts.push(`<span class="text-muted">${Math.ceil(ply/2)}.</span>`);
      if (isLast) {
        parts.push(`<span class="text-emerald-300 font-semibold">${n.move_san}</span>`);
      } else {
        parts.push(`<span class="cursor-pointer text-sky-400 hover:underline" onclick="goToTreeDepth(${idx})">${n.move_san}</span>`);
      }
    });
    moveLineEl.innerHTML = parts.join(' ');
  }

  // Continuations
  const inner = document.getElementById('treeContinuations');
  if (!node.children || node.children.length === 0) {
    inner.innerHTML = '<div class="text-meta text-muted text-center py-4">leaf — no continuations</div>';
  } else {
    const sorted = [...node.children].sort((a, b) => countTreeNodes(b) - countTreeNodes(a));
    inner.innerHTML = sorted.map((c, i) => {
      const lineSize = countTreeNodes(c);
      const reasonColor = {
        book: 'bg-emerald-500/15 text-emerald-300',
        engine: 'bg-amber-500/15 text-amber-300',
        opp:   'bg-red-500/15 text-red-300',
        top:   'bg-sky-500/15 text-sky-300',
        raw:   'bg-sky-500/15 text-sky-300',
      }[c.reason] || 'bg-slate-700 text-slate-300';
      const evalText = (c.eval_cp != null) ?
        `<span class="text-amber-300">${c.eval_cp >= 0 ? '+' : ''}${(c.eval_cp/100).toFixed(2)}</span>` : '';
      return `
      <div class="px-3 py-2 rounded cursor-pointer hover:bg-slate-800/60 flex items-center gap-3" onclick="descendTree(${i})">
        <span class="text-white font-bold w-14">${c.move_san}</span>
        <span class="text-meta text-muted flex-1">${lineSize.toLocaleString()} ${lineSize === 1 ? 'move' : 'moves'} ${evalText ? '· ' + evalText : ''}</span>
      </div>`;
    }).join('');
  }

  document.getElementById('treeViewerStatus').textContent = '';
}

function soundForTreeNode(node) {
  if (!node || !node.move_san) return null;
  const san = node.move_san;
  if (san.includes('+') || san.includes('#')) return 'move-check';
  if (san.includes('=')) return 'promote';
  if (san.includes('x')) return 'capture';
  if (san.startsWith('O-O')) return 'castle';
  return 'move-self';
}

function descendTree(i) {
  const cur = _treePath[_treePath.length - 1];
  const child = (cur.children || [])[i];
  if (!child) return;
  // sort matching renderTreeViewer
  const sorted = [...cur.children].sort((a, b) => countTreeNodes(b) - countTreeNodes(a));
  _treePath.push(sorted[i]);
  playSound(soundForTreeNode(sorted[i]));
  renderTreeViewer();
}

function goToTreeDepth(d) {
  _treePath = _treePath.slice(0, d + 1);
  playSound(soundForTreeNode(_treePath[_treePath.length - 1]));
  renderTreeViewer();
}

function treeFlipBoard() {
  _treeFlipped = !_treeFlipped;
  if (_ttMode === 'train') {
    // Re-render board only
    if (_ttGame && _ttGame.turn() === (_treeData.color === 'white' ? 'w' : 'b')) {
      setTrainerBoardInteractive();
    } else {
      setTrainerBoardViewOnly(_ttNode && _ttNode.move_uci);
    }
  } else {
    renderTreeViewer();
  }
}

// ══════════════════════════════════════════
// TRAINER MODE — Chessable-style "learn then test"
// with mainline-first + branch-up-from-deepest-diverge variation order
// ══════════════════════════════════════════
let _ttMode = 'view';            // 'view' | 'train'
let _ttPhase = 'idle';           // 'prefix' | 'learn' | 'test' | 'done' | 'all-done'
let _ttGame = null;              // chess.js instance
let _ttCurNode = null;           // tracks fen for board display only
let _ttLineNodes = [];           // path of TreeNodes from root to leaf for current variation
let _ttLineMoves = [];           // [{san, uci, is_user_turn, fen_after}] aligned with _ttLineNodes[1..]
let _ttChunkStart = 0;           // moves before this index are PREFIX (auto-play, no test)
let _ttChunkEnd = 0;             // moves before this index are part of the current chunk
let _ttLineIdx = 0;              // current playback index
let _ttStats = { correct: 0, wrong: 0 };
let _ttLineNum = 1;
let _ttExplored = new Map();     // Map<opp_fen, Set<chosen_san>>
let _ttPendingTimer = null;

const CHUNK_USER_MOVES = 2;      // user decisions per chunk
const LEARN_DELAY_MS = 750;
const PREFIX_DELAY_MS = 180;
const OPP_DELAY_MS = 500;
const CORRECT_DELAY_MS = 350;
const WRONG_DELAY_MS = 1500;

function enterLearnMode() {
  if (!_treeData) return;
  _ttMode = 'learn';
  _ttStats = { correct: 0, wrong: 0 };
  _ttLineNum = 1;
  _ttExplored = new Map();
  _ttLineNodes = [];
  document.getElementById('btnLearnNext').style.display = 'none';
  trainerStartLine();
}

function exitTrainerMode() {
  switchTreeTab('tree');
}

function trainerNextLine() {
  _ttLineNum++;
  trainerStartLine();
}

function trainerContinueLine() {
  // Find next unlearned user move at or after current chunkEnd; if none in this
  // line, jump to next variation.
  let nextStart = -1;
  for (let i = _ttChunkEnd; i < _ttLineMoves.length; i++) {
    const parentNode = _ttLineNodes[i];
    if (parentNode && parentNode.is_user_turn) {
      const k = learnedKey(parentNode.fen, _ttLineMoves[i].san);
      if (!_treeLearnedSet.has(k)) { nextStart = i; break; }
    }
  }
  if (nextStart === -1) {
    return trainerNextLine();
  }
  _ttChunkStart = nextStart;
  _ttChunkEnd = findChunkEnd(_ttChunkStart, CHUNK_USER_MOVES);
  startCurrentChunk();
}

function findChunkEnd(startIdx, n) {
  let userCount = 0;
  let i = startIdx;
  while (i < _ttLineMoves.length) {
    if (_ttLineMoves[i].is_user_turn) {
      userCount++;
      if (userCount === n) return i + 1;
    }
    i++;
  }
  return _ttLineMoves.length;
}

function markExplored(fen, san) {
  if (!_ttExplored.has(fen)) _ttExplored.set(fen, new Set());
  _ttExplored.get(fen).add(san);
}

// Pick a random opp child weighted by games count (used by Train mode).
function pickRandomOppChild(node) {
  const children = node.children || [];
  if (children.length === 0) return null;
  const totalWeight = children.reduce((s, c) => s + Math.max(c.games || 1, 1), 0);
  let r = Math.random() * totalWeight;
  for (const c of children) {
    r -= Math.max(c.games || 1, 1);
    if (r <= 0) return c;
  }
  return children[children.length - 1];
}

// Train-mode opp picker: prefer opp children whose user response IS learned, so
// lines stay in learned territory and don't end prematurely. Returns null if no
// such child exists (line ends).
function pickLearnedOppChild(node) {
  const children = node.children || [];
  const learnedChildren = children.filter(c => {
    if (!c.children || c.children.length === 0) return false;
    return _treeLearnedSet.has(learnedKey(c.fen, c.children[0].move_san));
  });
  if (learnedChildren.length === 0) return null;
  const total = learnedChildren.reduce((s, c) => s + Math.max(c.games || 1, 1), 0);
  let r = Math.random() * total;
  for (const c of learnedChildren) {
    r -= Math.max(c.games || 1, 1);
    if (r <= 0) return c;
  }
  return learnedChildren[learnedChildren.length - 1];
}

// Pick OPP child with the largest subtree (= deepest theory below it), ties
// broken by games. This makes the first line walk the deepest available branch
// rather than dying early just because the highest-popularity child happens to
// have a shallow subtree.
function pickUnexploredOppChild(node) {
  const explored = _ttExplored.get(node.fen) || new Set();
  const unseen = (node.children || []).filter(c => !explored.has(c.move_san));
  if (unseen.length === 0) return null;
  unseen.sort((a, b) => countTreeNodes(b) - countTreeNodes(a) || (b.games || 0) - (a.games || 0));
  return unseen[0];
}

// Walk down from `startNode`, picking unexplored opp children (or just descend if
// all explored — that's an internal mainline). Returns array of TreeNode (path
// from startNode INCLUSIVE down to a leaf). Trims trailing opp moves so the
// path always ends on a user move (lines we can't respond to are useless to teach).
function walkDownPath(startNode) {
  const path = [startNode];
  let node = startNode;
  while (node && node.children && node.children.length > 0) {
    let child;
    if (node.is_user_turn) {
      child = node.children[0];
    } else {
      child = pickUnexploredOppChild(node);
      if (!child) {
        const sorted = [...node.children].sort((a, b) => countTreeNodes(b) - countTreeNodes(a) || (b.games || 0) - (a.games || 0));
        child = sorted[0];
      }
      if (child) markExplored(node.fen, child.move_san);
    }
    if (!child) break;
    path.push(child);
    node = child;
  }
  // Trim trailing opp moves: while the last move was made by opp (= source node
  // was an opp-decision node), drop it.
  while (path.length >= 2 && !path[path.length - 2].is_user_turn) {
    path.pop();
  }
  return path;
}

// Build the next line. Returns { path, divergeIdx } or null if all variations done.
// Skips candidate variations that would give the user no move to play (i.e. the
// diverging opp move leads to a leaf with no user response in the tree).
function buildNextLine() {
  if (_ttLineNodes.length === 0) {
    const path = walkDownPath(_treeData.tree);
    return { path, divergeIdx: 0 };
  }

  for (let i = _ttLineNodes.length - 1; i >= 0; i--) {
    const node = _ttLineNodes[i];
    if (node.is_user_turn) continue;
    if (!node.children || node.children.length === 0) continue;
    // Try unexplored siblings until we find one that gives at least one user move.
    while (true) {
      const candidate = pickUnexploredOppChild(node);
      if (!candidate) break;
      markExplored(node.fen, candidate.move_san);
      const tail = walkDownPath(candidate);
      if (tail.length >= 2) {
        // At least one move after the diverging opp move (= a user move follows)
        const path = _ttLineNodes.slice(0, i + 1).concat(tail);
        return { path, divergeIdx: i };
      }
      // Useless variation — only the opp move, no user response. Skip.
    }
  }
  return null;
}

function pathToMoves(nodePath) {
  // nodePath[0] is root (no incoming move). Generate moves[1..] from edges.
  const moves = [];
  for (let i = 1; i < nodePath.length; i++) {
    const parent = nodePath[i - 1];
    const child = nodePath[i];
    moves.push({
      san: child.move_san,
      uci: child.move_uci,
      is_user_turn: parent.is_user_turn,
      fen_after: child.fen,
    });
  }
  return moves;
}

function findFirstUnlearnedMoveIdx(lineMoves, lineNodes) {
  for (let i = 0; i < lineMoves.length; i++) {
    const parentNode = lineNodes[i];
    if (parentNode && parentNode.is_user_turn) {
      const k = learnedKey(parentNode.fen, lineMoves[i].san);
      if (!_treeLearnedSet.has(k)) return i;
    }
  }
  return -1;
}

function trainerStartLine() {
  if (_ttPendingTimer) { clearTimeout(_ttPendingTimer); _ttPendingTimer = null; }

  // Loop variations until we find one with unlearned content
  let built = null;
  let firstUnlearned = -1;
  let moves = [];
  while (true) {
    built = buildNextLine();
    if (!built) break;
    moves = pathToMoves(built.path);
    firstUnlearned = findFirstUnlearnedMoveIdx(moves, built.path);
    if (firstUnlearned !== -1) break;
    // else: this variation is fully learned, try next
  }

  if (!built) {
    _ttPhase = 'all-done';
    setTreeTrainStatus(`<span class="text-emerald-300 font-semibold">Everything learned!</span> Session: ${_ttStats.correct} ✓ · ${_ttStats.wrong} ✗ · Switch to Train tab to drill.`);
    document.getElementById('btnLearnNext').style.display = 'none';
    if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
    return;
  }

  _ttLineNodes = built.path;
  _ttLineMoves = moves;
  _ttChunkStart = firstUnlearned;
  _ttChunkEnd = findChunkEnd(_ttChunkStart, CHUNK_USER_MOVES);
  document.getElementById('btnLearnNext').style.display = 'none';
  updateTreeTrainStats();

  if (_ttLineMoves.length === 0) {
    return lineComplete();
  }

  startCurrentChunk();
}

function startCurrentChunk() {
  if (_ttPendingTimer) { clearTimeout(_ttPendingTimer); _ttPendingTimer = null; }
  document.getElementById('btnLearnNext').style.display = 'none';

  // Fast-forward game state to chunk start (instant, no animation)
  _ttGame = new Chess(fenWithClocks(_treeData.tree.fen));
  _ttCurNode = _treeData.tree;
  for (let i = 0; i < _ttChunkStart; i++) {
    const mv = _ttLineMoves[i];
    try { _ttGame.move(mv.san); } catch { return chunkComplete(); }
    _ttCurNode = { fen: mv.fen_after };
  }
  _ttLineIdx = _ttChunkStart;
  setTrainerBoardViewOnly(_ttChunkStart > 0 ? _ttLineMoves[_ttChunkStart - 1].uci : null);

  _ttPhase = 'learn';
  const newMoves = _ttChunkEnd - _ttChunkStart;
  setTreeTrainStatus(`<span class="text-amber-300">Learn:</span> ${newMoves} new move${newMoves === 1 ? '' : 's'}`);
  _ttPendingTimer = setTimeout(playLearnStep, LEARN_DELAY_MS);
}

function playPrefixStep() {
  if (_ttPhase !== 'prefix') return;
  if (_ttLineIdx >= _ttChunkStart) {
    _ttPhase = 'learn';
    const newMoves = _ttChunkEnd - _ttChunkStart;
    setTreeTrainStatus(`<span class="text-amber-300">Learn:</span> ${newMoves} new move${newMoves === 1 ? '' : 's'}`);
    _ttPendingTimer = setTimeout(playLearnStep, LEARN_DELAY_MS);
    return;
  }
  const mv = _ttLineMoves[_ttLineIdx];
  try { _ttGame.move(mv.san); } catch { return startTestPhase(); }
  _ttCurNode = { fen: mv.fen_after };
  setTrainerBoardViewOnly(mv.uci);
  _ttLineIdx++;
  _ttPendingTimer = setTimeout(playPrefixStep, PREFIX_DELAY_MS);
}

function playLearnStep() {
  if (_ttPhase !== 'learn') return;
  if (_ttLineIdx >= _ttChunkEnd) {
    return startTestPhase();
  }
  const mv = _ttLineMoves[_ttLineIdx];
  let moveResult;
  try { moveResult = _ttGame.move(mv.san); } catch { return startTestPhase(); }
  playSound(soundForMove(_ttGame, moveResult));
  _ttCurNode = { fen: mv.fen_after };
  setTrainerBoardViewOnly(mv.uci);
  const who = mv.is_user_turn ? '<span class="text-emerald-300">you</span>' : '<span class="text-red-300">opp</span>';
  const inChunk = _ttLineIdx - _ttChunkStart + 1;
  const chunkSize = _ttChunkEnd - _ttChunkStart;
  setTreeTrainStatus(`<span class="text-amber-300">Learn:</span> ${who} <b>${mv.san}</b> · ${inChunk}/${chunkSize}`);
  _ttLineIdx++;
  _ttPendingTimer = setTimeout(playLearnStep, LEARN_DELAY_MS);
}

function startTestPhase() {
  _ttPhase = 'test';
  _ttChunkWrongs = 0;
  _ttLineIdx = 0;
  _ttGame = new Chess(fenWithClocks(_treeData.tree.fen));
  _ttCurNode = _treeData.tree;

  // Auto-play everything before the chunk (already-known content) silently
  while (_ttLineIdx < _ttChunkStart && _ttLineIdx < _ttLineMoves.length) {
    const mv = _ttLineMoves[_ttLineIdx];
    try { _ttGame.move(mv.san); } catch { return lineComplete(); }
    _ttCurNode = { fen: mv.fen_after };
    _ttLineIdx++;
  }
  setTrainerBoardViewOnly(_ttLineIdx > 0 ? _ttLineMoves[_ttLineIdx - 1].uci : null);
  setTreeTrainStatus('<span class="text-sky-300 font-semibold">Now you try.</span>');
  _ttPendingTimer = setTimeout(testAdvance, 700);
}

function testAdvance() {
  if (_ttPhase !== 'test') return;
  // Auto-play opp moves; stop when it's user's turn OR we hit the chunk end
  let lastOppMove = null;
  while (_ttLineIdx < _ttChunkEnd && !_ttLineMoves[_ttLineIdx].is_user_turn) {
    const mv = _ttLineMoves[_ttLineIdx];
    try { lastOppMove = _ttGame.move(mv.san); } catch { return chunkComplete(); }
    _ttCurNode = { fen: mv.fen_after };
    _ttLineIdx++;
  }
  if (lastOppMove) playSound(soundForMove(_ttGame, lastOppMove));
  if (_ttLineIdx >= _ttChunkEnd) {
    return chunkComplete();
  }
  // Show last opp move briefly, then enable interactive
  const lastMoveUci = _ttLineIdx > 0 ? _ttLineMoves[_ttLineIdx - 1].uci : null;
  setTrainerBoardViewOnly(lastMoveUci);
  _ttPendingTimer = setTimeout(() => {
    setTrainerBoardInteractive();
    const inChunk = _ttLineIdx - _ttChunkStart + 1;
    const chunkSize = _ttChunkEnd - _ttChunkStart;
    setTreeTrainStatus(`<span class="text-sky-300">Your move</span> · ${inChunk}/${chunkSize}`);
  }, OPP_DELAY_MS);
}

function setTrainerBoardInteractive() {
  if (!_ttGame || !_ttCurNode) return;
  if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  const boardEl = document.getElementById('treeBoard');
  boardEl.innerHTML = '';
  const fen = fenWithClocks(_ttCurNode.fen);
  const orientation = _treeFlipped ? 'black' : 'white';
  const turnColor = _ttGame.turn() === 'w' ? 'white' : 'black';
  const dests = new Map();
  for (const m of _ttGame.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  _treeCgInstance = Chessground(boardEl, {
    fen, orientation, turnColor,
    coordinates: true,
    viewOnly: false,
    animation: { duration: 200 },
    movable: {
      free: false, color: turnColor, dests, showDests: true,
      events: { after: (orig, dest) => onTrainerMove(orig, dest) },
    },
    draggable: { enabled: true },
  });
  setTimeout(resizeBoards, 50);
}

function setTrainerBoardViewOnly(lastMoveUci) {
  if (!_ttCurNode) return;
  if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  const boardEl = document.getElementById('treeBoard');
  boardEl.innerHTML = '';
  const fen = fenWithClocks(_ttCurNode.fen);
  const orientation = _treeFlipped ? 'black' : 'white';
  const lastMove = lastMoveUci ? [lastMoveUci.slice(0, 2), lastMoveUci.slice(2, 4)] : null;
  _treeCgInstance = Chessground(boardEl, {
    fen, orientation,
    coordinates: true, viewOnly: true,
    lastMove,
    animation: { duration: 200 },
  });
  setTimeout(resizeBoards, 50);
}

function onTrainerMove(orig, dest) {
  if (_ttPhase !== 'test' || !_ttGame) return;
  let move;
  try { move = _ttGame.move({ from: orig, to: dest, promotion: 'q' }); } catch { return; }
  if (!move) return;
  const expected = _ttLineMoves[_ttLineIdx];

  if (expected && move.san === expected.san) {
    _ttStats.correct++;
    playSound(soundForMove(_ttGame, move));
    _ttCurNode = { fen: expected.fen_after };
    _ttLineIdx++;
    setTrainerBoardViewOnly(expected.uci);
    setTreeTrainStatus('<span class="text-emerald-300 font-semibold">✓ Correct</span>');
    updateTreeTrainStats();
    _ttPendingTimer = setTimeout(testAdvance, CORRECT_DELAY_MS);
  } else {
    playSound(soundForMove(_ttGame, move));
    _ttStats.wrong++;
    _ttChunkWrongs++;
    const correctSan = expected ? expected.san : '???';
    _ttGame.undo();
    setTrainerBoardViewOnly(null);
    setTreeTrainStatus(`<span class="text-red-400 font-semibold">✗ Wrong</span> — correct: <span class="text-emerald-300 font-bold">${correctSan}</span>`);
    updateTreeTrainStats();
    _ttPendingTimer = setTimeout(() => {
      if (expected) {
        let corrMove;
        try { corrMove = _ttGame.move(expected.san); } catch { return chunkComplete(); }
        playSound(soundForMove(_ttGame, corrMove));
        _ttCurNode = { fen: expected.fen_after };
        _ttLineIdx++;
        setTrainerBoardViewOnly(expected.uci);
        _ttPendingTimer = setTimeout(testAdvance, CORRECT_DELAY_MS);
      } else chunkComplete();
    }, WRONG_DELAY_MS);
  }
}

function chunkComplete() {
  _ttPhase = 'done';
  setTrainerBoardViewOnly(_ttLineIdx > 0 ? _ttLineMoves[_ttLineIdx - 1].uci : null);

  // If no wrongs in this chunk, mark all of its user moves as learned (server + local set)
  if (_ttChunkWrongs === 0) {
    const newlyLearned = [];
    for (let i = _ttChunkStart; i < _ttChunkEnd; i++) {
      const mv = _ttLineMoves[i];
      if (!mv) continue;
      const parentNode = _ttLineNodes[i];
      if (!parentNode || !parentNode.is_user_turn) continue;
      const k = learnedKey(parentNode.fen, mv.san);
      if (!_treeLearnedSet.has(k)) {
        _treeLearnedSet.add(k);
        newlyLearned.push([parentNode.fen, mv.san]);
      }
    }
    if (newlyLearned.length && _treeData) {
      postTreeProgress(_treeData.id, newlyLearned);
    }
  }

  const moreInLine = _ttChunkEnd < _ttLineMoves.length;
  if (moreInLine) {
    const ok = _ttChunkWrongs === 0 ? '<span class="text-emerald-300">✓ all correct</span>' : `<span class="text-amber-300">${_ttChunkWrongs} wrong</span>`;
    setTreeTrainStatus(`<span class="text-sky-300 font-semibold">Chunk done</span> — ${ok} · more in this line`);
    setNextButtonLabel('Continue line →', trainerContinueLine);
  } else {
    const ok = _ttChunkWrongs === 0 ? '<span class="text-emerald-300">✓ all correct</span>' : `<span class="text-amber-300">${_ttChunkWrongs} wrong</span>`;
    setTreeTrainStatus(`<span class="text-sky-300 font-semibold">Line complete</span> — ${ok}`);
    setNextButtonLabel('Next variation →', trainerNextLine);
  }
}

function lineComplete() { chunkComplete(); }

function setNextButtonLabel(label, handler) {
  const btn = document.getElementById('btnLearnNext');
  if (!btn) return;
  btn.textContent = label;
  btn.onclick = handler;
  btn.style.display = '';
}

// Track if any wrong move occurred during the current chunk's test phase.
// If 0 wrongs, all the chunk's user moves are sent as "learned" on completion.
let _ttChunkWrongs = 0;

function setTreeTrainStatus(msg) {
  const el = document.getElementById('treeTrainStatus');
  if (el) el.innerHTML = msg;
}

function updateTreeTrainStats() {
  const el = document.getElementById('treeTrainStats');
  if (el) el.textContent = `${_ttStats.correct} ✓ · ${_ttStats.wrong} ✗ · line ${_ttLineNum}`;
}

// ══════════════════════════════════════════
// TRAIN MODE — recall practice on learned moves only.
// Walks the tree picking only learned user moves at user nodes (random opp at
// opp nodes). Wrong move → show correct → reset to that position → user retries
// until correct → continue.
// ══════════════════════════════════════════
let _tmGame = null;
let _tmLineMoves = [];        // [{san, uci, is_user_turn, fen_after}]
let _tmLineIdx = 0;
let _tmStats = { correct: 0, wrong: 0 };
let _tmPendingTimer = null;
let _tmActive = false;
let _tmCurNode = null;
let _tmExpectedSan = null;    // currently-expected user move (for retry)
let _tmLinesThisSession = 0;  // first line always starts from root; later lines random

function countFullyLearnedLines() {
  if (!_treeData) return 0;
  let count = 0;
  function walk(node, allLearned) {
    let stillAll = allLearned;
    if (node.is_user_turn && node.children && node.children.length > 0) {
      const child = node.children[0];
      if (!_treeLearnedSet.has(learnedKey(node.fen, child.move_san))) {
        stillAll = false;
      }
    }
    if (!node.children || node.children.length === 0) {
      if (stillAll) count++;
      return;
    }
    for (const c of node.children) walk(c, stillAll);
  }
  walk(_treeData.tree, true);
  return count;
}

function enterTrainMode() {
  _tmStats = { correct: 0, wrong: 0 };
  _tmActive = false;
  document.getElementById('btnTrainNextLine').style.display = 'none';
  document.getElementById('btnTrainStart').style.display = '';
  const totalLines = _treeData.lines_count || 0;
  const learnedLines = countFullyLearnedLines();
  if (_treeLearnedSet.size === 0) {
    setTrainModeStatus('<span class="text-amber-300">No learned moves yet.</span> Use the Learn tab to learn lines, then come back here to drill them.');
    if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  } else {
    setTrainModeStatus(`<span class="text-emerald-300">${learnedLines}</span> / ${totalLines} lines learned. Press Start.`);
  }
  updateTrainModeStats();
}

function trainModeExit() { switchTreeTab('tree'); }

function trainModeStart() {
  if (_treeLearnedSet.size === 0) return;
  _tmActive = true;
  _tmLinesThisSession = 0;
  document.getElementById('btnTrainStart').style.display = 'none';
  document.getElementById('btnTrainNextLine').style.display = 'none';
  trainModeStartLine();
}

function trainModeNext() {
  document.getElementById('btnTrainNextLine').style.display = 'none';
  trainModeStartLine();
}

function collectLearnedStartNodes() {
  // Walk tree, find user-decision nodes whose recommended move is learned.
  const out = [];
  function walk(node, depth) {
    if (node.is_user_turn && node.children && node.children.length > 0) {
      const child = node.children[0];
      if (_treeLearnedSet.has(learnedKey(node.fen, child.move_san))) {
        out.push({ node, depth });
      }
    }
    for (const c of (node.children || [])) walk(c, depth + 1);
  }
  walk(_treeData.tree, 0);
  return out;
}

// Gaussian-weighted pick centered on median depth — biases toward mid-ply
// positions so root/shallow theory isn't repeated to death.
function pickWeightedNode(items) {
  if (items.length === 0) return null;
  if (items.length === 1) return items[0];
  const depths = items.map(i => i.depth).sort((a, b) => a - b);
  const minD = depths[0];
  const maxD = depths[depths.length - 1];
  const midD = depths[Math.floor(depths.length / 2)];
  const sigma = Math.max((maxD - minD) / 3, 1);
  const weights = items.map(i => Math.exp(-Math.pow(i.depth - midD, 2) / (2 * sigma * sigma)));
  const total = weights.reduce((a, b) => a + b, 0);
  let r = Math.random() * total;
  for (let i = 0; i < items.length; i++) {
    r -= weights[i];
    if (r <= 0) return items[i];
  }
  return items[items.length - 1];
}

function trainModeStartLine() {
  if (_tmPendingTimer) { clearTimeout(_tmPendingTimer); _tmPendingTimer = null; }

  const candidates = collectLearnedStartNodes();
  if (candidates.length === 0) {
    setTrainModeStatus('<span class="text-amber-300">No learned moves yet.</span> Use the Learn tab first.');
    document.getElementById('btnTrainNextLine').style.display = '';
    document.getElementById('btnTrainStart').style.display = '';
    _tmActive = false;
    return;
  }

  // First line of a session always starts at the root; subsequent lines pick
  // a weighted-random learned position to reduce shallow-move repetition.
  let startNode, startDepth;
  if (_tmLinesThisSession === 0) {
    startNode = _treeData.tree;
    startDepth = 0;
  } else {
    const picked = pickWeightedNode(candidates);
    startNode = picked.node;
    startDepth = picked.depth;
  }
  _tmLinesThisSession++;

  // Walk down from startNode using learned moves; at opp nodes prefer children
  // whose user response is learned, so the line stays in learned territory.
  const path = [startNode];
  let node = startNode;
  while (node && node.children && node.children.length > 0) {
    let child;
    if (node.is_user_turn) {
      child = node.children[0];
      if (!_treeLearnedSet.has(learnedKey(node.fen, child.move_san))) break;
    } else {
      child = pickLearnedOppChild(node);
      if (!child) break;
    }
    if (!child) break;
    path.push(child);
    node = child;
  }
  while (path.length >= 2 && !path[path.length - 2].is_user_turn) {
    path.pop();
  }

  if (path.length < 2) {
    setTrainModeStatus('<span class="text-amber-300">Picked position has no testable line.</span>');
    document.getElementById('btnTrainNextLine').style.display = '';
    return;
  }

  _tmLineMoves = pathToMoves(path);
  _tmLineIdx = 0;
  _tmGame = new Chess(fenWithClocks(startNode.fen));
  _tmCurNode = startNode;
  _ttCurNode = startNode;
  setTrainerBoardViewOnly(null);
  const moveNum = Math.ceil((startDepth + 1 + (_treeData.start_ply || 0)) / 2);
  setTrainModeStatus(`<span class="text-sky-300">From move ${moveNum}</span> · ${path.length - 1} moves to play`);
  trainModeAdvance();
}

function trainModeAdvance() {
  if (_tmPendingTimer) { clearTimeout(_tmPendingTimer); _tmPendingTimer = null; }
  // Auto-play opp moves until user turn or end of line
  let lastOppMove = null;
  while (_tmLineIdx < _tmLineMoves.length && !_tmLineMoves[_tmLineIdx].is_user_turn) {
    const mv = _tmLineMoves[_tmLineIdx];
    try { lastOppMove = _tmGame.move(mv.san); } catch { return trainModeLineComplete(); }
    _tmCurNode = { fen: mv.fen_after };
    _ttCurNode = _tmCurNode;
    _tmLineIdx++;
  }
  if (lastOppMove) playSound(soundForMove(_tmGame, lastOppMove));
  if (_tmLineIdx >= _tmLineMoves.length) {
    return trainModeLineComplete();
  }
  // Show last opp move briefly, then enable interactive
  const lastMoveUci = _tmLineIdx > 0 ? _tmLineMoves[_tmLineIdx - 1].uci : null;
  setTrainerBoardViewOnly(lastMoveUci);
  _tmPendingTimer = setTimeout(() => {
    _tmExpectedSan = _tmLineMoves[_tmLineIdx].san;
    trainModeBoardInteractive();
    setTrainModeStatus('Your move');
  }, 350);
}

function trainModeBoardInteractive() {
  if (!_tmGame || !_tmCurNode) return;
  if (_treeCgInstance) { _treeCgInstance.destroy(); _treeCgInstance = null; }
  const boardEl = document.getElementById('treeBoard');
  boardEl.innerHTML = '';
  const fen = fenWithClocks(_tmCurNode.fen);
  const orientation = _treeFlipped ? 'black' : 'white';
  const turnColor = _tmGame.turn() === 'w' ? 'white' : 'black';
  const dests = new Map();
  for (const m of _tmGame.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  _treeCgInstance = Chessground(boardEl, {
    fen, orientation, turnColor,
    coordinates: true,
    viewOnly: false,
    animation: { duration: 200 },
    movable: {
      free: false, color: turnColor, dests, showDests: true,
      events: { after: (orig, dest) => onTrainModeMove(orig, dest) },
    },
    draggable: { enabled: true },
  });
  setTimeout(resizeBoards, 50);
}

function onTrainModeMove(orig, dest) {
  if (!_tmActive || !_tmGame) return;
  let move;
  try { move = _tmGame.move({ from: orig, to: dest, promotion: 'q' }); } catch { return; }
  if (!move) return;
  const expected = _tmLineMoves[_tmLineIdx];

  if (expected && move.san === expected.san) {
    _tmStats.correct++;
    playSound(soundForMove(_tmGame, move));
    _tmCurNode = { fen: expected.fen_after };
    _ttCurNode = _tmCurNode;
    _tmLineIdx++;
    setTrainerBoardViewOnly(expected.uci);
    setTrainModeStatus('<span class="text-emerald-300 font-semibold">✓</span>');
    updateTrainModeStats();
    _tmPendingTimer = setTimeout(trainModeAdvance, 250);
  } else {
    playSound(soundForMove(_tmGame, move));
    _tmStats.wrong++;
    const correctSan = expected ? expected.san : '???';
    _tmGame.undo();
    setTrainerBoardViewOnly(null);
    setTrainModeStatus(`<span class="text-red-400 font-semibold">✗ Wrong</span> — correct: <span class="text-emerald-300 font-bold">${correctSan}</span> — try again`);
    updateTrainModeStats();
    // After short delay, re-enable interactive at the same position so user retries
    _tmPendingTimer = setTimeout(() => {
      trainModeBoardInteractive();
    }, 1100);
  }
}

function trainModeLineComplete() {
  setTrainerBoardViewOnly(_tmLineIdx > 0 ? _tmLineMoves[_tmLineIdx - 1].uci : null);
  setTrainModeStatus(`<span class="text-sky-300 font-semibold">Line complete</span> · ${_tmStats.correct} ✓ · ${_tmStats.wrong} ✗`);
  document.getElementById('btnTrainNextLine').style.display = '';
}

function setTrainModeStatus(msg) {
  const el = document.getElementById('trainModeStatus');
  if (el) el.innerHTML = msg;
}

function updateTrainModeStats() {
  const el = document.getElementById('trainModeStats');
  if (el) el.textContent = `${_tmStats.correct} ✓ · ${_tmStats.wrong} ✗`;
}
