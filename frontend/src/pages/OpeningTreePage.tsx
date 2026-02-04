import { useState, useEffect } from 'react';
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

function formatCurrentLine(path: string[]): string {
  if (path.length === 0) return 'Starting position';

  let result = '';
  for (let i = 0; i < path.length; i++) {
    const fullMove = Math.floor(i / 2) + 1;
    const isWhite = i % 2 === 0;
    if (isWhite) {
      result += `${fullMove}. ${path[i]} `;
    } else {
      result += `${path[i]} `;
    }
  }
  return result.trim();
}

function getWinRateColor(winRate: number): string {
  if (winRate >= 60) return 'text-green-400';
  if (winRate >= 50) return 'text-green-300';
  if (winRate >= 40) return 'text-yellow-400';
  if (winRate >= 30) return 'text-orange-400';
  return 'text-red-400';
}

function getWinRateBg(winRate: number): string {
  if (winRate >= 60) return 'bg-green-500/20';
  if (winRate >= 50) return 'bg-green-500/10';
  if (winRate >= 40) return 'bg-yellow-500/10';
  if (winRate >= 30) return 'bg-orange-500/10';
  return 'bg-red-500/10';
}

export default function OpeningTreePage() {
  const { user } = useAuthStore();
  const chessComUsername = user?.chessComUsername;

  const [color, setColor] = useState<'white' | 'black'>('white');
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [currentPath, setCurrentPath] = useState<string[]>([]);
  const [currentNode, setCurrentNode] = useState<TreeNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [totalGames, setTotalGames] = useState(0);

  // Fetch tree when color changes
  useEffect(() => {
    if (chessComUsername) {
      fetchOpeningTree(color);
    }
  }, [color, chessComUsername]);

  const fetchOpeningTree = async (colorToFetch: 'white' | 'black') => {
    setLoading(true);
    setError(null);

    try {
      const response = await openingService.getOpeningTree(colorToFetch);
      setTree(response.rootNode);
      setCurrentNode(response.rootNode);
      setCurrentPath([]);
      setTotalGames(response.totalGames);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load opening tree');
      setTree(null);
      setCurrentNode(null);
    } finally {
      setLoading(false);
    }
  };

  // Navigate to a child node
  const navigateToMove = (move: string) => {
    if (!currentNode) return;

    const childNode = currentNode.children.find(c => c.move === move);
    if (childNode) {
      setCurrentPath([...currentPath, move]);
      setCurrentNode(childNode);
    }
  };

  // Go back one level
  const goBack = () => {
    if (currentPath.length === 0 || !tree) return;

    const newPath = [...currentPath];
    newPath.pop();

    // Navigate to the node at newPath
    let node = tree;
    for (const move of newPath) {
      const child = node.children.find(c => c.move === move);
      if (child) {
        node = child;
      } else {
        break;
      }
    }

    setCurrentPath(newPath);
    setCurrentNode(node);
  };

  // Reset to root
  const goToRoot = () => {
    if (!tree) return;
    setCurrentPath([]);
    setCurrentNode(tree);
  };

  // Current FEN for the board
  const currentFen = currentNode?.fen || STARTING_FEN;

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
            className="inline-block px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
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

      {/* Loading State */}
      {loading && (
        <div className="text-center py-12">
          <div className="inline-block w-8 h-8 border-4 border-slate-700 border-t-primary-500 rounded-full animate-spin"></div>
          <p className="text-slate-400 mt-4">Loading opening tree...</p>
        </div>
      )}

      {/* Error State */}
      {error && (
        <div className="card p-4 bg-red-500/10 border-red-500/30">
          <p className="text-red-400">{error}</p>
        </div>
      )}

      {/* Main Content */}
      {!loading && !error && tree && (
        <div className="space-y-3">
          {/* Current Line Breadcrumb */}
          <div className="flex items-center gap-2 flex-wrap text-sm">
            <button
              onClick={goToRoot}
              className={`px-2 py-1 rounded ${
                currentPath.length === 0
                  ? 'bg-primary-600 text-white'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800'
              }`}
            >
              Start
            </button>
            {currentPath.length > 0 && (
              <>
                {currentPath.map((move, idx) => (
                  <span key={idx} className="flex items-center gap-1">
                    <span className="text-slate-600">&gt;</span>
                    <span className="text-slate-300">
                      {formatMoveNumber(idx)} {move}
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
                    options={{
                      position: currentFen,
                      boardOrientation: color,
                      allowDragging: false,
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Move List */}
            <div className="flex-1 min-w-0">
              {currentNode?.children.length === 0 ? (
                <div className="text-center py-8 text-slate-500 text-sm">
                  No continuation data
                </div>
              ) : (
                <div className="space-y-1">
                  {currentNode?.children.map((child) => (
                    <button
                      key={child.move}
                      onClick={() => navigateToMove(child.move)}
                      className="w-full px-3 py-2 rounded-lg text-left hover:bg-slate-800 transition-colors flex items-center justify-between"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-slate-500 text-sm w-6">
                          {formatMoveNumber(currentPath.length)}
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
      )}

      {/* Empty State */}
      {!loading && !error && tree && totalGames === 0 && (
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
            className="inline-block px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
          >
            Go to Games Page
          </Link>
        </div>
      )}
    </div>
  );
}
