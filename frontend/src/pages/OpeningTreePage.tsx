import { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router-dom';
import { Chessboard } from 'react-chessboard';
import { useAuthStore } from '../stores/authStore';
import { openingService, type TreeNode } from '../services/openingService';

const STARTING_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

function formatMoveNumber(index: number): string {
  const fullMove = Math.floor(index / 2) + 1;
  const isWhite = index % 2 === 0;
  return isWhite ? `${fullMove}.` : `${fullMove}...`;
}

function getWinRateColor(winRate: number): string {
  if (winRate >= 60) return 'text-green-400';
  if (winRate >= 50) return 'text-green-300';
  if (winRate >= 40) return 'text-yellow-400';
  if (winRate >= 30) return 'text-orange-400';
  return 'text-red-400';
}

interface PathEntry {
  move: string;
  fen: string;
}

export default function OpeningTreePage() {
  const { user } = useAuthStore();
  const chessComUsername = user?.chessComUsername;

  const [color, setColor] = useState<'white' | 'black'>('white');
  const [children, setChildren] = useState<TreeNode[]>([]);
  const [currentFen, setCurrentFen] = useState(STARTING_FEN);
  const [path, setPath] = useState<PathEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [totalGames, setTotalGames] = useState(0);
  const [_nodeStats, setNodeStats] = useState({ games: 0, wins: 0, losses: 0, draws: 0, winRate: 0 });

  const fetchPosition = useCallback(async (fen: string, colorToFetch: string) => {
    setLoading(true);
    setError(null);
    try {
      const response = await openingService.getOpeningTree(colorToFetch as 'white' | 'black', fen);
      setChildren(response.children);
      setTotalGames(response.totalGames);
      setNodeStats({
        games: response.games,
        wins: response.wins,
        losses: response.losses,
        draws: response.draws,
        winRate: response.winRate,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load opening tree');
      setChildren([]);
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch root when color changes
  useEffect(() => {
    if (chessComUsername) {
      setPath([]);
      setCurrentFen(STARTING_FEN);
      fetchPosition(STARTING_FEN, color);
    }
  }, [color, chessComUsername, fetchPosition]);

  const navigateToMove = (child: TreeNode) => {
    setPath(prev => [...prev, { move: child.move, fen: child.fen }]);
    setCurrentFen(child.fen);
    fetchPosition(child.fen, color);
  };

  const goBack = () => {
    if (path.length === 0) return;
    const newPath = path.slice(0, -1);
    const fen = newPath.length > 0 ? newPath[newPath.length - 1].fen : STARTING_FEN;
    setPath(newPath);
    setCurrentFen(fen);
    fetchPosition(fen, color);
  };

  const goToRoot = () => {
    setPath([]);
    setCurrentFen(STARTING_FEN);
    fetchPosition(STARTING_FEN, color);
  };

  // Show link account prompt if no Chess.com username
  if (!chessComUsername) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-white">Opening Tree</h1>
          <p className="text-slate-400 text-sm mt-1">
            Explore your opening repertoire and see win rates for each line
          </p>
        </div>

        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-3xl">&#9823;</span>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">Link your Chess.com account</h2>
          <p className="text-slate-400 mb-6">
            Connect your Chess.com account to view your opening tree.
          </p>
          <Link
            to={user ? `/${user.username}` : '/'}
            className="inline-block px-6 py-3 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 shadow-[0_0_12px_rgba(16,185,129,0.3)]"
          >
            Go to Profile Settings
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Opening Tree</h1>
          <p className="text-slate-400 text-sm mt-1">
            Explore your opening repertoire and see win rates for each line
          </p>
        </div>
      </div>

      {/* Color Tabs */}
      <div className="flex gap-2">
        <button
          onClick={() => setColor('white')}
          className={`px-6 py-3 rounded-lg font-medium transition-colors ${
            color === 'white'
              ? 'bg-white text-slate-900'
              : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
          }`}
        >
          White Repertoire
        </button>
        <button
          onClick={() => setColor('black')}
          className={`px-6 py-3 rounded-lg font-medium transition-colors ${
            color === 'black'
              ? 'bg-slate-600 text-white'
              : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
          }`}
        >
          Black Repertoire
        </button>
      </div>

      {/* Error State */}
      {error && (
        <div className="card p-4 bg-red-500/10 border-red-500/30">
          <p className="text-red-400">{error}</p>
        </div>
      )}

      {/* Main Content */}
      <div className="space-y-3">
        {/* Current Line Breadcrumb */}
        <div className="flex items-center gap-2 flex-wrap text-sm">
          <button
            onClick={goToRoot}
            className={`px-2 py-1 rounded ${
              path.length === 0
                ? 'bg-emerald-600 text-white'
                : 'text-slate-400 hover:text-white hover:bg-slate-800'
            }`}
          >
            Start
          </button>
          {path.length > 0 && (
            <>
              {path.map((entry, idx) => (
                <span key={idx} className="flex items-center gap-1">
                  <span className="text-slate-600">&gt;</span>
                  <span className="text-slate-300">
                    {formatMoveNumber(idx)} {entry.move}
                  </span>
                </span>
              ))}
              <button
                onClick={goBack}
                className="ml-2 text-slate-500 hover:text-white transition-colors"
              >
                (back)
              </button>
            </>
          )}
          <span className="ml-auto text-slate-500 text-xs">
            {totalGames} games
          </span>
        </div>

        {/* Board + Moves side by side */}
        <div className="flex gap-4">
          {/* Board */}
          <div className="w-80 flex-shrink-0">
            <div className="card overflow-hidden">
              <div className="aspect-square">
                <Chessboard
                  key={`opening-${color}`}
                  options={{
                    position: currentFen,
                    boardOrientation: color,
                  }}
                />
              </div>
            </div>
          </div>

          {/* Move List */}
          <div className="flex-1 min-w-0">
            {loading ? (
              <div className="text-center py-8">
                <div className="inline-block w-6 h-6 border-3 border-slate-700 border-t-emerald-500 rounded-full animate-spin"></div>
              </div>
            ) : children.length === 0 ? (
              <div className="text-center py-8 text-slate-500 text-sm">
                No continuation data
              </div>
            ) : (
              <div className="space-y-1">
                {children.map((child) => (
                  <button
                    key={child.move}
                    onClick={() => navigateToMove(child)}
                    className="w-full px-3 py-2 rounded-lg text-left hover:bg-slate-800 transition-colors flex items-center justify-between"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-slate-500 text-sm w-6">
                        {formatMoveNumber(path.length)}
                      </span>
                      <span className="text-white font-medium">{child.move}</span>
                      <span className="text-slate-500 text-sm">
                        ({child.games})
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <div className="w-16 h-1.5 bg-slate-800 rounded-full overflow-hidden flex">
                        <div
                          className="bg-green-500 h-full"
                          style={{ width: `${(child.wins / child.games) * 100}%` }}
                        />
                        <div
                          className="bg-slate-500 h-full"
                          style={{ width: `${(child.draws / child.games) * 100}%` }}
                        />
                        <div
                          className="bg-red-500 h-full"
                          style={{ width: `${(child.losses / child.games) * 100}%` }}
                        />
                      </div>
                      <span className={`text-sm font-medium w-12 text-right ${getWinRateColor(child.winRate)}`}>
                        {child.winRate.toFixed(0)}%
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Empty State */}
      {!loading && totalGames === 0 && (
        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-3xl">&#9823;</span>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">No games found</h2>
          <p className="text-slate-400 mb-6">
            Sync your games first to build your opening tree.
          </p>
          <Link
            to="/games"
            className="inline-block px-6 py-3 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 shadow-[0_0_12px_rgba(16,185,129,0.3)]"
          >
            Go to Games Page
          </Link>
        </div>
      )}
    </div>
  );
}
