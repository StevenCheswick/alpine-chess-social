import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import { getEndgameStats, type EndgameStats } from '../services/endgameService';

export default function EndgameAnalyticsPage() {
  const { user } = useAuthStore();
  const [stats, setStats] = useState<EndgameStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const hasLinkedAccount = !!user?.chessComUsername;

  useEffect(() => {
    if (!hasLinkedAccount) {
      setLoading(false);
      return;
    }

    getEndgameStats()
      .then(setStats)
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [hasLinkedAccount]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-emerald-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-4xl mx-auto p-6">
        <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
          Failed to load endgame stats: {error}
        </div>
      </div>
    );
  }

  if (!hasLinkedAccount) {
    return (
      <div className="max-w-4xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Endgame Analytics</h1>
        <div className="card p-8 text-center">
          <h2 className="text-xl font-semibold text-white mb-2">No linked account</h2>
          <p className="text-slate-400 mb-4">
            Link your Chess.com account in settings to see your endgame analytics.
          </p>
          <Link
            to={`/u/${user?.username}`}
            className="inline-block px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg transition-colors"
          >
            Go to Settings
          </Link>
        </div>
      </div>
    );
  }

  if (!stats || stats.typeStats.length === 0) {
    return (
      <div className="max-w-4xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Endgame Analytics</h1>
        <div className="card p-8 text-center">
          <h2 className="text-xl font-semibold text-white mb-2">No endgame data yet</h2>
          <p className="text-slate-400 mb-4">
            Analyze some games to see your endgame performance breakdown.
          </p>
          <Link
            to="/games"
            className="inline-block px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg transition-colors"
          >
            Analyze Games
          </Link>
        </div>
      </div>
    );
  }

  // Sort by edge descending (strongest advantage first)
  const sorted = [...stats.typeStats].sort((a, b) => {
    const edgeA = a.opponentAvgCpLoss - a.userAvgCpLoss;
    const edgeB = b.opponentAvgCpLoss - b.userAvgCpLoss;
    return edgeB - edgeA;
  });

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Endgame Analytics</h1>
        <span className="text-sm text-slate-400">
          {stats.totalGamesWithEndgame} games with endgames
        </span>
      </div>

      <div className="card p-5">
        <h2 className="text-sm font-semibold text-white mb-4">By Endgame Type</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-slate-400 uppercase tracking-wider">
                <th className="text-left py-2 pr-4 font-medium">Type</th>
                <th className="text-right py-2 px-3 font-medium">Games</th>
                <th className="text-right py-2 px-3 font-medium">You</th>
                <th className="text-right py-2 px-3 font-medium">Opponent</th>
                <th className="text-right py-2 pl-3 font-medium">Edge</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((s, i) => {
                const edge = s.opponentAvgCpLoss - s.userAvgCpLoss;
                const edgeColor = edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-400';
                const userWins = s.userAvgCpLoss <= s.opponentAvgCpLoss;

                return (
                  <tr
                    key={s.type}
                    className={`border-t border-slate-800/50 ${i % 2 === 1 ? 'bg-slate-800/20' : ''}`}
                  >
                    <td className="py-2.5 pr-4 text-white font-medium">{s.type}</td>
                    <td className="py-2.5 px-3 text-right text-slate-300">{s.games}</td>
                    <td className={`py-2.5 px-3 text-right ${userWins ? 'text-emerald-400' : 'text-red-400'}`}>
                      {s.userAvgCpLoss.toFixed(1)}
                    </td>
                    <td className="py-2.5 px-3 text-right text-slate-300">
                      {s.opponentAvgCpLoss.toFixed(1)}
                    </td>
                    <td className={`py-2.5 pl-3 text-right font-semibold ${edgeColor}`}>
                      {edge > 0 ? '+' : ''}{edge.toFixed(1)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
