import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { Chessboard } from 'react-chessboard';
import { PuzzleBoard, type PuzzleStatus } from '../components/chess';
import { useAuthStore } from '../stores/authStore';
import { API_BASE_URL } from '../config/api';
import { tagDisplayName, isVisibleTag } from '../utils/tagDisplay';
import { getPuzzleStats, type PuzzleStats, type PositionStats, type ThemeStats } from '../services/puzzleStatsService';
import type { PuzzleWithContext } from '../types/analysis';

const API_BASE = API_BASE_URL;

interface PuzzlesResponse {
  puzzles: PuzzleWithContext[];
  total: number;
  themes: Record<string, number>;
}

const PUZZLES_PER_PAGE = 9;

export default function PuzzlesPage() {
  const { token } = useAuthStore();

  const [puzzles, setPuzzles] = useState<PuzzleWithContext[]>([]);
  const [themes, setThemes] = useState<Record<string, number>>({});
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTheme, setSelectedTheme] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [stats, setStats] = useState<PuzzleStats | null>(null);

  // Solve mode
  const [activePuzzle, setActivePuzzle] = useState<PuzzleWithContext | null>(null);
  const [puzzleStatus, setPuzzleStatus] = useState<PuzzleStatus>('solving');
  const [showSolution, setShowSolution] = useState(false);
  const [retryKey, setRetryKey] = useState(0);

  const loadPuzzles = async (theme?: string | null) => {
    setLoading(true);
    setError(null);

    let url = `${API_BASE}/api/puzzles`;
    if (theme) {
      url += `?theme=${encodeURIComponent(theme)}`;
    }

    try {
      const response = await fetch(url, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!response.ok) throw new Error(`Failed to load puzzles: ${response.statusText}`);
      const data: PuzzlesResponse = await response.json();
      setPuzzles(data.puzzles);
      setThemes(data.themes);
      setTotal(data.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load puzzles');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPuzzles(selectedTheme);
  }, [selectedTheme]);

  useEffect(() => {
    getPuzzleStats().then(setStats).catch(() => {});
  }, []);

  const selectTheme = (theme: string) => {
    setPage(1);
    if (selectedTheme === theme) {
      setSelectedTheme(null);
    } else {
      setSelectedTheme(theme);
    }
  };

  const openPuzzle = (puzzle: PuzzleWithContext) => {
    setActivePuzzle(puzzle);
    setPuzzleStatus('solving');
    setShowSolution(false);
    setRetryKey(0);
  };

  const retryPuzzle = () => {
    setPuzzleStatus('solving');
    setShowSolution(false);
    setRetryKey(k => k + 1);
  };

  const closePuzzle = () => {
    setActivePuzzle(null);
    setPuzzleStatus('solving');
    setShowSolution(false);
  };

  const nextPuzzle = () => {
    if (!activePuzzle) return;
    const idx = puzzles.findIndex(p => p.id === activePuzzle.id);
    if (idx >= 0 && idx < puzzles.length - 1) {
      openPuzzle(puzzles[idx + 1]);
    }
  };

  const sortedThemes = Object.entries(themes)
    .filter(([theme]) => isVisibleTag(theme))
    .sort((a, b) => b[1] - a[1])
    .map(([theme]) => theme);

  // Pagination
  const totalPages = Math.max(1, Math.ceil(puzzles.length / PUZZLES_PER_PAGE));
  const paginatedPuzzles = puzzles.slice(
    (page - 1) * PUZZLES_PER_PAGE,
    page * PUZZLES_PER_PAGE,
  );

  // --- Solve mode ---
  if (activePuzzle) {
    const currentIdx = puzzles.findIndex(p => p.id === activePuzzle.id);
    const hasNext = currentIdx >= 0 && currentIdx < puzzles.length - 1;
    // Solver is opposite of side to move in FEN (FEN is before opponent's blunder)
    const solverColor = activePuzzle.fen.split(' ')[1] === 'w' ? 'Black' : 'White';

    return (
      <div className="space-y-6">
        {/* Back button */}
        <button
          onClick={closePuzzle}
          className="flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Puzzles
        </button>

        {/* Two-column layout */}
        <div className="flex flex-col lg:flex-row gap-6">
          {/* Left: Board */}
          <div className="flex-1 max-w-[560px]">
            <PuzzleBoard
              fen={activePuzzle.fen}
              solutionMoves={activePuzzle.moves}
              onStatusChange={setPuzzleStatus}
              showSolution={showSolution}
              retryKey={retryKey}
            />
          </div>

          {/* Right: Info panel */}
          <div className="lg:w-80 space-y-4">
            {/* Theme badges */}
            <div className="card p-4">
              <h3 className="text-sm font-medium text-slate-400 mb-2">Themes</h3>
              <div className="flex flex-wrap gap-2">
                {activePuzzle.themes.filter(isVisibleTag).map(theme => (
                  <span
                    key={theme}
                    className="px-2.5 py-1 bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30 rounded-full text-xs font-medium text-amber-400"
                  >
                    {tagDisplayName(theme)}
                  </span>
                ))}
              </div>
            </div>

            {/* Source game */}
            <div className="card p-4">
              <h3 className="text-sm font-medium text-slate-400 mb-2">Source Game</h3>
              <Link
                to={`/games/${activePuzzle.gameId}`}
                className="text-emerald-400 hover:text-emerald-300 text-sm transition-colors"
              >
                vs {activePuzzle.opponent}
                {activePuzzle.date && <span className="text-slate-500 ml-2">{activePuzzle.date}</span>}
              </Link>
              <div className="flex items-center gap-2 mt-1">
                <span className={`w-4 h-4 rounded flex items-center justify-center text-[9px] font-bold ${
                  activePuzzle.source === 'chess_com'
                    ? 'bg-green-600 text-white'
                    : 'bg-white text-black'
                }`}>
                  {activePuzzle.source === 'chess_com' ? 'C' : 'L'}
                </span>
                <span className="text-slate-500 text-xs capitalize">as {activePuzzle.userColor}</span>
              </div>
            </div>

            {/* Puzzle info */}
            <div className="card p-4">
              <h3 className="text-sm font-medium text-slate-400 mb-2">Puzzle Info</h3>
              <p className="text-slate-300 text-sm">
                {solverColor} to move — {Math.floor(activePuzzle.moves.length / 2)} move
                {Math.floor(activePuzzle.moves.length / 2) !== 1 ? 's' : ''} to find
              </p>
            </div>

            {/* Action buttons */}
            <div className="flex flex-col gap-2">
              {puzzleStatus === 'solving' && (
                <button
                  onClick={() => setShowSolution(true)}
                  className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors text-sm"
                >
                  Show Solution
                </button>
              )}
              {(puzzleStatus === 'failed' || puzzleStatus === 'solved') && (
                <button
                  onClick={retryPuzzle}
                  className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors text-sm"
                >
                  Retry Puzzle
                </button>
              )}
              {puzzleStatus === 'failed' && !showSolution && (
                <button
                  onClick={() => setShowSolution(true)}
                  className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors text-sm"
                >
                  Show Solution
                </button>
              )}
              {hasNext && (
                <button
                  onClick={nextPuzzle}
                  className="px-4 py-2 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 text-sm shadow-[0_0_12px_rgba(16,185,129,0.3)]"
                >
                  Next Puzzle
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // --- List mode ---
  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      {/* Puzzle Performance Stats */}
      {stats && stats.user.total + stats.opponent.total > 0 && (() => {
        const edge = stats.user.rate - stats.opponent.rate;
        const CIRC = 2 * Math.PI * 50; // circumference for r=50
        const userOffset = CIRC * (1 - stats.user.rate / 100);
        const oppOffset = CIRC * (1 - stats.opponent.rate / 100);
        return (
        <div className="card p-6 relative overflow-hidden">
          {/* Header with gradient underline */}
          <div className="mb-6">
            <h2 className="text-base font-semibold text-white tracking-tight">Tactical Performance</h2>
            <div className="mt-2 h-px bg-gradient-to-r from-emerald-500/60 via-teal-500/30 to-transparent" />
          </div>

          {/* Summary: Gauges + Edge */}
          <div className="grid grid-cols-[1fr_1fr_1.1fr] gap-5">
            {/* Your Find Rate gauge */}
            <div className="flex flex-col items-center text-center">
              <div className="relative w-28 h-28 mb-3">
                <svg className="gauge-ring w-full h-full" viewBox="0 0 120 120">
                  <circle className="gauge-track" cx="60" cy="60" r="50" fill="none" strokeWidth="8" />
                  <circle className="gauge-fill" cx="60" cy="60" r="50" fill="none"
                    stroke="url(#emeraldGrad)" strokeWidth="8"
                    strokeDasharray={CIRC} strokeDashoffset={userOffset} />
                  <defs>
                    <linearGradient id="emeraldGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                      <stop offset="0%" stopColor="#34d399" />
                      <stop offset="100%" stopColor="#14b8a6" />
                    </linearGradient>
                  </defs>
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-2xl font-bold text-emerald-400">{stats.user.rate}<span className="text-lg">%</span></span>
                </div>
              </div>
              <p className="text-xs font-medium text-slate-400 mb-0.5">Your Find Rate</p>
              <p className="text-[11px] text-slate-400">{stats.user.found} of {stats.user.total} found</p>
            </div>

            {/* Opponent Find Rate gauge */}
            <div className="flex flex-col items-center text-center">
              <div className="relative w-28 h-28 mb-3">
                <svg className="gauge-ring w-full h-full" viewBox="0 0 120 120">
                  <circle className="gauge-track" cx="60" cy="60" r="50" fill="none" strokeWidth="8" />
                  <circle className="gauge-fill" cx="60" cy="60" r="50" fill="none"
                    stroke="url(#slateGrad)" strokeWidth="8"
                    strokeDasharray={CIRC} strokeDashoffset={oppOffset} />
                  <defs>
                    <linearGradient id="slateGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                      <stop offset="0%" stopColor="#94a3b8" />
                      <stop offset="100%" stopColor="#64748b" />
                    </linearGradient>
                  </defs>
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-2xl font-bold text-slate-300">{stats.opponent.rate}<span className="text-lg">%</span></span>
                </div>
              </div>
              <p className="text-xs font-medium text-slate-400 mb-0.5">Opponent Find Rate</p>
              <p className="text-[11px] text-slate-400">{stats.opponent.found} of {stats.opponent.total} found</p>
            </div>

            {/* Tactical Edge hero */}
            <div className={`flex flex-col items-center justify-center text-center rounded-lg border px-4 py-5 ${
              edge > 5
                ? 'bg-gradient-to-br from-emerald-950/40 via-slate-900/20 to-teal-950/30 border-emerald-500/10'
                : edge < -5
                ? 'bg-gradient-to-br from-red-950/40 via-slate-900/20 to-red-950/30 border-red-500/10'
                : 'bg-gradient-to-br from-slate-800/40 via-slate-900/20 to-slate-800/30 border-slate-600/10'
            }`}>
              <p className={`text-[10px] uppercase tracking-[0.15em] font-medium mb-2 ${
                edge > 5 ? 'text-emerald-500/70' : edge < -5 ? 'text-red-500/70' : 'text-slate-500/70'
              }`}>Tactical Edge</p>
              <span className={`text-4xl font-bold font-mono leading-none ${
                edge > 5 ? 'gradient-text' : edge < -5 ? 'text-red-400' : 'text-slate-400'
              }`}>
                {edge > 0 ? '+' : ''}{edge.toFixed(0)}%
              </span>
              <p className={`text-[11px] mt-2 ${
                edge > 5 ? 'text-emerald-500/50' : edge < -5 ? 'text-red-500/50' : 'text-slate-500/50'
              }`}>
                {edge > 5 ? 'You outperform opponents' : edge < -5 ? 'Opponents outperform you' : 'Evenly matched'}
              </p>
              <div className="flex items-center gap-3 mt-3 text-[10px]">
                <span className={`flex items-center gap-1 ${edge < -5 ? 'text-red-400/70' : 'text-emerald-400/70'}`}>
                  <span className={`w-1 h-1 rounded-full ${edge < -5 ? 'bg-red-400' : 'bg-emerald-400'}`} />
                  {stats.user.rate}%
                </span>
                <span className="text-slate-700">vs</span>
                <span className="flex items-center gap-1 text-slate-500">
                  <span className="w-1 h-1 rounded-full bg-slate-500" />
                  {stats.opponent.rate}%
                </span>
              </div>
            </div>
          </div>

          {/* Position type breakdown */}
          {stats.byPosition && stats.byPosition.length > 0 && (
            <PositionBreakdown positions={stats.byPosition} />
          )}

          {/* Theme breakdown */}
          {stats.byTheme && stats.byTheme.length > 0 && (
            <ThemeBreakdown themes={stats.byTheme} />
          )}
        </div>
        );
      })()}

      {loading && puzzles.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <span className="w-6 h-6 border-2 border-emerald-500/30 border-t-emerald-500 rounded-full animate-spin" />
        </div>
      )}

      {error && (
        <div className="card p-4 text-red-400 text-sm">{error}</div>
      )}

      {!loading && !error && total === 0 && (
        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg className="w-8 h-8 text-slate-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M12 2C9.5 2 8 3.5 8 5.5c0 1.5.5 2 1 2.5L8 10h8l-1-2c.5-.5 1-1 1-2.5C16 3.5 14.5 2 12 2z" />
              <rect x="7" y="10" width="10" height="2" rx="0.5" />
              <path d="M8 12v7a3 3 0 003 3h2a3 3 0 003-3v-7" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">No puzzles yet</h2>
          <p className="text-slate-400 mb-6">
            Puzzles are automatically extracted when you analyze your games. Head to your games page and analyze some games to generate puzzles.
          </p>
          <Link
            to="/games"
            className="inline-block px-6 py-3 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 shadow-[0_0_12px_rgba(16,185,129,0.3)]"
          >
            Go to Games
          </Link>
        </div>
      )}

      {total > 0 && (
        <>
          {/* Stats */}
          <p className="text-slate-400 text-sm">
            {puzzles.length} {puzzles.length === 1 ? 'puzzle' : 'puzzles'}
            {selectedTheme && (
              <span className="text-emerald-400 ml-1">
                filtered by {tagDisplayName(selectedTheme)}
              </span>
            )}
            {totalPages > 1 && (
              <span className="text-slate-500 ml-1">
                — showing {(page - 1) * PUZZLES_PER_PAGE + 1}–{Math.min(page * PUZZLES_PER_PAGE, puzzles.length)}
              </span>
            )}
          </p>

          {/* Theme filter */}
          {sortedThemes.length > 0 && (
            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                {sortedThemes.map(theme => {
                  const isSelected = selectedTheme === theme;
                  return (
                    <button
                      key={theme}
                      onClick={() => selectTheme(theme)}
                      className={`px-3 py-1.5 rounded-full text-sm font-medium transition-colors ${
                        isSelected
                          ? 'bg-emerald-600 text-white'
                          : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
                      }`}
                    >
                      {tagDisplayName(theme)} ({themes[theme]})
                    </button>
                  );
                })}
              </div>
              {selectedTheme && (
                <button
                  onClick={() => setSelectedTheme(null)}
                  className="text-sm text-slate-400 hover:text-white transition-colors"
                >
                  Clear filter
                </button>
              )}
            </div>
          )}

          {/* Puzzle grid */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {paginatedPuzzles.map(puzzle => (
              <button
                key={puzzle.id}
                onClick={() => openPuzzle(puzzle)}
                className="card p-4 text-left hover:border-emerald-500/60 transition-all duration-200 group"
              >
                {/* Mini board showing puzzle position */}
                <div className="mb-3">
                  <PuzzleMiniBoard
                    fen={puzzle.fen}
                    orientation={puzzle.fen.split(' ')[1] === 'w' ? 'black' : 'white'}
                  />
                </div>

                {/* Info */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-white group-hover:text-emerald-400 transition-colors font-medium">
                      vs {puzzle.opponent}
                    </span>
                    <span className={`w-4 h-4 rounded flex items-center justify-center text-[9px] font-bold ${
                      puzzle.source === 'chess_com'
                        ? 'bg-green-600 text-white'
                        : 'bg-white text-black'
                    }`}>
                      {puzzle.source === 'chess_com' ? 'C' : 'L'}
                    </span>
                  </div>

                  {puzzle.date && (
                    <p className="text-xs text-slate-500">{puzzle.date}</p>
                  )}

                  {/* Theme badges */}
                  <div className="flex flex-wrap gap-1.5">
                    {puzzle.themes.filter(isVisibleTag).slice(0, 3).map(theme => (
                      <span
                        key={theme}
                        className="px-2 py-0.5 bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30 rounded-full text-[10px] font-medium text-amber-400"
                      >
                        {tagDisplayName(theme)}
                      </span>
                    ))}
                    {puzzle.themes.filter(isVisibleTag).length > 3 && (
                      <span className="px-2 py-0.5 text-[10px] text-slate-500">
                        +{puzzle.themes.filter(isVisibleTag).length - 3}
                      </span>
                    )}
                  </div>
                </div>
              </button>
            ))}
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2 pt-2">
              <button
                onClick={() => setPage(1)}
                disabled={page === 1}
                className="px-3 py-1.5 rounded-lg text-sm font-medium bg-slate-800 text-slate-300 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                First
              </button>
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="px-3 py-1.5 rounded-lg text-sm font-medium bg-slate-800 text-slate-300 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Previous
              </button>
              <span className="text-sm text-slate-400 px-3">
                Page {page} of {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page === totalPages}
                className="px-3 py-1.5 rounded-lg text-sm font-medium bg-slate-800 text-slate-300 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Next
              </button>
              <button
                onClick={() => setPage(totalPages)}
                disabled={page === totalPages}
                className="px-3 py-1.5 rounded-lg text-sm font-medium bg-slate-800 text-slate-300 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Last
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

/** Position type breakdown - stacked bars with You/Opp labels */
function PositionBreakdown({ positions }: { positions: PositionStats[] }) {
  if (positions.length === 0) return null;

  const getEdgeColor = (edge: number) =>
    edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-600';

  const getPositionLabel = (pos: string) => {
    switch (pos) {
      case 'winning': return { label: 'Winning', color: 'text-emerald-400' };
      case 'equal': return { label: 'Equal', color: 'text-slate-300' };
      case 'losing': return { label: 'Losing', color: 'text-red-400' };
      default: return { label: pos, color: 'text-white' };
    }
  };

  return (
    <div className="mt-6 pt-5 border-t border-slate-800/80">
      <h3 className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-4">By Position</h3>
      <div className="space-y-4">
        {positions.map((p) => {
          const { label, color } = getPositionLabel(p.position);
          const edge = p.user.rate - p.opponent.rate;
          const losing = edge < 0;
          return (
            <div key={p.position}>
              <div className="flex items-center justify-between mb-1.5">
                <div className="flex items-baseline gap-2">
                  <span className={`text-sm font-medium ${color}`}>{label}</span>
                  <span className="text-[10px] text-slate-400">{p.user.total} puzzles</span>
                </div>
                <span className={`text-[11px] font-semibold ${getEdgeColor(edge)}`}>
                  {edge > 0 ? '+' : ''}{edge.toFixed(0)}
                </span>
              </div>
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <span className="w-7 text-[9px] text-slate-500 font-medium shrink-0">You</span>
                  <div className="flex-1 h-[18px] bg-slate-900/80 rounded-[4px] relative overflow-hidden">
                    <div className={`bar-fill absolute inset-y-0 left-0 rounded-[4px] bg-gradient-to-r ${losing ? 'from-red-400/60 to-red-500/40' : 'from-emerald-400/60 to-teal-500/40'}`} style={{ width: `${p.user.rate}%` }} />
                    <span className={`absolute inset-y-0 right-2 flex items-center text-[10px] font-medium ${losing ? 'text-red-400' : 'text-emerald-400'}`}>{p.user.rate}%</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="w-7 text-[9px] text-slate-500 font-medium shrink-0">Opp</span>
                  <div className="flex-1 h-[18px] bg-slate-900/80 rounded-[4px] relative overflow-hidden">
                    <div className="bar-fill absolute inset-y-0 left-0 rounded-[4px] bg-gradient-to-r from-slate-500/50 to-slate-400/35" style={{ width: `${p.opponent.rate}%` }} />
                    <span className="absolute inset-y-0 right-2 flex items-center text-[10px] font-medium text-slate-300">{p.opponent.rate}%</span>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

const MATE_THEMES = new Set([
  'backRankMate', 'smotheredMate', 'anastasiaMate', 'arabianMate',
  'bodenMate', 'dovetailMate', 'doubleBishopMate', 'balestraMate',
  'blindSwineMate', 'cornerMate', 'hookMate', 'killBoxMate',
  'morphysMate', 'operaMate', 'pillsburysMate', 'triangleMate',
  'vukovicMate', 'doubleCheckmate',
  'mateIn1', 'mateIn2', 'mateIn3', 'mateIn4', 'mateIn5',
]);

/** Theme table - proper table layout matching endgame analytics */
function ThemeTable({ title, items }: { title: string; items: ThemeStats[] }) {
  return (
    <div className="mt-6 pt-5 border-t border-slate-800/80">
      <h3 className="text-sm font-semibold text-white mb-3">{title}</h3>
      <table className="w-full text-sm">
        <thead>
          <tr className="text-xs text-slate-400 uppercase tracking-wider">
            <th className="text-left py-2 pr-4 font-medium">Theme</th>
            <th className="text-right py-2 px-3 font-medium">Games</th>
            <th className="text-right py-2 px-3 font-medium">You</th>
            <th className="text-right py-2 px-3 font-medium">Opponent</th>
            <th className="text-right py-2 pl-3 font-medium">Edge</th>
          </tr>
        </thead>
        <tbody>
          {items.map((t, i) => {
            const edge = t.user.rate - t.opponent.rate;
            const losing = edge < 0;
            const edgeColor = edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-400';
            return (
              <tr
                key={t.theme}
                className={`border-t border-slate-800/50 ${i % 2 === 1 ? 'bg-slate-800/20' : ''}`}
              >
                <td className="py-2.5 pr-4 text-white font-medium">{tagDisplayName(t.theme)}</td>
                <td className="py-2.5 px-3 text-right text-slate-300">{t.user.total}</td>
                <td className={`py-2.5 px-3 text-right ${losing ? 'text-red-400' : 'text-emerald-400'}`}>
                  {t.user.rate}%
                </td>
                <td className="py-2.5 px-3 text-right text-slate-300">
                  {t.opponent.rate}%
                </td>
                <td className={`py-2.5 pl-3 text-right font-semibold ${edgeColor}`}>
                  {edge > 0 ? '+' : ''}{edge.toFixed(0)}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

/** Theme + Mate breakdown - proper tables sorted alphabetically */
function ThemeBreakdown({ themes }: { themes: ThemeStats[] }) {
  const eligible = themes.filter(t => isVisibleTag(t.theme) && t.user.total >= 50);
  const tactics = eligible
    .filter(t => !MATE_THEMES.has(t.theme))
    .sort((a, b) => tagDisplayName(a.theme).localeCompare(tagDisplayName(b.theme)));
  const mates = eligible
    .filter(t => MATE_THEMES.has(t.theme))
    .sort((a, b) => tagDisplayName(a.theme).localeCompare(tagDisplayName(b.theme)));

  if (tactics.length === 0 && mates.length === 0) return null;

  return (
    <>
      {tactics.length > 0 && <ThemeTable title="By Tactic" items={tactics} />}
      {mates.length > 0 && <ThemeTable title="By Checkmate Pattern" items={mates} />}
    </>
  );
}

/** Small non-interactive board for puzzle cards */
function PuzzleMiniBoard({ fen, orientation }: { fen: string; orientation: 'white' | 'black' }) {
  return (
    <div className="aspect-square w-full">
      <Chessboard
        options={{
          position: fen,
          boardOrientation: orientation,
          allowDragging: false,
        }}
      />
    </div>
  );
}
