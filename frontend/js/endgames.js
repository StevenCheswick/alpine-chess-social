// ══════════════════════════════════════════
// ENDGAMES PAGE INIT
// ══════════════════════════════════════════
async function initEndgames() {
  window._endgamesInit = true;
  const token = localStorage.getItem('alpine_token');
  if (!token) return;

  let data;
  try {
    const res = await fetch(API_URL + '/api/games/endgame-stats', { headers: { 'Authorization': 'Bearer ' + token } });
    if (!res.ok) throw new Error('Failed');
    data = await res.json();
  } catch (err) {
    document.getElementById('endgameTableBody').innerHTML = '<tr><td colspan="5" class="text-center py-6 text-label text-muted">No endgame data yet. Analyze some games first!</td></tr>';
    return;
  }

  const stats = (data.typeStats || []).sort((a, b) => {
    const edgeA = a.opponentAvgCpLoss - a.userAvgCpLoss;
    const edgeB = b.opponentAvgCpLoss - b.userAvgCpLoss;
    return edgeB - edgeA;
  });

  if (stats.length === 0) {
    document.getElementById('endgameTableBody').innerHTML = '<tr><td colspan="5" class="text-center py-6 text-label text-muted">No endgame data yet.</td></tr>';
    return;
  }

  const tbody = document.getElementById('endgameTableBody');
  tbody.innerHTML = stats.map((s, i) => {
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
