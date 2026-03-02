// ══════════════════════════════════════════
// ANALYSIS PAGE INIT
// ══════════════════════════════════════════
let _currentGameId = null;
let _analysisSource = 'games'; // tracks where user came from: 'games' or 'dashboard'

function analysisGoBack() {
  switchPage(_analysisSource);
}

function setAnalysisBackButton(source) {
  _analysisSource = source;
  const label = document.getElementById('analysisBackLabel');
  if (label) label.textContent = source === 'dashboard' ? 'Back to Dashboard' : 'Back to Games';
}

function initAnalysis() {
  window._analysisInit = true;
  setCgBoard('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', 'white');
}

function setCgBoard(fen, orientation) {
  if (!Chessground) {
    // Libs not loaded yet, retry shortly
    setTimeout(() => setCgBoard(fen, orientation), 200);
    return;
  }
  const el = document.getElementById('chessboard');
  if (_cgInstance) {
    _cgInstance.set({ fen, orientation, viewOnly: true });
  } else {
    el.innerHTML = '';
    _cgInstance = Chessground(el, {
      fen,
      orientation,
      viewOnly: true,
      coordinates: true,
      animation: { duration: 250 },
    });
    setTimeout(resizeBoards, 50);
  }
}

async function loadGameAnalysis(gameId) {
  _currentGameId = gameId;
  window._analysisInit = true;
  switchPage('analysis');

  const token = localStorage.getItem('alpine_token');
  if (!token) return;
  const headers = { 'Authorization': 'Bearer ' + token };

  // Fetch game + analysis in parallel
  const [gameRes, analysisRes] = await Promise.all([
    fetch(API_URL + `/api/games/${gameId}`, { headers }).catch(() => null),
    fetch(API_URL + `/api/games/${gameId}/analysis`, { headers }).catch(() => null),
  ]);

  const game = gameRes?.ok ? await gameRes.json() : null;
  const analysis = analysisRes?.ok ? await analysisRes.json() : null;

  if (!game) {
    document.getElementById('analysisHeader').innerHTML = '<div class="text-center py-4 text-label text-bad">Game not found</div>';
    return;
  }

  // Get SAN moves (prefer cached from games list)
  const cached = _gamesCache[gameId];
  const sanMoves = (cached && Array.isArray(cached.moves)) ? cached.moves : (Array.isArray(game.moves) ? game.moves : []);

  // Build positions array using chess.js
  _analysisPositions = [];
  _analysisMoveIndex = -1;
  _analysisMoveCount = sanMoves.length;

  if (Chess && sanMoves.length > 0) {
    _chessInstance = new Chess();
    _analysisPositions.push(_chessInstance.fen()); // starting position = index -1
    for (const san of sanMoves) {
      try { _chessInstance.move(san); } catch { break; }
      _analysisPositions.push(_chessInstance.fen());
    }
    _analysisMoveCount = _analysisPositions.length - 1; // exclude starting pos
  }

  // Store analysis data for eval bar updates
  window._analysisData = analysis;

  const userSide = game.userColor || 'white';
  const oppSide = userSide === 'white' ? 'black' : 'white';

  // Header
  const user = JSON.parse(localStorage.getItem('alpine_user') || '{}');
  const username = user.chessComUsername || user.username || 'You';
  const userRating = game.userRating || '?';
  const oppRating = game.opponentRating || '?';
  const moveCount = Math.ceil(sanMoves.length / 2);
  const resultMap = { W: '1 - 0', L: '0 - 1', D: '½ - ½' };
  const resultColor = { W: 'text-good', L: 'text-bad', D: 'text-muted' };

  document.getElementById('analysisHeader').innerHTML = `
    <div class="flex items-center justify-between">
      <div>
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold text-white">${username}</span>
          <span class="text-label font-mono text-muted">(${userRating})</span>
          <span class="text-label ${resultColor[game.result] || 'text-muted'} font-semibold">${resultMap[game.result] || game.result}</span>
          <span class="text-label font-mono text-muted">(${oppRating})</span>
          <span class="text-sm font-semibold text-white">${game.opponent}</span>
        </div>
        <div class="flex items-center gap-3 mt-1 text-label text-muted">
          ${game.timeControl ? `<span>${game.timeControl}</span><span>&middot;</span>` : ''}
          <span>${moveCount} moves</span>
          ${game.date ? `<span>&middot;</span><span>${new Date(game.date).toLocaleDateString('en', { month:'short', day:'numeric', year:'numeric' })}</span>` : ''}
        </div>
      </div>
    </div>`;

  // Set board to starting position with user's color orientation
  setCgBoard('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', userSide);

  // Move quality grid
  const qualityClass = { book:'move-book', best:'move-best', excellent:'move-excellent', good:'move-good', inaccuracy:'move-inaccuracy', mistake:'move-mistake', blunder:'move-blunder', forced:'text-muted' };

  if (analysis && analysis.moves) {
    const counts = { white: {}, black: {} };
    const qualities = ['book','best','excellent','good','inaccuracy','mistake','blunder','forced'];
    qualities.forEach(q => { counts.white[q] = 0; counts.black[q] = 0; });

    analysis.moves.forEach((m, i) => {
      const side = i % 2 === 0 ? 'white' : 'black';
      const q = (m.classification || '').toLowerCase();
      if (counts[side][q] !== undefined) counts[side][q]++;
    });

    let mqHtml = `<div class="grid grid-cols-[1fr_50px_50px] gap-x-3 gap-y-1.5 text-label">
      <div></div>
      <div class="text-center text-meta text-muted uppercase tracking-wider font-medium">You</div>
      <div class="text-center text-meta text-muted uppercase tracking-wider font-medium" style="opacity:0.6">Opp</div>`;

    qualities.forEach(q => {
      const bgClass = q === 'forced' ? 'bg-slate-600/80' : `bg-move-${q}`;
      mqHtml += `
        <div class="flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-sm ${bgClass}"></span><span class="text-muted">${q.charAt(0).toUpperCase() + q.slice(1)}</span></div>
        <div class="text-center font-mono text-white">${counts[userSide][q]}</div>
        <div class="text-center font-mono text-muted" style="opacity:0.6">${counts[oppSide][q]}</div>`;
    });

    const userAcc = userSide === 'white' ? analysis.white_accuracy : analysis.black_accuracy;
    const oppAcc = oppSide === 'white' ? analysis.white_accuracy : analysis.black_accuracy;
    mqHtml += `</div><div class="gradient-line mt-3 mb-3"></div>
      <div class="grid grid-cols-[1fr_50px_50px] gap-x-3 text-label">
        <div class="flex items-center gap-1.5"><span class="text-muted font-medium">Accuracy</span></div>
        <div class="text-center font-mono font-bold text-good">${userAcc != null ? Math.round(userAcc) + '%' : '—'}</div>
        <div class="text-center font-mono font-semibold text-muted" style="opacity:0.6">${oppAcc != null ? Math.round(oppAcc) + '%' : '—'}</div>
      </div>`;

    document.getElementById('analysisMqGrid').innerHTML = mqHtml;
  } else {
    document.getElementById('analysisMqGrid').innerHTML = '<div class="text-label text-muted py-2">No analysis data. Queue this game for analysis on the Games page.</div>';
  }

  // Move list — clickable moves
  const ml = document.getElementById('moveList');
  let moveHtml = '';
  const aMoves = analysis?.moves || [];
  for (let i = 0; i < sanMoves.length; i += 2) {
    const moveNum = Math.floor(i / 2) + 1;
    const wSan = sanMoves[i] || '';
    const bSan = sanMoves[i + 1] || '';
    const wClass = aMoves[i] ? (aMoves[i].classification || '').toLowerCase() : '';
    const bClass = aMoves[i + 1] ? (aMoves[i + 1].classification || '').toLowerCase() : '';

    moveHtml += `<span class="text-secondary mr-1">${moveNum}.</span>`;
    moveHtml += `<span class="move-item ${qualityClass[wClass] || ''}" data-mi="${i}" onclick="analysisGoTo(${i})">${wSan}</span> `;
    if (bSan) moveHtml += `<span class="move-item ${qualityClass[bClass] || ''}" data-mi="${i+1}" onclick="analysisGoTo(${i+1})">${bSan}</span> `;
  }
  ml.innerHTML = moveHtml;

  // Update eval bar to starting position
  updateEvalBar(-1);
}

