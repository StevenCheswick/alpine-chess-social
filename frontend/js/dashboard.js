// ══════════════════════════════════════════
// DASHBOARD INIT
// ══════════════════════════════════════════
async function initDashboard() {
  window._dashInit = true;
  // Destroy existing charts if re-initing
  ['accuracyChart', 'phaseChart', 'mistakeChart', 'ratingChart'].forEach(id => {
    const existing = Chart.getChart(document.getElementById(id));
    if (existing) existing.destroy();
  });
  // ── Auth ──
  const token = localStorage.getItem('alpine_token');
  if (!token) { console.warn('No auth token'); return; }
  const headers = { 'Authorization': 'Bearer ' + token };

  // ── Fetch stats ──
  let res, s;
  try {
    res = await fetch(API_URL + '/api/games/stats', { headers });
    if (!res.ok) { console.error('Stats fetch failed:', res.status); return; }
    s = await res.json();
  } catch (err) {
    console.error('Dashboard fetch error (is backend running?):', err.message);
    return;
  }

  // ── MIN_GAMES gate (same as React frontend) ──
  const MIN_GAMES = 100;
  const totalGames = s.totalAnalyzedGames || 0;
  if (totalGames < MIN_GAMES) {
    const pct = Math.round((totalGames / MIN_GAMES) * 100);
    document.getElementById('dashMinGamesCount').textContent = totalGames + ' / ' + MIN_GAMES + ' games';
    document.getElementById('dashMinGamesPct').textContent = pct + '%';
    document.getElementById('dashMinGamesBar').style.width = pct + '%';
    document.getElementById('dashMinGamesGate').style.display = '';
    document.getElementById('dashContent').style.display = 'none';
    return;
  }
  document.getElementById('dashMinGamesGate').style.display = 'none';
  document.getElementById('dashContent').style.display = '';

  // ── Hero cards ──
  const accArr = s.accuracyOverTime || [];
  const avgAcc = accArr.length ? Math.round(accArr.reduce((a,d) => a + d.accuracy, 0) / accArr.length) : 0;
  const GAUGE_C = 2 * Math.PI * 50;
  document.getElementById('dash-accuracy').innerHTML = avgAcc + '<span class="text-label">%</span>';
  document.querySelector('.gauge-fill').setAttribute('stroke-dashoffset', GAUGE_C * (1 - avgAcc / 100));

  document.getElementById('dash-games').textContent = s.totalAnalyzedGames;

  const allRatings = s.ratingOverTime || [];
  function tcCategory(tc) {
    const m = (tc || '').match(/^(\d+)/);
    if (!m) return null;
    let base = parseInt(m[1]);
    if (base > 60) base = base / 60;
    if (base < 3) return 'Bullet';
    if (base < 10) return 'Blitz';
    if (base < 30) return 'Rapid';
    return 'Classical';
  }
  const catCounts = {};
  allRatings.forEach(r => { const c = tcCategory(r.timeControl); if (c) catCounts[c] = (catCounts[c] || 0) + 1; });
  const topCat = Object.entries(catCounts).sort((a, b) => b[1] - a[1])[0];
  const modeName = topCat ? topCat[0] : 'Blitz';
  const ratings = allRatings.filter(r => tcCategory(r.timeControl) === modeName);
  document.getElementById('dash-rating-mode').textContent = modeName;
  if (ratings.length) {
    document.getElementById('dash-rating').textContent = ratings[ratings.length - 1].rating;
  }

  document.getElementById('dash-winrate').textContent = Math.round(s.winRate) + '%';
  document.getElementById('dash-wld').textContent = s.wins + 'W / ' + s.losses + 'L / ' + s.draws + 'D';

  // ── Choke & Clutch ──
  const cc = s.chokeClutch || {};
  document.getElementById('dash-choke').textContent = (cc.chokeRate ?? 0) + '%';
  document.getElementById('dash-choke-detail').textContent = (cc.choked || 0) + ' of ' + (cc.wasWinning || 0) + ' winning games lost';
  document.getElementById('dash-clutch').textContent = (cc.clutchRate ?? 0) + '%';
  document.getElementById('dash-clutch-detail').textContent = (cc.clutched || 0) + ' of ' + (cc.wasLosing || 0) + ' losing games won';

  // ── Move Quality Breakdown ──
  const mqOrder = ['book','best','excellent','good','inaccuracy','mistake','blunder'];
  const mqLabels = { book:'', best:'Best', excellent:'Exc', good:'Good', inaccuracy:'', mistake:'', blunder:'' };
  const mqTextClass = { book:'text-white/90', best:'text-white/90', excellent:'text-white/90', good:'text-slate-800/80', inaccuracy:'text-slate-800/80', mistake:'text-white/90', blunder:'text-white/90' };
  const mq = s.moveQualityBreakdown || {};
  const mqTotal = mqOrder.reduce((a,k) => a + (mq[k]||0), 0);

  const barEl = document.getElementById('dash-mqbar');
  const legendEl = document.getElementById('dash-mqlegend');
  barEl.innerHTML = mqOrder.map((k, i) => {
    const pct = mqTotal > 0 ? (mq[k]||0) / mqTotal * 100 : 0;
    if (pct < 0.5) return '';
    const rounded = i === 0 ? 'rounded-l-md' : i === mqOrder.length - 1 ? 'rounded-r-md' : '';
    const label = mqLabels[k] ? mqLabels[k] + ' ' + Math.round(pct) + '%' : Math.round(pct) + '%';
    return `<div class="h-full bg-move-${k} ${rounded} relative" style="width:${pct}%"><div class="absolute inset-0 flex items-center justify-center text-meta font-mono font-semibold ${mqTextClass[k]}">${pct >= 4 ? Math.round(pct) + '%' : ''}</div></div>`;
  }).join('');

  const mqNames = { book:'Book', best:'Best', excellent:'Excellent', good:'Good', inaccuracy:'Inaccuracy', mistake:'Mistake', blunder:'Blunder' };
  legendEl.innerHTML = mqOrder.map(k =>
    `<span class="flex items-center gap-1"><span class="w-1.5 h-1.5 rounded-sm bg-move-${k}"></span>${mqNames[k]}</span>`
  ).join('');

  // ── Charts ──
  const fmtDate = d => new Date(d).toLocaleDateString('en',{month:'short',day:'numeric'});

  // Accuracy Over Time
  new Chart(document.getElementById('accuracyChart'), {
    type: 'line',
    data: { labels: accArr.map(d=>fmtDate(d.date)), datasets: [{ data: accArr.map(d=>d.accuracy), borderColor: T('--chart-accuracy'), fill: false, tension: 0.4, pointRadius: 0, pointHoverRadius: 3, pointHoverBackgroundColor: T('--chart-accuracy'), borderWidth: 1.5 }] },
    options: { responsive: true, maintainAspectRatio: false, scales: { x: {...axisOpts, ticks:{...axisOpts.ticks, maxTicksLimit:5}}, y: {...axisOpts, min:50, max:100} }, interaction:{mode:'index',intersect:false} }
  });

  // Phase
  const phase = s.phaseAccuracyOverTime || [];
  new Chart(document.getElementById('phaseChart'), {
    type: 'line',
    data: { labels: phase.map(d=>fmtDate(d.date)), datasets: [
      { data: phase.map(d=>d.opening), borderColor:T('--chart-opening'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
      { data: phase.map(d=>d.middlegame), borderColor:T('--chart-middle'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
      { data: phase.map(d=>d.endgame), borderColor:T('--chart-endgame'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
    ]},
    options: { responsive:true, maintainAspectRatio:false, scales: { x:{...axisOpts,ticks:{...axisOpts.ticks,maxTicksLimit:5}}, y:{...axisOpts,min:50,max:100} }, interaction:{mode:'index',intersect:false} }
  });

  // Earliest Mistake
  const fm = s.firstInaccuracyOverTime || [];
  if (fm.length) {
    new Chart(document.getElementById('mistakeChart'), {
      type: 'line',
      data: { labels: fm.map(d=>fmtDate(d.date)), datasets: [
        { data: fm.map(d=>d.moveNumber), borderColor:T('--chart-inaccuracy'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
        { data: fm.map(d=>d.mistakeMoveNumber), borderColor:T('--chart-mistake'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
        { data: fm.map(d=>d.blunderMoveNumber), borderColor:T('--chart-blunder'), tension:0.4, pointRadius:0, pointHoverRadius:3, borderWidth:1.5 },
      ]},
      options: { responsive:true, maintainAspectRatio:false, scales: { x:{...axisOpts,ticks:{...axisOpts.ticks,maxTicksLimit:5}}, y:{...axisOpts,min:0} }, interaction:{mode:'index',intersect:false} }
    });
  }

  // Rating
  if (ratings.length) {
    new Chart(document.getElementById('ratingChart'), {
      type: 'line',
      data: { labels: ratings.map(d=>fmtDate(d.date)), datasets: [{ data: ratings.map(d=>d.rating), borderColor:T('--chart-rating'), fill:false, tension:0.4, pointRadius:0, pointHoverRadius:3, pointHoverBackgroundColor:T('--chart-rating'), borderWidth:1.5 }] },
      options: { responsive:true, maintainAspectRatio:false, scales: { x:{...axisOpts,ticks:{...axisOpts.ticks,maxTicksLimit:5}}, y:{...axisOpts} }, interaction:{mode:'index',intersect:false} }
    });
  }

  // ── Game lists ──
  const resultLabel = { W:'Won', L:'Lost', D:'Draw' };
  function renderGameList(container, games, good) {
    container.innerHTML = games.map((g, i) => `
      <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openGame(${g.gameId}, 'dashboard')">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
          <div class="min-w-0">
            <div class="flex items-center gap-1">
              <span class="inline-block w-2.5 h-2.5 rounded-full shrink-0 ${g.result==='W'?'bg-good':g.result==='L'?'bg-bad':'bg-slate-400'}"></span>
              <span class="text-body text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-meta text-muted font-mono">(${g.opponentRating})</span>` : ''}
            </div>
          </div>
        </div>
        <span class="text-xs font-bold ${good?'text-good':'text-bad'} font-mono">${g.accuracy}%</span>
      </div>
    `).join('');
  }
  renderGameList(document.getElementById('mostAccurateList'), s.mostAccurateGames || [], true);
  renderGameList(document.getElementById('leastAccurateList'), s.leastAccurateGames || [], false);

  // ── Opening Habits ──
  const prepEl = document.getElementById('deepestPrepList');
  const cleanLines = s.cleanestLines || [];
  const blunders = s.openingBlunders || [];
  // Store for click handlers
  window._cleanLines = cleanLines;
  window._openingBlunders = blunders;

  prepEl.innerHTML = cleanLines.map((c, i) => `
    <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openOpeningLine('clean', ${i})">
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
        <div class="min-w-0">
          <div class="text-body text-secondary font-medium truncate">${c.line}</div>
          <div class="text-meta text-muted">${c.color} &middot; ${c.gameCount} game${c.gameCount===1?'':'s'}</div>
        </div>
      </div>
      <span class="text-xs font-bold text-good font-mono">${c.cleanDepth} moves</span>
    </div>
  `).join('');

  const habitsEl = document.getElementById('costliestHabitsList');
  habitsEl.innerHTML = blunders.map((b, i) => {
    const lastSp = b.line.lastIndexOf(' ');
    const prefix = lastSp > 0 ? b.line.slice(0, lastSp) : '';
    const badMove = lastSp > 0 ? b.line.slice(lastSp + 1) : b.line;
    return `
    <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openOpeningLine('blunder', ${i})">
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
        <div class="min-w-0">
          <div class="text-body truncate"><span class="text-secondary">${prefix} </span><span class="text-bad font-semibold">${badMove}</span></div>
          <div class="text-meta text-muted">${b.mistakeCount}&times; as ${b.color}</div>
        </div>
      </div>
      <span class="text-xs font-bold text-bad font-mono">-${b.avgCpLoss} cp</span>
    </div>`;
  }).join('');

  // ── Tactics stats ──
  try {
    const pRes = await fetch(API_URL + '/api/puzzles/stats', { headers });
    if (pRes.ok) {
      const stats = await pRes.json();
      const PG = 2 * Math.PI * 50;
      const user = stats.user || {};
      const opp = stats.opponent || {};
      const userRate = Math.round(user.rate || 0);
      const oppRate = Math.round(opp.rate || 0);
      const edge = userRate - oppRate;

      document.getElementById('dashPuzzleGauges').innerHTML = `
        <div class="flex flex-col items-center">
          <div class="relative w-28 h-28">
            <svg class="gauge-ring w-full h-full" viewBox="0 0 120 120">
              <circle class="gauge-track" cx="60" cy="60" r="50" fill="none" stroke-width="8" />
              <circle class="gauge-fill" cx="60" cy="60" r="50" fill="none"
                stroke="url(#gGlacierDashPuzzle)" stroke-width="8"
                stroke-dasharray="${PG}" stroke-dashoffset="${PG * (1 - userRate / 100)}" />
              <defs><linearGradient id="gGlacierDashPuzzle" x1="0%" y1="0%" x2="100%" y2="100%">
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
                stroke-dasharray="${PG}" stroke-dashoffset="${PG * (1 - oppRate / 100)}" />
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

      const positions = stats.byPosition || [];
      const posLabels = { winning: 'Winning Positions', equal: 'Equal Positions', losing: 'Losing Positions' };
      document.getElementById('dashPuzzlePositions').innerHTML = positions.map(p => {
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

      const themes = (stats.byTheme || []).filter(t => (t.user?.total || 0) >= 10 && isVisibleTag(t.theme));
      const alphaSort = (a, b) => tagDisplayName(a.theme).localeCompare(tagDisplayName(b.theme));
      const mateThemes = themes.filter(t => t.theme.toLowerCase().includes('mate')).sort(alphaSort);
      const tacticThemes = themes.filter(t => !t.theme.toLowerCase().includes('mate')).sort(alphaSort);

      function dashThemeTable(title, items) {
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

      document.getElementById('dashPuzzleThemeTables').innerHTML =
        dashThemeTable('By Tactic', tacticThemes) + dashThemeTable('By Checkmate Pattern', mateThemes);
    }
  } catch (err) {
    document.getElementById('dashPuzzleGauges').innerHTML = '<div class="col-span-3 text-center text-label text-muted py-4">No puzzle data yet.</div>';
  }

  // ── Endgame stats ──
  try {
    const eRes = await fetch(API_URL + '/api/games/endgame-stats', { headers });
    if (eRes.ok) {
      const data = await eRes.json();
      const egStats = (data.typeStats || []).sort((a, b) => {
        return (b.opponentAvgCpLoss - b.userAvgCpLoss) - (a.opponentAvgCpLoss - a.userAvgCpLoss);
      });

      const tbody = document.getElementById('dashEndgameTableBody');
      if (egStats.length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" class="text-center py-6 text-label text-muted">No endgame data yet.</td></tr>';
      } else {
        tbody.innerHTML = egStats.map((s, i) => {
          const edge = s.opponentAvgCpLoss - s.userAvgCpLoss;
          const edgeColor = Math.abs(edge) < 0.5 ? 'text-muted' : edge > 0 ? 'text-good' : 'text-bad';
          return `
            <tr class="border-t border-slate-800/50 ${i % 2 === 1 ? 'bg-slate-800/20' : ''}">
              <td class="py-2.5 pr-4 text-white font-medium">${s.type}</td>
              <td class="py-2.5 px-3 text-right font-mono text-white">${s.userAvgCpLoss.toFixed(1)}</td>
              <td class="py-2.5 px-3 text-right font-mono text-muted">${s.opponentAvgCpLoss.toFixed(1)}</td>
              <td class="py-2.5 px-3 text-right font-mono font-semibold ${edgeColor}">${edge > 0 ? '+' : ''}${edge.toFixed(1)}</td>
              <td class="py-2.5 pl-3 text-right font-mono text-muted">${s.games}</td>
            </tr>`;
        }).join('');
      }
    }
  } catch (err) {
    document.getElementById('dashEndgameTableBody').innerHTML = '<tr><td colspan="5" class="text-center py-6 text-label text-muted">No endgame data yet.</td></tr>';
  }

  // ── Smoothest Wins ──
  const sw = s.smoothestWins || [];
  const swEl = document.getElementById('dashSmoothestWins');
  if (sw.length === 0) {
    swEl.innerHTML = '<div class="text-center text-label text-muted py-6">No qualifying games yet.</div>';
  } else {
    swEl.innerHTML = sw.map((g, i) => {
      const scoreColor = 'text-accent';
      return `
      <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openGame(${g.gameId}, 'dashboard')">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
          <div class="min-w-0">
            <div class="flex items-center gap-1">
              <span class="inline-block w-2.5 h-2.5 rounded-full shrink-0 bg-good"></span>
              <span class="text-body text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-meta text-muted font-mono">(${g.opponentRating})</span>` : ''}
            </div>
          </div>
        </div>
        <span class="text-xs font-bold ${scoreColor} font-mono">${g.maxDelta}cp</span>
      </div>`;
    }).join('');
  }

  // ── Roller Coasters ──
  const rc = s.rollerCoasters || [];
  const rcEl = document.getElementById('dashRollerCoasters');
  if (rc.length === 0) {
    rcEl.innerHTML = '<div class="text-center text-label text-muted py-6">No qualifying games yet.</div>';
  } else {
    rcEl.innerHTML = rc.map((g, i) => {
      const swingColor = g.swings >= 6 ? 'text-bad' : g.swings >= 4 ? 'text-warning' : 'text-accent';
      return `
      <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openGame(${g.gameId}, 'dashboard')">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
          <div class="min-w-0">
            <div class="flex items-center gap-1">
              <span class="inline-block w-2.5 h-2.5 rounded-full shrink-0 ${g.result==='W'?'bg-good':g.result==='L'?'bg-bad':'bg-slate-400'}"></span>
              <span class="text-body text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-meta text-muted font-mono">(${g.opponentRating})</span>` : ''}
            </div>
          </div>
        </div>
        <span class="text-xs font-bold ${swingColor} font-mono">${g.swings} swings</span>
      </div>`;
    }).join('');
  }

  // ── Swindles ──
  const swd = s.swindles || [];
  const swdEl = document.getElementById('dashSwindles');
  if (swd.length === 0) {
    swdEl.innerHTML = '<div class="text-center text-label text-muted py-6">No qualifying games yet.</div>';
  } else {
    swdEl.innerHTML = swd.map((g, i) => {
      return `
      <div class="game-row flex items-center justify-between px-1.5 py-2 rounded-lg min-w-0" onclick="openGame(${g.gameId}, 'dashboard')">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-label text-muted font-mono w-3 text-right shrink-0">${i+1}</span>
          <div class="min-w-0">
            <div class="flex items-center gap-1">
              <span class="inline-block w-2.5 h-2.5 rounded-full shrink-0 bg-good"></span>
              <span class="text-body text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-meta text-muted font-mono">(${g.opponentRating})</span>` : ''}
            </div>
          </div>
        </div>
        <span class="text-xs font-bold text-accent font-mono">${g.maxDelta}cp</span>
      </div>`;
    }).join('');
  }
}
