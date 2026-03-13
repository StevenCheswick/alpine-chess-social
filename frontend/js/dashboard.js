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
  const ratings = allRatings.filter(r => {
    const tc = r.timeControl || '';
    const m = tc.match(/^(\d+)/);
    if (!m) return false;
    let base = parseInt(m[1]);
    if (base > 60) base = base / 60;
    return base >= 3 && base < 10; // Blitz only
  });
  if (ratings.length) {
    document.getElementById('dash-rating').textContent = ratings[ratings.length - 1].rating;
  }

  document.getElementById('dash-winrate').textContent = Math.round(s.winRate) + '%';
  document.getElementById('dash-wld').textContent = s.wins + 'W / ' + s.losses + 'L / ' + s.draws + 'D';

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
              <span class="text-body text-white font-medium truncate">vs ${g.opponent}</span>
              ${g.opponentRating ? `<span class="text-meta text-muted font-mono">(${g.opponentRating})</span>` : ''}
            </div>
            <div class="flex items-center gap-1.5 text-meta">
              <span class="${g.result==='W'?'text-good':g.result==='L'?'text-bad':'text-secondary'}">${resultLabel[g.result]||g.result}</span>
              <span class="text-slate-700">&middot;</span>
              <span class="text-secondary">${g.date}</span>
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
}