// ── Move navigation ──
function analysisGoTo(idx) {
  if (idx < -1 || idx >= _analysisMoveCount) return;
  _analysisMoveIndex = idx;
  const fen = _analysisPositions[idx + 1]; // +1 because index 0 = starting pos
  if (fen && _cgInstance) {
    _cgInstance.set({ fen, animation: { enabled: true } });
  }
  updateEvalBar(idx);
  highlightActiveMove(idx);
}

function analysisNav(dir) {
  switch (dir) {
    case 'first': analysisGoTo(-1); break;
    case 'prev': analysisGoTo(_analysisMoveIndex - 1); break;
    case 'next': analysisGoTo(_analysisMoveIndex + 1); break;
    case 'last': analysisGoTo(_analysisMoveCount - 1); break;
  }
}

function updateEvalBar(moveIdx) {
  const analysis = window._analysisData;
  const fillEl = document.getElementById('analysisEvalFill');
  const textEl = document.getElementById('analysisEvalText');
  if (!fillEl || !textEl) return;

  if (!analysis || !analysis.moves || moveIdx < 0) {
    fillEl.style.height = '50%';
    textEl.textContent = '0.0';
    return;
  }

  const m = analysis.moves[moveIdx];
  if (!m) return;

  // Use move_eval (from white's perspective)
  const cp = m.move_eval ?? 0;
  // Convert centipawns to bar height (50% = even, clamped 5-95%)
  const pct = Math.max(5, Math.min(95, 50 + (cp / 10)));
  fillEl.style.height = pct + '%';

  // Format eval text
  if (Math.abs(cp) > 9000) {
    textEl.textContent = cp > 0 ? 'M' : '-M';
  } else {
    textEl.textContent = (cp >= 0 ? '+' : '') + (cp / 100).toFixed(1);
  }
}

