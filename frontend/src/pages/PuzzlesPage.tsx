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
                {activePuzzle.themes.map(theme => (
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
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-white">Puzzles</h1>
        <p className="text-slate-400 text-sm mt-1">
          Practice tactics extracted from your analyzed games
        </p>
      </div>

      {/* Puzzle Performance Stats */}
      {stats && stats.user.total + stats.opponent.total > 0 && (
        <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5">
          <h2 className="text-lg font-semibold text-white mb-4">Tactical Performance</h2>
          <div className="grid grid-cols-2 gap-6">
            {/* Your tactics */}
            <div>
              <h3 className="text-sm font-medium text-slate-400 mb-2">Your Tactics</h3>
              <p className="text-slate-500 text-xs mb-3">When opponent blundered, did you punish?</p>
              <div className="flex items-baseline gap-2 mb-2">
                <span className="text-3xl font-bold text-emerald-400">{stats.user.rate}%</span>
                <span className="text-slate-500 text-sm">found</span>
              </div>
              <div className="flex gap-4 text-sm">
                <span className="text-emerald-400">{stats.user.found} found</span>
                <span className="text-red-400">{stats.user.missed} missed</span>
              </div>
            </div>
            {/* Opponent tactics */}
            <div>
              <h3 className="text-sm font-medium text-slate-400 mb-2">Opponent Tactics</h3>
              <p className="text-slate-500 text-xs mb-3">When you blundered, did they punish?</p>
              <div className="flex items-baseline gap-2 mb-2">
                <span className="text-3xl font-bold text-indigo-400">{stats.opponent.rate}%</span>
                <span className="text-slate-500 text-sm">found</span>
              </div>
              <div className="flex gap-4 text-sm">
                <span className="text-emerald-400">{stats.opponent.found} found</span>
                <span className="text-red-400">{stats.opponent.missed} missed</span>
              </div>
            </div>
          </div>
          {/* Edge comparison */}
          {stats.user.total > 0 && stats.opponent.total > 0 && (
            <div className="mt-4 pt-4 border-t border-slate-700">
              <div className="flex items-center justify-between">
                <span className="text-sm text-slate-400">Tactical Edge</span>
                <span className={`text-lg font-semibold ${
                  stats.user.rate > stats.opponent.rate ? 'text-emerald-400' :
                  stats.user.rate < stats.opponent.rate ? 'text-red-400' : 'text-slate-400'
                }`}>
                  {stats.user.rate > stats.opponent.rate ? '+' : ''}
                  {(stats.user.rate - stats.opponent.rate).toFixed(1)}%
                </span>
              </div>
            </div>
          )}

          {/* Position type breakdown */}
          {stats.byPosition && stats.byPosition.length > 0 && (
            <PositionBreakdown positions={stats.byPosition} />
          )}

          {/* Theme breakdown */}
          {stats.byTheme && stats.byTheme.length > 0 && (
            <ThemeBreakdown themes={stats.byTheme} />
          )}
        </div>
      )}

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
                    {puzzle.themes.slice(0, 3).map(theme => (
                      <span
                        key={theme}
                        className="px-2 py-0.5 bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30 rounded-full text-[10px] font-medium text-amber-400"
                      >
                        {tagDisplayName(theme)}
                      </span>
                    ))}
                    {puzzle.themes.length > 3 && (
                      <span className="px-2 py-0.5 text-[10px] text-slate-500">
                        +{puzzle.themes.length - 3}
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

/** Position type breakdown - compare find rates by position type */
function PositionBreakdown({ positions }: { positions: PositionStats[] }) {
  if (positions.length === 0) return null;

  const getEdgeColor = (edge: number) =>
    edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-400';

  const getPositionLabel = (pos: string) => {
    switch (pos) {
      case 'winning': return { label: 'Winning', desc: '(you were ahead)', color: 'text-emerald-400' };
      case 'equal': return { label: 'Equal', desc: '(balanced position)', color: 'text-slate-300' };
      case 'losing': return { label: 'Losing', desc: '(you were behind)', color: 'text-red-400' };
      default: return { label: pos, desc: '', color: 'text-white' };
    }
  };

  return (
    <div className="mt-4 pt-4 border-t border-slate-700">
      <h3 className="text-sm font-medium text-slate-400 mb-3">Find Rate by Position</h3>
      <p className="text-xs text-slate-500 mb-3">
        How well you find tactics based on the position before opponent blundered.
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-slate-400 border-b border-slate-700">
              <th className="text-left py-2 pr-4">Position</th>
              <th className="text-right py-2 px-3">Your Puzzles</th>
              <th className="text-right py-2 px-3">Your Rate</th>
              <th className="text-right py-2 px-3">Opp Rate</th>
              <th className="text-right py-2 pl-3">Edge</th>
            </tr>
          </thead>
          <tbody>
            {positions.map((p) => {
              const { label, desc, color } = getPositionLabel(p.position);
              const edge = p.user.rate - p.opponent.rate;
              return (
                <tr key={p.position} className="border-b border-slate-700/50 hover:bg-slate-700/30">
                  <td className="py-2 pr-4">
                    <span className={`font-medium ${color}`}>{label}</span>
                    <span className="text-slate-500 text-xs ml-2">{desc}</span>
                  </td>
                  <td className="py-2 px-3 text-right text-slate-300">{p.user.total}</td>
                  <td className="py-2 px-3 text-right text-emerald-400">{p.user.rate}%</td>
                  <td className="py-2 px-3 text-right text-indigo-400">{p.opponent.rate}%</td>
                  <td className={`py-2 pl-3 text-right font-medium ${getEdgeColor(edge)}`}>
                    {edge > 0 ? '+' : ''}{edge.toFixed(1)}%
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/** Theme breakdown - compare find rates by puzzle theme */
function ThemeBreakdown({ themes }: { themes: ThemeStats[] }) {
  // Only show visible themes with enough data, sorted by total puzzles
  const filtered = themes
    .filter(t => isVisibleTag(t.theme) && t.user.total >= 50)
    .sort((a, b) => (b.user.total + b.opponent.total) - (a.user.total + a.opponent.total));

  if (filtered.length === 0) return null;

  const getEdgeColor = (edge: number) =>
    edge > 5 ? 'text-emerald-400' : edge < -5 ? 'text-red-400' : 'text-slate-400';

  return (
    <div className="mt-4 pt-4 border-t border-slate-700">
      <h3 className="text-sm font-medium text-slate-400 mb-3">Find Rate by Theme</h3>
      <p className="text-xs text-slate-500 mb-3">
        How well you find tactics by puzzle type.
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-slate-400 border-b border-slate-700">
              <th className="text-left py-2 pr-4">Theme</th>
              <th className="text-right py-2 px-3">Your Puzzles</th>
              <th className="text-right py-2 px-3">Your Rate</th>
              <th className="text-right py-2 px-3">Opp Rate</th>
              <th className="text-right py-2 pl-3">Edge</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((t) => {
              const edge = t.user.rate - t.opponent.rate;
              return (
                <tr key={t.theme} className="border-b border-slate-700/50 hover:bg-slate-700/30">
                  <td className="py-2 pr-4">
                    <span className="font-medium text-amber-400">{tagDisplayName(t.theme)}</span>
                  </td>
                  <td className="py-2 px-3 text-right text-slate-300">{t.user.total}</td>
                  <td className="py-2 px-3 text-right text-emerald-400">{t.user.rate}%</td>
                  <td className="py-2 px-3 text-right text-indigo-400">{t.opponent.rate}%</td>
                  <td className={`py-2 pl-3 text-right font-medium ${getEdgeColor(edge)}`}>
                    {edge > 0 ? '+' : ''}{edge.toFixed(1)}%
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
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
          darkSquareStyle: { backgroundColor: '#779952' },
          lightSquareStyle: { backgroundColor: '#edeed1' },
        }}
      />
    </div>
  );
}
