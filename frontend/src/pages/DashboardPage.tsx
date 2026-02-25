import { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';
import { useAuthStore } from '../stores/authStore';
import { getStats, type DashboardStats, type GameSummary, type OpeningBlunder, type CleanLine } from '../services/dashboardService';

const QUALITY_ORDER = ['book', 'best', 'excellent', 'good', 'inaccuracy', 'mistake', 'blunder'] as const;

const QUALITY_COLORS: Record<string, string> = {
  book: '#06b6d4',
  best: '#10b981',
  excellent: 'rgba(52, 211, 153, 0.8)',
  good: 'rgba(110, 231, 183, 0.6)',
  inaccuracy: 'rgba(251, 191, 36, 0.8)',
  mistake: 'rgba(249, 115, 22, 0.8)',
  blunder: 'rgba(239, 68, 68, 0.8)',
};

const QUALITY_LABEL: Record<string, string> = {
  book: 'Book',
  best: 'Best',
  excellent: 'Excellent',
  good: 'Good',
  inaccuracy: 'Inaccuracy',
  mistake: 'Mistake',
  blunder: 'Blunder',
};

const MIN_GAMES = 100;
const GAUGE_R = 50;
const GAUGE_C = 2 * Math.PI * GAUGE_R;

const RESULT_LABEL: Record<string, string> = { W: 'Won', L: 'Lost', D: 'Draw' };
const RESULT_COLOR: Record<string, string> = { W: 'text-emerald-400', L: 'text-red-400', D: 'text-slate-400' };

function CustomTooltip({ active, payload, label }: { active?: boolean; payload?: Array<{ value: number; name: string }>; label?: string }) {
  if (!active || !payload?.length) return null;
  return (
    <div className="bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 shadow-xl">
      <p className="text-slate-400 text-[11px] mb-1">{label}</p>
      {payload.map((entry, i) => (
        <p key={i} className="text-white font-mono text-xs">
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
      <div className="max-w-5xl mx-auto p-6">
        <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
          Failed to load dashboard: {error}
        </div>
      </div>
    );
  }

  if (!hasLinkedAccount) {
    return (
      <div className="max-w-5xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Dashboard</h1>
        <div className="card p-8 text-center">
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
      <div className="max-w-5xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-white mb-6">Dashboard</h1>
        <div className="card p-8 text-center">
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

  // Derived data
  const avgAccuracy = Math.round(
    stats.accuracyOverTime.reduce((s, d) => s + d.accuracy, 0) / stats.accuracyOverTime.length
  );
  const gaugeOffset = GAUGE_C * (1 - avgAccuracy / 100);

  const latestRating = stats.ratingOverTime.length > 0
    ? stats.ratingOverTime[stats.ratingOverTime.length - 1].rating
    : null;
  // Move quality percentages
  const mqTotal = QUALITY_ORDER.reduce((s, k) => s + (stats.moveQualityBreakdown[k] || 0), 0);
  const mqSegments = QUALITY_ORDER.map((key) => {
    const raw = stats.moveQualityBreakdown[key] || 0;
    const pct = mqTotal > 0 ? (raw / mqTotal) * 100 : 0;
    return { key, pct };
  }).filter(s => s.pct > 0);

  return (
    <div className="max-w-5xl mx-auto p-4 sm:p-6">
      {/* Hero Metrics Row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        {/* Overall Accuracy — gauge */}
        <div className="card p-5">
          <div className="flex items-center gap-4">
            <div className="relative w-20 h-20 shrink-0">
              <svg className="w-full h-full" viewBox="0 0 120 120" style={{ transform: 'rotate(-90deg)' }}>
                <circle cx="60" cy="60" r={GAUGE_R} fill="none" strokeWidth="7" stroke="rgba(51,65,85,0.5)" />
                <circle cx="60" cy="60" r={GAUGE_R} fill="none" strokeWidth="7"
                  stroke="#34d399" strokeLinecap="round"
                  strokeDasharray={GAUGE_C} strokeDashoffset={gaugeOffset}
                />
              </svg>
              <div className="absolute inset-0 flex items-center justify-center">
                <span className="text-xl font-bold text-emerald-400 font-mono">
                  {avgAccuracy}<span className="text-sm">%</span>
                </span>
              </div>
            </div>
            <div>
              <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500 font-medium mb-1">Accuracy</p>
              <p className="text-[11px] text-slate-600">Across all games</p>
            </div>
          </div>
        </div>

        {/* Games Analyzed */}
        <div className="card p-5 flex flex-col items-center justify-center text-center">
          <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500 font-medium mb-2">Games Analyzed</p>
          <p className="text-3xl font-bold text-white font-mono leading-none">{stats.totalAnalyzedGames}</p>
        </div>

        {/* Rating */}
        <div className="card p-5 flex flex-col items-center justify-center text-center">
          <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500 font-medium mb-2">Rating</p>
          {latestRating ? (
            <p className="text-3xl font-bold text-white font-mono leading-none">{latestRating}</p>
          ) : (
            <p className="text-sm text-slate-600">No rating data</p>
          )}
        </div>

        {/* Win Rate */}
        <div className="card p-5 flex flex-col items-center justify-center text-center">
          <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500 font-medium mb-2">Win Rate</p>
          <p className="text-3xl font-bold text-emerald-400 font-mono leading-none">{Math.round(stats.winRate)}%</p>
        </div>
      </div>

      {/* Move Quality Breakdown */}
      <div className="card p-5 mb-4">
        <h2 className="text-sm font-semibold text-white mb-4">Move Quality Breakdown</h2>
        <div className="flex items-center gap-1 h-8 rounded-lg overflow-hidden">
          {mqSegments.map((seg, i) => {
            const rounded = i === 0 ? 'rounded-l-md' : i === mqSegments.length - 1 ? 'rounded-r-md' : '';
            return (
              <div
                key={seg.key}
                className={`h-full relative cursor-default ${rounded}`}
                style={{ width: `${seg.pct}%`, backgroundColor: QUALITY_COLORS[seg.key] }}
              />
            );
          })}
        </div>
        <div className="flex gap-5 mt-3 text-xs text-slate-400 justify-center flex-wrap">
          {QUALITY_ORDER.map(key => {
            const raw = stats.moveQualityBreakdown[key] || 0;
            const pct = mqTotal > 0 ? Math.round((raw / mqTotal) * 100) : 0;
            return (
              <span key={key} className="flex items-center gap-1.5">
                <span className="w-2.5 h-2.5 rounded-sm" style={{ backgroundColor: QUALITY_COLORS[key] }} />
                {QUALITY_LABEL[key]} {pct}%
              </span>
            );
          })}
        </div>
      </div>

      {/* Charts — 2-column grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
        {/* Accuracy Over Time */}
        <div className="card p-5">
          <div className="flex items-baseline justify-between mb-4">
            <h2 className="text-sm font-semibold text-white">Accuracy Over Time</h2>
            <span className="text-[10px] text-slate-600 font-mono">Last {stats.accuracyOverTime.length} games</span>
          </div>
          <div className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={stats.accuracyOverTime}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(51,65,85,0.25)" />
                <XAxis dataKey="date" tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} interval="preserveStartEnd" />
                <YAxis domain={[50, 100]} tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} />
                <Tooltip content={<CustomTooltip />} />
                <Line type="monotone" dataKey="accuracy" name="Accuracy" stroke="#34d399" strokeWidth={2} dot={false} activeDot={{ r: 3 }} isAnimationActive={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Accuracy by Phase */}
        <div className="card p-5">
          <div className="flex items-baseline justify-between mb-1">
            <h2 className="text-sm font-semibold text-white">Accuracy by Phase</h2>
          </div>
          <div className="flex gap-4 mb-3 text-[10px] text-slate-500">
            <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-blue-400 inline-block rounded" />Opening</span>
            <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-amber-400 inline-block rounded" />Middlegame</span>
            <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-red-400 inline-block rounded" />Endgame</span>
          </div>
          <div className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={stats.phaseAccuracyOverTime}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(51,65,85,0.25)" />
                <XAxis dataKey="date" tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} interval="preserveStartEnd" />
                <YAxis domain={[50, 100]} tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} />
                <Tooltip content={<CustomTooltip />} />
                <Line type="monotone" dataKey="opening" name="Opening" stroke="#60a5fa" strokeWidth={1.5} dot={false} connectNulls activeDot={{ r: 3 }} isAnimationActive={false} />
                <Line type="monotone" dataKey="middlegame" name="Middlegame" stroke="#fbbf24" strokeWidth={1.5} dot={false} connectNulls activeDot={{ r: 3 }} isAnimationActive={false} />
                <Line type="monotone" dataKey="endgame" name="Endgame" stroke="#f87171" strokeWidth={1.5} dot={false} connectNulls activeDot={{ r: 3 }} isAnimationActive={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
        {/* Earliest Mistake */}
        {stats.firstInaccuracyOverTime.length > 0 && (
          <div className="card p-5">
            <div className="flex items-baseline justify-between mb-1">
              <h2 className="text-sm font-semibold text-white">Earliest Mistake</h2>
            </div>
            <div className="flex gap-4 mb-3 text-[10px] text-slate-500">
              <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-amber-400 inline-block rounded" />Inaccuracy</span>
              <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-orange-400 inline-block rounded" />Mistake</span>
              <span className="flex items-center gap-1.5"><span className="w-2 h-[2px] bg-red-400 inline-block rounded" />Blunder</span>
            </div>
            <div className="h-44">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={stats.firstInaccuracyOverTime}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(51,65,85,0.25)" />
                  <XAxis dataKey="date" tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} interval="preserveStartEnd" />
                  <YAxis domain={[0, 'auto']} tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} />
                  <Tooltip content={<CustomTooltip />} />
                  <Line type="monotone" dataKey="moveNumber" name="Inaccuracy" stroke="#fbbf24" strokeWidth={1.5} dot={false} activeDot={{ r: 3 }} isAnimationActive={false} />
                  <Line type="monotone" dataKey="mistakeMoveNumber" name="Mistake" stroke="#fb923c" strokeWidth={1.5} dot={false} activeDot={{ r: 3 }} isAnimationActive={false} />
                  <Line type="monotone" dataKey="blunderMoveNumber" name="Blunder" stroke="#f87171" strokeWidth={1.5} dot={false} activeDot={{ r: 3 }} isAnimationActive={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}

        {/* Rating Over Time */}
        {stats.ratingOverTime.length > 0 && (
          <div className="card p-5">
            <div className="flex items-baseline justify-between mb-4">
              <h2 className="text-sm font-semibold text-white">Rating Over Time</h2>
            </div>
            <div className="h-48">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={stats.ratingOverTime}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(51,65,85,0.25)" />
                  <XAxis dataKey="date" tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} interval="preserveStartEnd" />
                  <YAxis domain={['auto', 'auto']} tick={{ fill: '#475569', fontSize: 10 }} axisLine={false} tickLine={false} />
                  <Tooltip content={<CustomTooltip />} />
                  <Line type="monotone" dataKey="rating" name="Rating" stroke="#a78bfa" strokeWidth={2} dot={false} activeDot={{ r: 3 }} isAnimationActive={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}
      </div>

      {/* Game Lists — 2-column */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
        <GameAccuracyList title="Most Accurate Games" games={stats.mostAccurateGames} accent="emerald" />
        <GameAccuracyList title="Least Accurate Games" games={stats.leastAccurateGames} accent="red" />
      </div>

      {/* Opening Habits — 2-column (Deepest Prep left, Costliest Habits right) */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {stats.cleanestLines && stats.cleanestLines.length > 0 && (
          <CleanestLinesList lines={stats.cleanestLines} />
        )}
        {stats.openingBlunders && stats.openingBlunders.length > 0 && (
          <OpeningBlundersList blunders={stats.openingBlunders} />
        )}
      </div>
    </div>
  );
}

function GameAccuracyList({ title, games, accent }: { title: string; games: GameSummary[]; accent: 'emerald' | 'red' }) {
  const accentColor = accent === 'emerald' ? 'text-emerald-400' : 'text-red-400';
  const dividerBg = accent === 'emerald' ? 'rgba(51,65,85,0.4)' : 'rgba(239,68,68,0.25)';

  return (
    <div className="card p-5">
      <h2 className="text-sm font-semibold text-white mb-3">{title}</h2>
      <div className="h-px mb-3" style={{ background: dividerBg }} />
      <div className="space-y-0">
        {games.map((game, i) => (
          <Link
            key={game.gameId}
            to={`/games/${game.gameId}`}
            className="flex items-center justify-between px-2 py-2.5 rounded-lg hover:bg-slate-700/25 transition-colors"
          >
            <div className="flex items-center gap-2.5">
              <span className="text-[11px] text-slate-600 font-mono w-4 text-right">{i + 1}</span>
              <div>
                <div className="flex items-center gap-1.5">
                  <span className="text-[13px] text-white font-medium">vs {game.opponent}</span>
                  {game.opponentRating && (
                    <span className="text-[10px] text-slate-600 font-mono">({game.opponentRating})</span>
                  )}
                </div>
                <div className="flex items-center gap-2 text-[10px]">
                  <span className={RESULT_COLOR[game.result] || 'text-slate-400'}>
                    {RESULT_LABEL[game.result] || game.result}
                  </span>
                  <span className="text-slate-700">&middot;</span>
                  <span className="text-slate-600">{game.date}</span>
                </div>
              </div>
            </div>
            <span className={`text-sm font-bold ${accentColor} font-mono`}>{game.accuracy}%</span>
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
    <div className="card p-5">
      <h2 className="text-sm font-semibold text-white mb-1">Costliest Opening Habits</h2>
      <p className="text-[10px] text-slate-600 mb-3">Mistakes you keep repeating</p>
      <div className="h-px mb-3" style={{ background: 'rgba(239,68,68,0.25)' }} />
      <div className="space-y-0">
        {blunders.map((b, i) => {
          const lastSpace = b.line.lastIndexOf(' ');
          const prefix = lastSpace > 0 ? b.line.slice(0, lastSpace) : '';
          const blunderMove = lastSpace > 0 ? b.line.slice(lastSpace + 1) : b.line;

          return (
            <button
              key={`${b.ply}-${b.line}`}
              onClick={() => openGame(b)}
              className="w-full flex items-center justify-between px-2 py-2.5 rounded-lg hover:bg-slate-700/25 transition-colors text-left cursor-pointer"
            >
              <div className="flex items-center gap-2.5 min-w-0">
                <span className="text-[11px] text-slate-600 font-mono w-4 text-right shrink-0">{i + 1}</span>
                <div className="min-w-0">
                  <div className="text-[13px] leading-relaxed">
                    {prefix && <span className="text-slate-400">{prefix} </span>}
                    <span className="text-red-400 font-semibold">{blunderMove}</span>
                  </div>
                  <div className="text-[10px] text-slate-600">
                    Repeated {b.mistakeCount}&times; as {b.color}
                  </div>
                </div>
              </div>
              <span className="text-sm font-bold text-red-400 font-mono whitespace-nowrap ml-3">
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
    <div className="card p-5">
      <h2 className="text-sm font-semibold text-white mb-1">Deepest Opening Prep</h2>
      <p className="text-[10px] text-slate-600 mb-3">Longest lines with no inaccuracies</p>
      <div className="h-px mb-3" style={{ background: 'rgba(51,65,85,0.4)' }} />
      <div className="space-y-0">
        {lines.map((c, i) => (
          <button
            key={`${c.cleanDepth}-${c.line}`}
            onClick={() => openGame(c)}
            className="w-full flex items-center justify-between px-2 py-2.5 rounded-lg hover:bg-slate-700/25 transition-colors text-left cursor-pointer"
          >
            <div className="flex items-center gap-2.5 min-w-0">
              <span className="text-[11px] text-slate-600 font-mono w-4 text-right shrink-0">{i + 1}</span>
              <div className="min-w-0">
                <div className="text-[13px] leading-relaxed text-emerald-400 font-medium truncate">
                  {c.line}
                </div>
                <div className="text-[10px] text-slate-600">
                  {c.cleanDepth} moves deep as {c.color} &middot; {c.gameCount} {c.gameCount === 1 ? 'game' : 'games'}
                </div>
              </div>
            </div>
            <span className="text-sm font-bold text-emerald-400 font-mono whitespace-nowrap ml-3">
              {c.cleanDepth} moves
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