function highlightActiveMove(idx) {
  document.querySelectorAll('#moveList .move-item').forEach(el => {
    el.classList.remove('active-move');
  });
  if (idx >= 0) {
    const el = document.querySelector(`#moveList .move-item[data-mi="${idx}"]`);
    if (el) {
      el.classList.add('active-move');
      el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }
}

// ── Opening Line Viewer (from dashboard habits) ──
function openOpeningLine(type, idx) {
  const data = type === 'blunder' ? (window._openingBlunders || [])[idx] : (window._cleanLines || [])[idx];
  if (!data || !data.moves || !Chess) return;

  // Build positions from SAN moves
  _analysisPositions = [];
  _chessInstance = new Chess();
  _analysisPositions.push(_chessInstance.fen());
  for (const san of data.moves) {
    try { _chessInstance.move(san); } catch { break; }
    _analysisPositions.push(_chessInstance.fen());
  }
  _analysisMoveCount = _analysisPositions.length - 1;

  const userSide = data.color || 'white';
  const userIsWhite = userSide === 'white';

  // Build synthetic analysis (like React's OpeningLinePage)
  const syntheticMoves = data.moves.map((san, i) => {
    const isUserMove = userIsWhite ? (i % 2 === 0) : (i % 2 === 1);
    let classification = 'book', cpLoss = 0, bestMove = '';
    if (type === 'blunder' && i === data.ply && isUserMove) {
      cpLoss = data.avgCpLoss;
      classification = cpLoss >= 200 ? 'blunder' : cpLoss >= 100 ? 'mistake' : cpLoss >= 50 ? 'inaccuracy' : 'good';
      bestMove = data.bestMove || '';
    }
    return { move: san, move_eval: 0, best_move: bestMove, best_eval: 0, cp_loss: cpLoss, classification };
  });

  // Build classification counts
  const counts = { white: {}, black: {} };
  const qualities = ['book','best','excellent','good','inaccuracy','mistake','blunder','forced'];
  qualities.forEach(q => { counts.white[q] = 0; counts.black[q] = 0; });
  syntheticMoves.forEach((m, i) => {
    const side = i % 2 === 0 ? 'white' : 'black';
    if (counts[side][m.classification] !== undefined) counts[side][m.classification]++;
  });

  window._analysisData = { moves: syntheticMoves, white_accuracy: null, black_accuracy: null };

  // Switch to analysis page
  setAnalysisBackButton('dashboard');
  switchPage('analysis');
  window._analysisInit = true;

  // Header
  const subtitle = type === 'blunder'
    ? `Repeated ${data.mistakeCount} times as ${data.color} · avg -${data.avgCpLoss} cp`
    : `${data.cleanDepth} moves deep as ${data.color} · ${data.gameCount} game${data.gameCount === 1 ? '' : 's'}`;

  document.getElementById('analysisHeader').innerHTML = `
    <div>
      <div class="text-sm font-semibold text-white">${data.line}</div>
      <div class="text-label text-muted mt-0.5">${subtitle}</div>
    </div>`;

  setCgBoard('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', userSide);

  // Move quality grid
  const qualityClass = { book:'move-book', best:'move-best', excellent:'move-excellent', good:'move-good', inaccuracy:'move-inaccuracy', mistake:'move-mistake', blunder:'move-blunder', forced:'text-muted' };
  const oppSide = userSide === 'white' ? 'black' : 'white';
  let mqHtml = `<div class="grid grid-cols-[1fr_50px_50px] gap-x-3 gap-y-1.5 text-label">
    <div></div>
    <div class="text-center text-meta text-muted uppercase tracking-wider font-medium">You</div>
    <div class="text-center text-meta text-muted uppercase tracking-wider font-medium" style="opacity:0.6">Opp</div>`;
  qualities.forEach(q => {
    const bgClass = q === 'forced' ? 'bg-slate-600/80' : `bg-move-${q}`;
    mqHtml += `
      <div class="flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-sm ${bgClass}"></span><span class="text-muted">${q.charAt(0).toUpperCase() + q.slice(1)}</span></div>
      <div class="text-center font-mono text-white">${counts[userSide][q]}</div>
      <div class="text-center font-mono text-muted" style="opacity:0.6">${counts[oppSide][q]}</div>`;
  });
  mqHtml += `</div>`;
  document.getElementById('analysisMqGrid').innerHTML = mqHtml;

  // Move list
  const ml = document.getElementById('moveList');
  let moveHtml = '';
  for (let i = 0; i < data.moves.length; i += 2) {
    const moveNum = Math.floor(i / 2) + 1;
    const wSan = data.moves[i] || '';
    const bSan = data.moves[i + 1] || '';
    const wClass = syntheticMoves[i] ? syntheticMoves[i].classification : '';
    const bClass = syntheticMoves[i + 1] ? syntheticMoves[i + 1].classification : '';
    moveHtml += `<span class="text-secondary mr-1">${moveNum}.</span>`;
    moveHtml += `<span class="move-item ${qualityClass[wClass] || ''}" data-mi="${i}" onclick="analysisGoTo(${i})">${wSan}</span> `;
    if (bSan) moveHtml += `<span class="move-item ${qualityClass[bClass] || ''}" data-mi="${i+1}" onclick="analysisGoTo(${i+1})">${bSan}</span> `;
  }
  ml.innerHTML = moveHtml;

  // Jump to blunder move, or start for clean lines
  const startIdx = type === 'blunder' ? data.ply : -1;
  _analysisMoveIndex = -1;
  analysisGoTo(startIdx);
}

// Keyboard navigation for analysis
document.addEventListener('keydown', (e) => {
  // Only when on analysis page
  const analysisPage = document.getElementById('page-analysis');
  if (!analysisPage || !analysisPage.classList.contains('active')) return;

  if (e.key === 'ArrowLeft') { e.preventDefault(); analysisNav('prev'); }
  else if (e.key === 'ArrowRight') { e.preventDefault(); analysisNav('next'); }
  else if (e.key === 'Home') { e.preventDefault(); analysisNav('first'); }
  else if (e.key === 'End') { e.preventDefault(); analysisNav('last'); }
});
