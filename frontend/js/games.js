// ══════════════════════════════════════════
// GAMES LIST PAGE
// ══════════════════════════════════════════
const GAMES_PER_PAGE = 10;
let _gamesPage = 1;
let _gamesTotal = 0;
let _gamesSelectedTags = new Set();
let _gamesHasMoreHistory = false;
let _gamesTotalUnanalyzed = 0;
let _gamesTotalAnalyzed = 0;
let _gamesAllTags = {};
let _gamesCache = {}; // id -> game object (with SAN moves from stored endpoint)

// Tag display names
const TAG_DISPLAY = {
  // Game-level tags
  queen_sacrifice:'Queen Sacrifice', rook_sacrifice:'Rook Sacrifice', smothered_mate:'Smothered Mate',
  king_mate:'King Mate', castling_mate:'Castling Mate', en_passant_mate:'En Passant Mate',
  titled:'Titled', GM:'GM', IM:'IM', FM:'FM', CM:'CM', NM:'NM',
  WGM:'WGM', WIM:'WIM', WFM:'WFM', WCM:'WCM', WNM:'WNM',
  'Chess.com':'Chess.com', 'Lichess':'Lichess',
  Win:'Win', Loss:'Loss', Draw:'Draw',
  // Tactic themes
  fork:'Fork', pin:'Pin', skewer:'Skewer', deflection:'Deflection',
  attraction:'Attraction', interference:'Interference', intermezzo:'Intermezzo',
  clearance:'Clearance', discoveredAttack:'Discovered Attack',
  discoveredCheck:'Discovered Check', doubleCheck:'Double Check',
  xRayAttack:'X-Ray Attack', windmill:'Windmill', sacrifice:'Sacrifice',
  capturingDefender:'Capturing Defender', hangingPiece:'Hanging Piece',
  trappedPiece:'Trapped Piece', overloading:'Overloading',
  exposedKing:'Exposed King', kingsideAttack:'Kingside Attack',
  queensideAttack:'Queenside Attack', attackingF2F7:'Attacking f2/f7',
  advancedPawn:'Advanced Pawn', promotion:'Promotion', underPromotion:'Under-Promotion',
  enPassant:'En Passant', castling:'Castling',
  defensiveMove:'Defensive Move', quietMove:'Quiet Move', zugzwang:'Zugzwang',
  // Mate patterns
  greekGift:'Greek Gift',
  backRankMate:'Back Rank Mate', smotheredMate:'Smothered Mate',
  anastasiaMate:'Anastasia Mate', arabianMate:'Arabian Mate',
  bodenMate:'Boden Mate', dovetailMate:'Dovetail Mate',
  doubleBishopMate:'Double Bishop Mate', balestraMate:'Balestra Mate',
  blindSwineMate:'Blind Swine Mate', cornerMate:'Corner Mate',
  hookMate:'Hook Mate', killBoxMate:'Kill Box Mate',
  morphysMate:"Morphy's Mate", operaMate:'Opera Mate',
  pillsburysMate:"Pillsbury's Mate", triangleMate:'Triangle Mate',
  vukovicMate:'Vukovic Mate', doubleCheckmate:'Double Checkmate',
  // Puzzle metadata
  mate:'Mate', mateIn1:'Mate in 1', mateIn2:'Mate in 2',
  mateIn3:'Mate in 3', mateIn4:'Mate in 4', mateIn5:'Mate in 5',
  oneMove:'One Move', short:'Short', long:'Long', veryLong:'Very Long',
  advantage:'Advantage', crushing:'Crushing', equality:'Equality',
  // Endgame types
  pawnEndgame:'Pawn Endgame', knightEndgame:'Knight Endgame',
  bishopEndgame:'Bishop Endgame', rookEndgame:'Rook Endgame',
  queenEndgame:'Queen Endgame', queenRookEndgame:'Queen + Rook Endgame',
  // FCE endgame segment types
  'Pawn Endings':'Pawn Endings', 'Knight Endings':'Knight Endings',
  'Bishop Endings':'Bishop Endings', 'Bishop vs Knight':'Bishop vs Knight',
  'Rook Endings':'Rook Endings', 'Rook vs Minor Piece':'Rook vs Minor Piece',
  'Rook + Minor vs Rook + Minor':'Rook+Minor vs Rook+Minor',
  'Rook + Minor vs Rook':'Rook+Minor vs Rook',
  'Queen Endings':'Queen Endings', 'Queen vs Rook':'Queen vs Rook',
  'Queen vs Minor Piece':'Queen vs Minor Piece',
  'Queen + Piece vs Queen':'Queen+Piece vs Queen',
};
// Game page: only show game-level tags, not puzzle tactic tags
const GAME_PAGE_TAGS = new Set([
  'Chess.com', 'Lichess',
  'Win', 'Loss', 'Draw',
  'queen_sacrifice', 'rook_sacrifice', 'smothered_mate',
  'king_mate', 'castling_mate', 'en_passant_mate',
  'titled', 'GM', 'IM', 'FM', 'CM', 'NM',
  'WGM', 'WIM', 'WFM', 'WCM', 'WNM',
]);
function tagDisplayName(tag) {
  if (TAG_DISPLAY[tag]) return TAG_DISPLAY[tag];
  // Convert camelCase or snake_case to readable words
  return tag
    .replace(/_/g, ' ')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, c => c.toUpperCase());
}
function isGameTag(tag) { return GAME_PAGE_TAGS.has(tag); }

