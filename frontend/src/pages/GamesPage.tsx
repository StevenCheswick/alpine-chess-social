import { useState, useEffect } from 'react';
import { ChessBoard, MiniChessBoard } from '../components/chess';

import { API_BASE_URL } from '../config/api';
const API_BASE = API_BASE_URL;
const GAMES_PER_PAGE = 25;
// TODO: Get from auth context when login is implemented
const TEST_USERNAME = 'brexwick';

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
  const [username] = useState(TEST_USERNAME);
  const [games, setGames] = useState<Game[]>([]);
  const [loading, setLoading] = useState(true); // Start loading
  const [error, setError] = useState<string | null>(null);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [expandedGameId, setExpandedGameId] = useState<string | null>(null);
  const [lastAnalyzed, setLastAnalyzed] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);

  // Load stored games on mount
  useEffect(() => {
    loadStoredGames(TEST_USERNAME);
  }, []);

  // Load games from database (fast, no re-analysis)
  const loadStoredGames = async (user: string) => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE}/api/games/stored?username=${encodeURIComponent(user)}`);
      if (!response.ok) {
        throw new Error(`Failed to load games: ${response.statusText}`);
      }
      const data = await response.json();
      setGames(data.games || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load games');
    } finally {
      setLoading(false);
    }
  };

  const analyzeGames = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE}/api/games?username=${encodeURIComponent(username)}`);
      if (!response.ok) {
        throw new Error(`Failed to analyze games: ${response.statusText}`);
      }
      const data = await response.json();
      setGames(data.games || []);
      setLastAnalyzed(new Date().toLocaleTimeString());
      setCurrentPage(1); // Reset to first page on new analysis
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to analyze games');
    } finally {
      setLoading(false);
    }
  };

  const tagCounts = getTagCounts(games);
  const allTags = Array.from(tagCounts.keys()).sort();

  const filteredGames = selectedTags.size === 0
    ? games
    : games.filter(game =>
        Array.from(selectedTags).every(tag => game.tags.includes(tag))
      );

  // Pagination
  const totalPages = Math.ceil(filteredGames.length / GAMES_PER_PAGE);
  const startIndex = (currentPage - 1) * GAMES_PER_PAGE;
  const endIndex = startIndex + GAMES_PER_PAGE;
  const paginatedGames = filteredGames.slice(startIndex, endIndex);

  const goToPage = (page: number) => {
    setCurrentPage(Math.max(1, Math.min(page, totalPages)));
    setExpandedGameId(null); // Collapse any expanded game when changing pages
  };

  const toggleTag = (tag: string) => {
    setSelectedTags(prev => {
      const next = new Set(prev);
      if (next.has(tag)) {
        next.delete(tag);
      } else {
        next.add(tag);
      }
      return next;
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

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-white">My Games</h1>
        <p className="text-slate-400 text-sm mt-1">
          Analyze your games to discover patterns and tag notable moments
        </p>
      </div>

      {/* Analyze Section */}
      <div className="card p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-white font-medium">Re-analyze games to detect new patterns</p>
            {lastAnalyzed && (
              <p className="text-slate-500 text-xs mt-1">Last analyzed: {lastAnalyzed}</p>
            )}
          </div>
          <button
            onClick={analyzeGames}
            disabled={loading}
            className="px-4 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Analyzing...' : 'Analyze Games'}
          </button>
        </div>
        {error && (
          <p className="text-red-500 text-sm mt-2">{error}</p>
        )}
      </div>

      {/* Loading State */}
      {loading && (
        <div className="text-center py-12">
          <div className="inline-block w-8 h-8 border-4 border-slate-700 border-t-primary-500 rounded-full animate-spin"></div>
          <p className="text-slate-400 mt-4">Loading games...</p>
        </div>
      )}

      {/* Games Content */}
      {!loading && games.length > 0 && (
        <>
          {/* Stats */}
          <div className="flex items-center justify-between">
            <p className="text-slate-400 text-sm">
              {filteredGames.length} {filteredGames.length === 1 ? 'game' : 'games'}
              {selectedTags.size > 0 && ` (filtered from ${games.length})`}
            </p>
          </div>

          {/* Tag Filter */}
          {allTags.length > 0 && (
            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                {allTags.map(tag => {
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
                      {tag} ({tagCounts.get(tag)})
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
                Showing {startIndex + 1}-{Math.min(endIndex, filteredGames.length)} of {filteredGames.length}
              </span>
              <span>Page {currentPage} of {totalPages}</span>
            </div>
          )}

          {/* Games List */}
          <div className="space-y-3">
            {paginatedGames.map(game => {
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
                          username: game.userColor === 'white' ? username : game.opponent,
                          rating: game.userColor === 'white' ? game.userRating || undefined : game.opponentRating || undefined,
                        }}
                        blackPlayer={{
                          username: game.userColor === 'black' ? username : game.opponent,
                          rating: game.userColor === 'black' ? game.userRating || undefined : game.opponentRating || undefined,
                        }}
                      />
                    </div>
                  )}
                </div>
              );
            })}

            {paginatedGames.length === 0 && (
              <div className="text-center py-12 text-slate-500">
                No games match the selected filters
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
      {!loading && games.length === 0 && !error && (
        <div className="text-center py-12 text-slate-500">
          No games found. Click "Analyze Games" to scan for patterns.
        </div>
      )}
    </div>
  );
}
