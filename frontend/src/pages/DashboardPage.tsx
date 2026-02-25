import { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  ResponsiveContainer,
  LineChart,
  Line,
  BarChart,
  Bar,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';
import { useAuthStore } from '../stores/authStore';
import { getStats, type DashboardStats, type GameSummary, type OpeningBlunder, type CleanLine } from '../services/dashboardService';

const CLASSIFICATION_COLORS: Record<string, string> = {
  book: '#06b6d4',
  best: '#22c55e',
  excellent: '#4ade80',
  good: '#86efac',
  inaccuracy: '#facc15',
  mistake: '#f97316',
  blunder: '#ef4444',
};

const CLASSIFICATION_LABELS: Record<string, string> = {
  book: 'Book',
  best: 'Best',
  excellent: 'Excellent',
  good: 'Good',
  inaccuracy: 'Inaccuracy',
  mistake: 'Mistake',
  blunder: 'Blunder',
};

const MIN_GAMES = 100;

function CustomTooltip({ active, payload, label }: { active?: boolean; payload?: Array<{ value: number; name: string }>; label?: string }) {
  if (!active || !payload?.length) return null;
  return (
    <div className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm">
      <p className="text-slate-400 mb-1">{label}</p>
      {payload.map((entry, i) => (
        <p key={i} className="text-white font-medium">
          {entry.name}: {entry.value}
        </p>
      ))}
    </div>
  );
}

export default function DashboardPage() {
  const { user } = useAuthStore();
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const hasLinkedAccount = !!user?.chessComUsername;

  useEffect(() => {
    if (!hasLinkedAccount) {
      setLoading(false);
      return;
    }

    getStats()
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
          Failed to load dashboard: {error}
        </div>
      </div>
    );
  }

  if (!hasLinkedAccount) {
    return (
      <div className="max-w-4xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Dashboard</h1>
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-8 text-center">
          <svg className="w-16 h-16 text-slate-600 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
          </svg>
          <h2 className="text-xl font-semibold text-white mb-2">No linked account</h2>
          <p className="text-slate-400 mb-4">
            Link your Chess.com account in settings to see your dashboard.
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

  if (!stats || stats.totalAnalyzedGames < MIN_GAMES) {
    const count = stats?.totalAnalyzedGames ?? 0;
    const percent = Math.round((count / MIN_GAMES) * 100);
    return (
      <div className="max-w-4xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Dashboard</h1>
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-8 text-center">
          <svg className="w-16 h-16 text-slate-600 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
          <h2 className="text-xl font-semibold text-white mb-2">Not enough analyzed games</h2>
          <p className="text-slate-400 mb-4">
            You need at least {MIN_GAMES} analyzed games to view your dashboard.
          </p>
          <div className="max-w-xs mx-auto mb-2">
            <div className="flex justify-between text-sm text-slate-400 mb-1">
              <span>{count} / {MIN_GAMES} games</span>
              <span>{percent}%</span>
            </div>
            <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
              <div
                className="h-full bg-emerald-500 rounded-full transition-all"
                style={{ width: `${percent}%` }}
              />
            </div>
          </div>
          <Link
            to="/games"
            className="inline-block mt-4 px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg transition-colors"
          >
            Analyze Games
          </Link>
        </div>
      </div>
    );
  }

  const smoothedAccuracy = stats.accuracyOverTime;
  const smoothedPhase = stats.phaseAccuracyOverTime;
  const smoothedInaccuracy = stats.firstInaccuracyOverTime;

  const QUALITY_ORDER = ['book', 'best', 'excellent', 'good', 'inaccuracy', 'mistake', 'blunder'] as const;
  const moveQualityData = QUALITY_ORDER.map((key) => ({
    name: CLASSIFICATION_LABELS[key] || key,
    value: stats.moveQualityBreakdown[key] || 0,
    color: CLASSIFICATION_COLORS[key] || '#94a3b8',
  }));

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Dashboard</h1>
        <span className="text-sm text-slate-400">
          {stats.totalAnalyzedGames} analyzed games
        </span>
      </div>

      <div className="space-y-6">
        {/* Accuracy Over Time */}
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
          <h2 className="text-lg font-semibold text-white mb-4">Accuracy Over Time</h2>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={smoothedAccuracy}>
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                <XAxis
                  dataKey="date"
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <YAxis
                  domain={[50, 100]}
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <Tooltip content={<CustomTooltip />} />
                <Line
                  type="monotone"
                  dataKey="accuracy"
                  name="Accuracy"
                  stroke="#10b981"
                  strokeWidth={2}
                  dot={false}
                  isAnimationActive={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Accuracy by Phase */}
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
          <h2 className="text-lg font-semibold text-white mb-1">Accuracy by Phase</h2>
          <p className="text-sm text-slate-500 mb-4">Opening (moves 1-10) / Middlegame (11-25) / Endgame (26+)</p>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={smoothedPhase}>
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                <XAxis
                  dataKey="date"
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <YAxis
                  domain={[50, 100]}
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <Tooltip content={<CustomTooltip />} />
                <Line type="monotone" dataKey="opening" name="Opening" stroke="#3b82f6" strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
                <Line type="monotone" dataKey="middlegame" name="Middlegame" stroke="#f59e0b" strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
                <Line type="monotone" dataKey="endgame" name="Endgame" stroke="#ef4444" strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
          <div className="flex gap-4 mt-3 justify-center text-xs text-slate-400">
            <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-blue-500 inline-block rounded" /> Opening</span>
            <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-amber-500 inline-block rounded" /> Middlegame</span>
            <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-red-500 inline-block rounded" /> Endgame</span>
          </div>
        </div>

        {/* Earliest Inaccuracy / Mistake / Blunder Over Time */}
        {smoothedInaccuracy.length > 0 && (
          <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
            <h2 className="text-lg font-semibold text-white mb-1">Earliest Mistake</h2>
            <p className="text-sm text-slate-500 mb-4">Move number of first inaccuracy / mistake / blunder (higher is better)</p>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={smoothedInaccuracy}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                  <XAxis
                    dataKey="date"
                    tick={{ fill: '#94a3b8', fontSize: 12 }}
                    tickLine={{ stroke: '#475569' }}
                    axisLine={{ stroke: '#475569' }}
                  />
                  <YAxis
                    domain={[0, 'auto']}
                    tick={{ fill: '#94a3b8', fontSize: 12 }}
                    tickLine={{ stroke: '#475569' }}
                    axisLine={{ stroke: '#475569' }}
                    label={{ value: 'Move #', angle: -90, position: 'insideLeft', fill: '#64748b', fontSize: 12 }}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Line
                    type="monotone"
                    dataKey="moveNumber"
                    name="Inaccuracy"
                    stroke="#f59e0b"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                  <Line
                    type="monotone"
                    dataKey="mistakeMoveNumber"
                    name="Mistake"
                    stroke="#f97316"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                  <Line
                    type="monotone"
                    dataKey="blunderMoveNumber"
                    name="Blunder"
                    stroke="#ef4444"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
            <div className="flex gap-4 mt-3 justify-center text-xs text-slate-400">
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-amber-500 inline-block rounded" /> Inaccuracy</span>
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-orange-500 inline-block rounded" /> Mistake</span>
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-red-500 inline-block rounded" /> Blunder</span>
            </div>
          </div>
        )}

        {/* Move Quality Breakdown */}
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
          <h2 className="text-lg font-semibold text-white mb-4">Move Quality Breakdown</h2>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={moveQualityData} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" horizontal={false} />
                <XAxis
                  type="number"
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <YAxis
                  type="category"
                  dataKey="name"
                  width={80}
                  tick={{ fill: '#94a3b8', fontSize: 12 }}
                  tickLine={{ stroke: '#475569' }}
                  axisLine={{ stroke: '#475569' }}
                />
                <Tooltip content={<CustomTooltip />} />
                <Bar dataKey="value" name="Moves" radius={[0, 4, 4, 0]} isAnimationActive={false}>
                  {moveQualityData.map((entry, index) => (
                    <Cell key={index} fill={entry.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Rating Over Time */}
        {stats.ratingOverTime.length > 0 && (
          <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
            <h2 className="text-lg font-semibold text-white mb-4">Rating Over Time</h2>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={stats.ratingOverTime}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                  <XAxis
                    dataKey="date"
                    tick={{ fill: '#94a3b8', fontSize: 12 }}
                    tickLine={{ stroke: '#475569' }}
                    axisLine={{ stroke: '#475569' }}
                  />
                  <YAxis
                    domain={['auto', 'auto']}
                    tick={{ fill: '#94a3b8', fontSize: 12 }}
                    tickLine={{ stroke: '#475569' }}
                    axisLine={{ stroke: '#475569' }}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Line
                    type="monotone"
                    dataKey="rating"
                    name="Rating"
                    stroke="#8b5cf6"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}

        {/* Most & Least Accurate Games */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <GameAccuracyList title="Most Accurate Games" games={stats.mostAccurateGames} accent="emerald" />
          <GameAccuracyList title="Least Accurate Games" games={stats.leastAccurateGames} accent="red" />
        </div>

        {/* Opening Blunders + Cleanest Lines */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {stats.openingBlunders && stats.openingBlunders.length > 0 && (
            <OpeningBlundersList blunders={stats.openingBlunders} />
          )}
          {stats.cleanestLines && stats.cleanestLines.length > 0 && (
            <CleanestLinesList lines={stats.cleanestLines} />
          )}
        </div>
      </div>
    </div>
  );
}

const RESULT_LABEL: Record<string, string> = { W: 'Won', L: 'Lost', D: 'Draw' };
const RESULT_COLOR: Record<string, string> = { W: 'text-emerald-400', L: 'text-red-400', D: 'text-slate-400' };

function GameAccuracyList({ title, games, accent }: { title: string; games: GameSummary[]; accent: 'emerald' | 'red' }) {
  const accentColor = accent === 'emerald' ? 'text-emerald-400' : 'text-red-400';

  return (
    <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
      <h2 className="text-lg font-semibold text-white mb-4">{title}</h2>
      <div className="space-y-2">
        {games.map((game, i) => (
          <Link
            key={game.gameId}
            to={`/games/${game.gameId}`}
            className="flex items-center justify-between px-3 py-2.5 rounded-lg hover:bg-slate-700/50 transition-colors group"
          >
            <div className="flex items-center gap-3 min-w-0">
              <span className="text-sm text-slate-500 w-5 text-right">{i + 1}.</span>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-white text-sm font-medium truncate">vs {game.opponent}</span>
                  {game.opponentRating && (
                    <span className="text-slate-500 text-xs">({game.opponentRating})</span>
                  )}
                </div>
                <div className="flex items-center gap-2 text-xs text-slate-500">
                  <span className={RESULT_COLOR[game.result] || 'text-slate-400'}>
                    {RESULT_LABEL[game.result] || game.result}
                  </span>
                  <span>{game.date}</span>
                </div>
              </div>
            </div>
            <span className={`text-sm font-semibold ${accentColor} whitespace-nowrap`}>
              {game.accuracy}%
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}

function OpeningBlundersList({ blunders }: { blunders: OpeningBlunder[] }) {
  const navigate = useNavigate();

  const openGame = (b: OpeningBlunder) => {
    navigate('/opening-line', { state: {
      type: 'blunder',
      moves: b.moves,
      color: b.color,
      line: b.line,
      ply: b.ply,
      avgCpLoss: b.avgCpLoss,
      bestMove: b.bestMove,
      mistakeCount: b.mistakeCount,
    }});
  };

  return (
    <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
      <h2 className="text-lg font-semibold text-white mb-1">Costliest Opening Habits</h2>
      <p className="text-sm text-slate-500 mb-4">Opening mistakes you keep repeating</p>
      <div className="space-y-2">
        {blunders.map((b, i) => {
          const lastSpace = b.line.lastIndexOf(' ');
          const prefix = lastSpace > 0 ? b.line.slice(0, lastSpace) : '';
          const blunderMove = lastSpace > 0 ? b.line.slice(lastSpace + 1) : b.line;

          return (
            <button
              key={`${b.ply}-${b.line}`}
              onClick={() => openGame(b)}
              className="w-full flex items-center justify-between px-3 py-2.5 rounded-lg hover:bg-slate-700/50 transition-colors text-left cursor-pointer"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-sm text-slate-500 w-5 text-right shrink-0">{i + 1}.</span>
                <div className="min-w-0">
                  <div className="text-sm leading-relaxed">
                    {prefix && <span className="text-slate-400">{prefix} </span>}
                    <span className="text-orange-400 font-semibold">{blunderMove}</span>
                  </div>
                  <div className="text-xs text-slate-500">
                    Repeated {b.mistakeCount} times as {b.color}
                  </div>
                </div>
              </div>
              <span className="text-sm font-semibold text-orange-400 whitespace-nowrap ml-3">
                -{b.avgCpLoss} cp
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function CleanestLinesList({ lines }: { lines: CleanLine[] }) {
  const navigate = useNavigate();

  const openGame = (c: CleanLine) => {
    navigate('/opening-line', { state: {
      type: 'clean',
      moves: c.moves,
      color: c.color,
      line: c.line,
      avgCpLoss: c.avgCpLoss,
      cleanDepth: c.cleanDepth,
      gameCount: c.gameCount,
    }});
  };

  return (
    <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
      <h2 className="text-lg font-semibold text-white mb-1">Deepest Opening Prep</h2>
      <p className="text-sm text-slate-500 mb-4">Your longest lines with no inaccuracies</p>
      <div className="space-y-2">
        {lines.map((c, i) => (
          <button
            key={`${c.cleanDepth}-${c.line}`}
            onClick={() => openGame(c)}
            className="w-full flex items-center justify-between px-3 py-2.5 rounded-lg hover:bg-slate-700/50 transition-colors text-left cursor-pointer"
          >
            <div className="flex items-center gap-3 min-w-0">
              <span className="text-sm text-slate-500 w-5 text-right shrink-0">{i + 1}.</span>
              <div className="min-w-0">
                <div className="text-sm leading-relaxed text-emerald-400 font-medium truncate">
                  {c.line}
                </div>
                <div className="text-xs text-slate-500">
                  {c.cleanDepth} moves deep as {c.color} &middot; {c.gameCount} {c.gameCount === 1 ? 'game' : 'games'}
                </div>
              </div>
            </div>
            <span className="text-sm font-semibold text-emerald-400 whitespace-nowrap ml-3">
              ~{c.avgCpLoss} cp
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