const HIDDEN_PUZZLE_TAGS = new Set([
  'mate','crushing','advantage','equality',
  'oneMove','short','long','veryLong',
  'pawnEndgame','knightEndgame','bishopEndgame','rookEndgame','queenEndgame','queenRookEndgame',
  'queen_sacrifice','rook_sacrifice','smothered_mate',
  'king_mate','castling_mate','en_passant_mate',
  'Chess.com','Lichess','Win','Loss','Draw','titled',
  'GM','IM','FM','CM','NM','WGM','WIM','WFM','WCM','WNM',
]);
function isVisibleTag(tag) { return !HIDDEN_PUZZLE_TAGS.has(tag); }

// Time control classification
function getGameType(tc) {
  if (!tc) return { label:'Rapid', color:'text-green-400' };
  if (tc.includes('d') || tc.includes('day')) return { label:'Daily', color:'text-purple-400' };
  const m = tc.match(/^(\d+)/);
  if (!m) return { label:'Rapid', color:'text-green-400' };
  let base = parseInt(m[1]);
  if (base > 60) base = base / 60;
  if (base < 3) return { label:'Bullet', color:'text-yellow-400' };
  if (base < 10) return { label:'Blitz', color:'text-orange-400' };
  if (base < 30) return { label:'Rapid', color:'text-green-400' };
  return { label:'Classical', color:'text-blue-400' };
}

async function initGames() {
  window._gamesInit = true;
  _gamesPage = 1;
  _gamesSelectedTags = new Set();

  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  const headers = { 'Authorization': 'Bearer ' + token };

  // Fetch everything in parallel
  const [storedRes, tagsRes, unanalyzedRes, analyzedRes, backfillRes] = await Promise.all([
    fetch(API_URL + `/api/games/stored?limit=${GAMES_PER_PAGE}&offset=0`, { headers }).catch(() => null),
    fetch(API_URL + '/api/games/tags', { headers }).catch(() => null),
    fetch(API_URL + '/api/games/stored?limit=0&analyzed=false', { headers }).catch(() => null),
    fetch(API_URL + '/api/games/stored?limit=0&analyzed=true', { headers }).catch(() => null),
    fetch(API_URL + '/api/games/backfill/status', { headers }).catch(() => null),
  ]);

  // Parse responses
  const storedData = storedRes?.ok ? await storedRes.json() : { games: [], total: 0 };
  const tagsData = tagsRes?.ok ? await tagsRes.json() : { tags: {} };
  const unanalyzedData = unanalyzedRes?.ok ? await unanalyzedRes.json() : { total: 0 };
  const analyzedData = analyzedRes?.ok ? await analyzedRes.json() : { total: 0 };
  const backfillData = backfillRes?.ok ? await backfillRes.json() : { hasMoreHistory: false };

  _gamesTotal = storedData.total || 0;
  _gamesAllTags = tagsData.tags || {};
  _gamesTotalUnanalyzed = unanalyzedData.total || 0;
  _gamesTotalAnalyzed = analyzedData.total || 0;
  _gamesHasMoreHistory = backfillData.hasMoreHistory || false;

  // Update stats
  renderGamesStats();
  // Update action buttons
  renderGamesActions();
  // Render tags
  renderGamesTags();
  // Render game list
  renderGamesList(storedData.games || []);
  // Render pagination
  renderGamesPagination();
}

function renderGamesStats() {
  const el = document.getElementById('gamesStats');
  el.innerHTML = `<span class="text-white font-mono">${_gamesTotal}</span> games` +
    (_gamesTotalAnalyzed > 0 ? ` <span class="text-good font-mono">(${_gamesTotalAnalyzed} analyzed)</span>` : '');
}

