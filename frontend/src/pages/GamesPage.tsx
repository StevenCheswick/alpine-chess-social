import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { ChessBoard, MiniChessBoard } from '../components/chess';
import { useAuthStore } from '../stores/authStore';
import { API_BASE_URL } from '../config/api';
import { gameService, type SyncResponse, type AnalyzeResponse } from '../services/gameService';

const API_BASE = API_BASE_URL;
const GAMES_PER_PAGE = 25;

interface Game {
  id: string;
  opponent: string;
  opponentRating: number | null;
  userRating: number | null;
  result: 'W' | 'L' | 'D';
  timeControl: string;
  date: string;
  tags: string[];
  moves: string[];
  userColor: 'white' | 'black';
}

// Get all unique tags with counts
function getTagCounts(games: Game[]): Map<string, number> {
  const counts = new Map<string, number>();
  games.forEach(game => {
    game.tags.forEach(tag => {
      counts.set(tag, (counts.get(tag) || 0) + 1);
    });
  });
  return counts;
}

const resultColors = {
  W: 'text-green-500',
  L: 'text-red-500',
  D: 'text-slate-400',
};

const resultLabels = {
  W: 'Won',
  L: 'Lost',
  D: 'Draw',
};

function formatDate(dateStr: string): string {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

export default function GamesPage() {
  const { user } = useAuthStore();
  const chessComUsername = user?.chessComUsername;
  
  const [games, setGames] = useState<Game[]>([]);
  const [totalGames, setTotalGames] = useState(0);
  const [loading, setLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [expandedGameId, setExpandedGameId] = useState<string | null>(null);
  const [lastSynced, setLastSynced] = useState<string | null>(null);
  const [unanalyzedCount, setUnanalyzedCount] = useState<number | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [allTags, setAllTags] = useState<Map<string, number>>(new Map());

  // Load stored games on mount if user has linked Chess.com account
  useEffect(() => {
    if (chessComUsername) {
      loadStoredGames(chessComUsername, 1);
      loadAllTags(chessComUsername);
    }
  }, [chessComUsername]);

  // Get currently selected tag (only support single tag filter for now)
  const selectedTag = selectedTags.size > 0 ? Array.from(selectedTags)[0] : null;

  // Load games when page or tag filter changes
  useEffect(() => {
    if (chessComUsername && currentPage > 0) {
      loadStoredGames(chessComUsername, currentPage, selectedTag);
    }
  }, [currentPage, selectedTag]);

  // Load games from database with pagination and optional tag filter
  const loadStoredGames = async (user: string, page: number, tag: string | null) => {
    setLoading(true);
    setError(null);

    const offset = (page - 1) * GAMES_PER_PAGE;
    let url = `${API_BASE}/api/games/stored?username=${encodeURIComponent(user)}&limit=${GAMES_PER_PAGE}&offset=${offset}`;
    if (tag) {
      url += `&tag=${encodeURIComponent(tag)}`;
    }

    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to load games: ${response.statusText}`);
      }
      const data = await response.json();
      setGames(data.games || []);
      setTotalGames(data.total || 0);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load games');
    } finally {
      setLoading(false);
    }
  };

  // Load tag counts from dedicated endpoint
  const loadAllTags = async (user: string) => {
    try {
      const response = await fetch(
        `${API_BASE}/api/games/tags?username=${encodeURIComponent(user)}`
      );
      if (!response.ok) return;
      const data = await response.json();
      setAllTags(new Map(Object.entries(data.tags || {})));
    } catch {
      // Ignore errors for tag loading
    }
  };

  const syncGames = async () => {
    if (!chessComUsername) return;

    setSyncing(true);
    setError(null);

    try {
      const syncResult: SyncResponse = await gameService.syncGames();
      setLastSynced(syncResult.lastSyncedAt);

      // Reload games from database after sync
      setCurrentPage(1);
      setSelectedTags(new Set()); // Clear filter after sync
      await loadStoredGames(chessComUsername, 1, null);
      await loadAllTags(chessComUsername);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sync games');
    } finally {
      setSyncing(false);
    }
  };

  const analyzeGames = async () => {
    if (!chessComUsername) return;

    setAnalyzing(true);
    setError(null);

    try {
      const result: AnalyzeResponse = await gameService.analyzeGames(50);
      setUnanalyzedCount(result.remaining);

      // Reload games to show new tags
      await loadStoredGames(chessComUsername, currentPage, selectedTag);
      await loadAllTags(chessComUsername);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to analyze games');
    } finally {
      setAnalyzing(false);
    }
  };

  const sortedTags = Array.from(allTags.keys()).sort();

  // Server-side pagination - games already paginated from API
  const totalPages = Math.ceil(totalGames / GAMES_PER_PAGE);
  const startIndex = (currentPage - 1) * GAMES_PER_PAGE;
  const endIndex = Math.min(startIndex + GAMES_PER_PAGE, totalGames);

  const goToPage = (page: number) => {
    const newPage = Math.max(1, Math.min(page, totalPages));
    if (newPage !== currentPage) {
      setCurrentPage(newPage);
      setExpandedGameId(null); // Collapse any expanded game when changing pages
    }
  };

  const toggleTag = (tag: string) => {
    setSelectedTags(prev => {
      // Single tag selection - clicking same tag deselects, clicking different tag switches
      if (prev.has(tag)) {
        return new Set(); // Deselect
      } else {
        return new Set([tag]); // Select only this tag
      }
    });
    setCurrentPage(1); // Reset to first page when filtering
  };

  const clearTags = () => {
    setSelectedTags(new Set());
    setCurrentPage(1);
  };

  const toggleGame = (gameId: string) => {
    setExpandedGameId(prev => prev === gameId ? null : gameId);
  };

  // Show link account prompt if no Chess.com username
  if (!chessComUsername) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-white">My Games</h1>
          <p className="text-slate-400 text-sm mt-1">
            Analyze your games to discover patterns and tag notable moments
          </p>
        </div>

        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-3xl">&#9823;</span>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">Link your Chess.com account</h2>
          <p className="text-slate-400 mb-6">
            Connect your Chess.com account to sync and analyze your games.
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
      <div>
        <h1 className="text-2xl font-bold text-white">My Games</h1>
        <p className="text-slate-400 text-sm mt-1">
          Analyze your games to discover patterns and tag notable moments
        </p>
      </div>

      {/* Sync & Analyze Section */}
      <div className="card p-4 space-y-4">
        {/* Sync Row */}
        <div className="flex items-center justify-between">
          <div>
            <p className="text-white font-medium">Sync games from Chess.com</p>
            <p className="text-slate-500 text-xs mt-1">
              {lastSynced
                ? `Last synced: ${new Date(lastSynced).toLocaleString()}`
                : games.length === 0
                  ? 'First sync will fetch all your games'
                  : 'Sync to fetch new games'}
            </p>
          </div>
          <button
            onClick={syncGames}
            disabled={syncing || loading || analyzing}
            className="px-4 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
          >
            {syncing ? (
              <>
                <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                Syncing...
              </>
            ) : (
              'Sync Games'
            )}
          </button>
        </div>

        {/* Analyze Row */}
        <div className="flex items-center justify-between border-t border-slate-800 pt-4">
          <div>
            <p className="text-white font-medium">Analyze games for patterns</p>
            <p className="text-slate-500 text-xs mt-1">
              {unanalyzedCount !== null
                ? unanalyzedCount > 0
                  ? `${unanalyzedCount} games need analysis`
                  : 'All games analyzed'
                : 'Detect queen sacrifices, knight forks, and more'}
            </p>
          </div>
          <button
            onClick={analyzeGames}
            disabled={analyzing || loading || syncing || games.length === 0}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
          >
            {analyzing ? (
              <>
                <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                Analyzing...
              </>
            ) : (
              'Analyze Games'
            )}
          </button>
        </div>

        {error && (
          <p className="text-red-500 text-sm mt-2">{error}</p>
        )}
      </div>

      {/* Loading State */}
      {(loading || syncing || analyzing) && (
        <div className="text-center py-12">
          <div className="inline-block w-8 h-8 border-4 border-slate-700 border-t-primary-500 rounded-full animate-spin"></div>
          <p className="text-slate-400 mt-4">
            {syncing ? 'Syncing games from Chess.com...' : analyzing ? 'Analyzing games...' : 'Loading games...'}
          </p>
          {syncing && (
            <p className="text-slate-500 text-sm mt-2">This may take a moment for first-time sync</p>
          )}
          {analyzing && (
            <p className="text-slate-500 text-sm mt-2">Detecting patterns in your games</p>
          )}
        </div>
      )}

      {/* Games Content */}
      {!loading && !syncing && !analyzing && (games.length > 0 || totalGames > 0) && (
        <>
          {/* Stats */}
          <div className="flex items-center justify-between">
            <p className="text-slate-400 text-sm">
              {totalGames} {totalGames === 1 ? 'game' : 'games'}
            </p>
          </div>

          {/* Tag Filter */}
          {sortedTags.length > 0 && (
            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                {sortedTags.map(tag => {
                  const isSelected = selectedTags.has(tag);
                  return (
                    <button
                      key={tag}
                      onClick={() => toggleTag(tag)}
                      className={`px-3 py-1.5 rounded-full text-sm font-medium transition-colors ${
                        isSelected
                          ? 'bg-primary-600 text-white'
                          : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
                      }`}
                    >
                      {tag} ({allTags.get(tag)})
                    </button>
                  );
                })}
              </div>
              {selectedTags.size > 0 && (
                <button
                  onClick={clearTags}
                  className="text-sm text-slate-400 hover:text-white transition-colors"
                >
                  Clear filters
                </button>
              )}
            </div>
          )}

          {/* Pagination Info */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between text-sm text-slate-400">
              <span>
                Showing {startIndex + 1}-{endIndex} of {totalGames}
              </span>
              <span>Page {currentPage} of {totalPages}</span>
            </div>
          )}

          {/* Games List */}
          <div className="space-y-3">
            {games.map(game => {
              const isExpanded = expandedGameId === game.id;
              return (
                <div
                  key={game.id}
                  className="card overflow-hidden"
                >
                  {/* Game Header - Clickable */}
                  <div
                    onClick={() => toggleGame(game.id)}
                    className="p-4 hover:bg-slate-800/50 transition-colors cursor-pointer"
                  >
                    <div className="flex items-start gap-4">
                      {/* Mini Board */}
                      <MiniChessBoard
                        moves={game.moves}
                        orientation={game.userColor}
                        size={80}
                      />

                      {/* Game Info */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between mb-1">
                          <div className="flex items-center gap-2">
                            <span className="text-white font-medium">vs {game.opponent}</span>
                            {game.opponentRating && (
                              <span className="text-slate-500 text-sm">({game.opponentRating})</span>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            <span className={`font-semibold ${resultColors[game.result]}`}>
                              {resultLabels[game.result]}
                            </span>
                            <svg
                              className={`w-5 h-5 text-slate-400 transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                            >
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                            </svg>
                          </div>
                        </div>

                        <div className="flex items-center gap-3 text-slate-500 text-sm mb-2">
                          {game.timeControl && <span>{game.timeControl}</span>}
                          {game.date && <span>{formatDate(game.date)}</span>}
                          <span>{game.moves.length} moves</span>
                        </div>

                        {game.tags.length > 0 && (
                          <div className="flex flex-wrap gap-1.5">
                            {game.tags.map(tag => (
                              <span
                                key={tag}
                                className="px-2 py-0.5 bg-amber-500/20 border border-amber-500/30 rounded text-xs text-amber-400"
                              >
                                {tag}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Expanded Game Board */}
                  {isExpanded && (
                    <div className="border-t border-slate-800 p-4">
                      <ChessBoard
                        moves={game.moves}
                        orientation={game.userColor}
                        whitePlayer={{
                          username: game.userColor === 'white' ? chessComUsername : game.opponent,
                          rating: game.userColor === 'white' ? game.userRating || undefined : game.opponentRating || undefined,
                        }}
                        blackPlayer={{
                          username: game.userColor === 'black' ? chessComUsername : game.opponent,
                          rating: game.userColor === 'black' ? game.userRating || undefined : game.opponentRating || undefined,
                        }}
                      />
                    </div>
                  )}
                </div>
              );
            })}

            {games.length === 0 && (
              <div className="text-center py-12 text-slate-500">
                No games on this page
              </div>
            )}
          </div>

          {/* Pagination Controls */}
          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2">
              <button
                onClick={() => goToPage(1)}
                disabled={currentPage === 1}
                className="px-3 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                First
              </button>
              <button
                onClick={() => goToPage(currentPage - 1)}
                disabled={currentPage === 1}
                className="px-3 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                Previous
              </button>
              <span className="px-4 py-2 text-slate-400">
                {currentPage} / {totalPages}
              </span>
              <button
                onClick={() => goToPage(currentPage + 1)}
                disabled={currentPage === totalPages}
                className="px-3 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                Next
              </button>
              <button
                onClick={() => goToPage(totalPages)}
                disabled={currentPage === totalPages}
                className="px-3 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                Last
              </button>
            </div>
          )}
        </>
      )}

      {/* Empty State */}
      {!loading && !syncing && !analyzing && totalGames === 0 && !error && (
        <div className="text-center py-12 text-slate-500">
          No games found. Click "Sync Games" to download your Chess.com games.
        </div>
      )}
    </div>
  );
}
