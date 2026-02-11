import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import { getEndgameStats, type EndgameStats } from '../services/endgameService';

export default function EndgameAnalyticsPage() {
  const { user } = useAuthStore();
  const [stats, setStats] = useState<EndgameStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const hasLinkedAccount = !!(user?.chessComUsername || user?.lichessUsername);

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
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-8 text-center">
          <h2 className="text-xl font-semibold text-white mb-2">No linked account</h2>
          <p className="text-slate-400 mb-4">
            Link your Chess.com or Lichess account in settings to see your endgame analytics.
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
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-8 text-center">
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

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Endgame Analytics</h1>
        <span className="text-sm text-slate-400">
          {stats.totalGamesWithEndgame} games with endgames
        </span>
      </div>

      <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
        <h2 className="text-lg font-semibold text-white mb-1">Breakdown</h2>
        <p className="text-sm text-slate-500 mb-4">
          Lower CP loss is better. Positive edge means you outplay your opponents.
        </p>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-slate-400 border-b border-slate-700">
                <th className="text-left py-2 pr-4">Endgame Type</th>
                <th className="text-right py-2 px-4">Games</th>
                <th className="text-right py-2 px-4">Your Avg CP Loss</th>
                <th className="text-right py-2 px-4">Opp Avg CP Loss</th>
                <th className="text-right py-2 pl-4">Edge</th>
              </tr>
            </thead>
            <tbody>
              {stats.typeStats.map((s) => {
                const edge = s.opponentAvgCpLoss - s.userAvgCpLoss;
                const edgeColor = edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-400';
                return (
                  <tr key={s.type} className="border-b border-slate-700/50 hover:bg-slate-700/30">
                    <td className="py-2.5 pr-4 text-white">{s.type}</td>
                    <td className="py-2.5 px-4 text-right text-slate-300">{s.games}</td>
                    <td className="py-2.5 px-4 text-right text-emerald-400">{s.userAvgCpLoss}</td>
                    <td className="py-2.5 px-4 text-right text-indigo-400">{s.opponentAvgCpLoss}</td>
                    <td className={`py-2.5 pl-4 text-right font-medium ${edgeColor}`}>
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