function renderGamesActions() {
  const btnHistory = document.getElementById('btnLoadHistory');
  const btnAnalyze = document.getElementById('btnAnalyze');
  const btnAnalyzeText = document.getElementById('btnAnalyzeText');

  btnHistory.style.display = (_gamesTotal > 0 && _gamesHasMoreHistory) ? 'flex' : 'none';

  if (_gamesTotalUnanalyzed > 0) {
    btnAnalyze.style.display = 'flex';
    btnAnalyzeText.textContent = `Analyze (${_gamesTotalUnanalyzed})`;
  } else {
    btnAnalyze.style.display = 'none';
  }
}

function renderGamesTags() {
  const container = document.getElementById('gamesTagFilters');
  const sortedTags = Object.entries(_gamesAllTags)
    .filter(([tag]) => isGameTag(tag))
    .sort((a, b) => b[1] - a[1]);

  if (sortedTags.length === 0) { container.innerHTML = ''; return; }

  let html = '<span class="text-label text-muted mr-1">Tags:</span>';
  sortedTags.forEach(([tag, count]) => {
    const selected = _gamesSelectedTags.has(tag);
    html += `<button onclick="toggleGamesTag('${tag}')" class="px-2 py-1 text-meta font-medium rounded-md border transition-colors ${
      selected
        ? 'bg-sky-500/20 text-white border-sky-400/50'
        : 'bg-transparent text-secondary border-slate-700 hover:border-slate-500'
    }">${tagDisplayName(tag)} <span class="font-mono text-muted">${count}</span></button>`;
  });

  if (_gamesSelectedTags.size > 0) {
    html += `<button onclick="clearGamesTags()" class="text-label text-muted hover:text-white transition-colors ml-1">Clear</button>`;
  }
  container.innerHTML = html;
}

function renderGamesList(games) {
  // Cache games so analysis page can use SAN moves
  games.forEach(g => { _gamesCache[g.id] = g; });

  const resultLabel = { W:'Won', L:'Lost', D:'Draw' };
  const resultColor = { W:'text-good', L:'text-bad', D:'text-muted' };
  const resultBg = { W:'bg-good/20', L:'bg-red-500/20', D:'bg-slate-600/30' };

  const list = document.getElementById('gamesList');
  if (games.length === 0) {
    list.innerHTML = '<div class="card p-8 text-center text-muted text-label">No games found. Sync your games to get started.</div>';
    return;
  }

  list.innerHTML = games.map((g, i) => {
    const moveCount = Array.isArray(g.moves) ? Math.ceil(g.moves.length / 2) : (g.moves || 0);
    const tc = getGameType(g.timeControl || '');
    const acc = g.userColor === 'white' ? g.whiteAccuracy : g.blackAccuracy;
    const source = g.source === 'lichess' ? 'L' : 'C';
    const sourceBg = g.source === 'lichess' ? 'bg-white/20 text-white' : 'bg-good/20 text-good';
    const tags = (g.tags || []).filter(t => isGameTag(t));
    const finalFen = Array.isArray(g.moves) ? getFinalFen(g.moves) : null;
    const flip = g.userColor === 'black';

    return `
    <div class="card p-4 cursor-pointer transition-all hover:border-sky-400/40 fade-up" onclick="openGame(${g.id})" style="animation-delay:${0.05 + i * 0.03}s">
      <div class="flex items-center justify-between gap-3 min-w-0">
        <div class="flex items-center gap-4 min-w-0">
          ${finalFen ? `<div class="hidden sm:block w-48 h-48 rounded overflow-hidden shrink-0">${fenToMiniBoard(finalFen, flip)}</div>` : ''}
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="w-5 h-5 rounded text-meta font-bold flex items-center justify-center shrink-0 ${sourceBg}">${source}</span>
              <span class="text-sm text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-label font-mono text-muted">(${g.opponentRating})</span>` : ''}
              <span class="px-1.5 py-0.5 text-label font-semibold rounded ${resultBg[g.result] || ''} ${resultColor[g.result] || 'text-muted'}">${resultLabel[g.result] || g.result}</span>
            </div>
            <div class="flex items-center gap-2 flex-wrap mt-1 text-label text-muted">
              <span class="${tc.color} font-medium">${tc.label}</span>
              ${g.timeControl ? `<span class="text-slate-700">&middot;</span><span>${g.timeControl}</span>` : ''}
              <span class="text-slate-700">&middot;</span>
              <span>${moveCount} moves</span>
              <span class="text-slate-700">&middot;</span>
              <span>as ${g.userColor || 'white'}</span>
              ${tags.map(t => `<span class="px-1.5 py-0.5 text-label rounded border text-good whitespace-nowrap" style="border-color:var(--accent-dim)">${tagDisplayName(t)}</span>`).join('')}
            </div>
          </div>
        </div>
        <div class="flex items-center gap-3 shrink-0">
          ${acc != null ? `<span class="px-2 py-0.5 text-label font-mono font-semibold rounded bg-slate-800/60 ${acc >= 80 ? 'text-good' : acc >= 60 ? 'text-secondary' : 'text-bad'}">${Math.round(acc)}%</span>` : ''}
          ${g.hasAnalysis ? `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--good)" stroke-width="2.5" stroke-linecap="round"><path d="M5 13l4 4L19 7"/></svg>` : ''}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-muted"><path d="M9 18l6-6-6-6"/></svg>
        </div>
      </div>
    </div>`;
  }).join('');
}

function renderGamesPagination() {
  const totalPages = Math.ceil(_gamesTotal / GAMES_PER_PAGE);
  const container = document.getElementById('gamesPagination');

  if (totalPages <= 1) { container.innerHTML = ''; return; }

  const start = (_gamesPage - 1) * GAMES_PER_PAGE + 1;
  const end = Math.min(_gamesPage * GAMES_PER_PAGE, _gamesTotal);
  const isFirst = _gamesPage === 1;
  const isLast = _gamesPage === totalPages;

  const btnClass = (disabled) => disabled
    ? 'px-2 py-1 text-meta text-muted rounded border border-slate-800 opacity-40 cursor-default'
    : 'px-2 py-1 text-meta text-muted rounded border border-slate-700 hover:border-slate-500 cursor-pointer transition-colors';

  container.innerHTML = `
    <p class="text-label text-muted">Showing ${start}–${end} of ${_gamesTotal}</p>
    <div class="flex items-center gap-1">
      <button class="${btnClass(isFirst)}" ${isFirst ? '' : 'onclick="gamesGoToPage(1)"'}>First</button>
      <button class="${btnClass(isFirst)}" ${isFirst ? '' : `onclick="gamesGoToPage(${_gamesPage - 1})"`}>Prev</button>
      <span class="px-3 py-1 text-label text-muted font-mono">Page ${_gamesPage} of ${totalPages}</span>
      <button class="${btnClass(isLast)}" ${isLast ? '' : `onclick="gamesGoToPage(${_gamesPage + 1})"`}>Next</button>
      <button class="${btnClass(isLast)}" ${isLast ? '' : `onclick="gamesGoToPage(${totalPages})"`}>Last</button>
    </div>`;
}

async function gamesGoToPage(page) {
  _gamesPage = page;
  const token = localStorage.getItem('alpine_token');
  if (!token) return;

  const offset = (page - 1) * GAMES_PER_PAGE;
  const tagsParam = _gamesSelectedTags.size > 0 ? `&tags=${encodeURIComponent([..._gamesSelectedTags].join(','))}` : '';
  const res = await fetch(API_URL + `/api/games/stored?limit=${GAMES_PER_PAGE}&offset=${offset}${tagsParam}`, {
    headers: { 'Authorization': 'Bearer ' + token },
  });
  if (!res.ok) return;
  const data = await res.json();
  _gamesTotal = data.total || 0;
  renderGamesList(data.games || []);
  renderGamesPagination();
  // Scroll to top of games list
  document.getElementById('page-games').scrollIntoView({ behavior: 'smooth' });
}

async function toggleGamesTag(tag) {
  if (_gamesSelectedTags.has(tag)) _gamesSelectedTags.delete(tag);
  else _gamesSelectedTags.add(tag);
  _gamesPage = 1;

  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  const headers = { 'Authorization': 'Bearer ' + token };

  // Reload tags with selection context + reload games
  const tagsParam = _gamesSelectedTags.size > 0 ? `?selected_tags=${encodeURIComponent([..._gamesSelectedTags].join(','))}` : '';
  const gamesTagsParam = _gamesSelectedTags.size > 0 ? `&tags=${encodeURIComponent([..._gamesSelectedTags].join(','))}` : '';

  const [tagsRes, gamesRes] = await Promise.all([
    fetch(API_URL + '/api/games/tags' + tagsParam, { headers }),
    fetch(API_URL + `/api/games/stored?limit=${GAMES_PER_PAGE}&offset=0${gamesTagsParam}`, { headers }),
  ]);

  if (tagsRes.ok) { const d = await tagsRes.json(); _gamesAllTags = d.tags || {}; }
  if (gamesRes.ok) { const d = await gamesRes.json(); _gamesTotal = d.total || 0; renderGamesList(d.games || []); }

  renderGamesTags();
  renderGamesPagination();
}

async function clearGamesTags() {
  _gamesSelectedTags = new Set();
  _gamesPage = 1;
  await gamesGoToPage(1);

  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  const tagsRes = await fetch(API_URL + '/api/games/tags', { headers: { 'Authorization': 'Bearer ' + token } });
  if (tagsRes.ok) { const d = await tagsRes.json(); _gamesAllTags = d.tags || {}; }
  renderGamesTags();
}

function showGamesFeedback(msg, type) {
  const el = document.getElementById('gamesFeedback');
  const colors = {
    success: 'bg-cyan-500/20 border border-cyan-500/30 text-cyan-300',
    error: 'bg-red-500/20 border border-red-500/30 text-red-300',
    info: 'bg-violet-500/20 border border-violet-500/30 text-violet-300',
  };
  el.className = `mb-3 px-4 py-2 rounded-lg text-label flex items-center gap-2 ${colors[type] || colors.info}`;
  el.innerHTML = msg;
  setTimeout(() => { el.className = 'hidden mb-3 px-4 py-2 rounded-lg text-label flex items-center gap-2'; }, 8000);
}

async function gamesSync() {
  const btn = document.getElementById('btnSyncGames');
  btn.innerHTML = '<span class="w-3.5 h-3.5 border-2 border-slate-600 border-t-sky-400 rounded-full animate-spin"></span> Syncing...';
  btn.disabled = true;

  try {
    const token = localStorage.getItem('alpine_token');
    const res = await fetch(API_URL + '/api/games/sync', {
      method: 'POST', headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' }, body: '{}',
    });
    if (!res.ok) throw new Error('Sync failed');
    const data = await res.json();
    if (data.hasMoreHistory !== undefined) _gamesHasMoreHistory = data.hasMoreHistory;
    showGamesFeedback(`Synced ${data.synced} games`, 'success');

    // Invalidate dashboard so it re-fetches on next visit
    window._dashInit = false;

    // Reload everything
    _gamesPage = 1;
    _gamesSelectedTags = new Set();
    window._gamesInit = false;
    await initGames();
  } catch (err) {
    showGamesFeedback(err.message, 'error');
  } finally {
    btn.innerHTML = '<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg> Sync Games';
    btn.disabled = false;
  }
}

async function gamesBackfill() {
  const btn = document.getElementById('btnLoadHistory');
  btn.innerHTML = '<span class="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin"></span> Loading...';
  btn.disabled = true;

  try {
    const token = localStorage.getItem('alpine_token');
    const res = await fetch(API_URL + '/api/games/backfill', {
      method: 'POST', headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' }, body: '{}',
    });
    if (!res.ok) throw new Error('Backfill failed');
    const data = await res.json();
    _gamesHasMoreHistory = data.hasMoreHistory;
    showGamesFeedback(`Loaded ${data.synced} older games`, 'info');

    // Invalidate dashboard so it re-fetches on next visit
    window._dashInit = false;

    window._gamesInit = false;
    await initGames();
  } catch (err) {
    showGamesFeedback(err.message, 'error');
  } finally {
    btn.innerHTML = '<svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg> Load More History';
    btn.disabled = false;
  }
}

async function gamesAnalyze() {
  const btn = document.getElementById('btnAnalyze');
  const btnText = document.getElementById('btnAnalyzeText');
  btnText.textContent = 'Queueing...';
  btn.disabled = true;

  try {
    const token = localStorage.getItem('alpine_token');
    const res = await fetch(API_URL + '/api/games/analyze-server', {
      method: 'POST', headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
      body: JSON.stringify({ all_unanalyzed: true, limit: 1000 }),
    });
    if (!res.ok) throw new Error('Analysis failed');
    const data = await res.json();
    showGamesFeedback(`Queued ${data.queued} games for analysis. Results will appear automatically.`, 'success');
  } catch (err) {
    showGamesFeedback(err.message, 'error');
  } finally {
    btnText.textContent = `Analyze (${_gamesTotalUnanalyzed})`;
    btn.disabled = false;
  }
}

function openGame(id, source) {
  setAnalysisBackButton(source || 'games');
  loadGameAnalysis(id);
}
