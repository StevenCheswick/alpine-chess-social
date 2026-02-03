import { useState } from 'react';
import { ChessBoard } from '../components/chess';

const API_BASE = 'http://localhost:8000';

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
  const [username, setUsername] = useState('');
  const [games, setGames] = useState<Game[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [expandedGameId, setExpandedGameId] = useState<string | null>(null);
  const [lastSynced, setLastSynced] = useState<string | null>(null);

  const fetchGames = async () => {
    if (!username.trim()) {
      setError('Please enter a username');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE}/api/games?username=${encodeURIComponent(username)}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch games: ${response.statusText}`);
      }
      const data = await response.json();
      setGames(data.games || []);
      setLastSynced(new Date().toLocaleTimeString());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch games');
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
  };

  const clearTags = () => setSelectedTags(new Set());

  const toggleGame = (gameId: string) => {
    setExpandedGameId(prev => prev === gameId ? null : gameId);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-white">My Games</h1>
        <p className="text-slate-400 text-sm mt-1">
          Sync your Chess.com games to browse and filter by patterns
        </p>
      </div>

      {/* Sync Section */}
      <div className="card p-4">
        <div className="flex items-center gap-3">
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && fetchGames()}
            placeholder="Chess.com username"
            className="flex-1 px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-primary-500"
          />
          <button
            onClick={fetchGames}
            disabled={loading}
            className="px-4 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Syncing...' : 'Sync Games'}
          </button>
        </div>
        {lastSynced && (
          <p className="text-slate-500 text-xs mt-2">Last synced: {lastSynced}</p>
        )}
        {error && (
          <p className="text-red-500 text-sm mt-2">{error}</p>
        )}
      </div>

      {/* Loading State */}
      {loading && (
        <div className="text-center py-12">
          <div className="inline-block w-8 h-8 border-4 border-slate-700 border-t-primary-500 rounded-full animate-spin"></div>
          <p className="text-slate-400 mt-4">Fetching games from Chess.com (this may take a minute)...</p>
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

          {/* Games List */}
          <div className="space-y-3">
            {filteredGames.map(game => {
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
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-3">
                        <span className="text-white font-medium">vs {game.opponent}</span>
                        {game.opponentRating && (
                          <span className="text-slate-500 text-sm">({game.opponentRating})</span>
                        )}
                      </div>
                      <div className="flex items-center gap-3">
                        <span className={`font-semibold ${resultColors[game.result]}`}>
                          {resultLabels[game.result]}
                        </span>
                        {game.timeControl && (
                          <span className="text-slate-500 text-sm">{game.timeControl}</span>
                        )}
                        {game.date && (
                          <span className="text-slate-500 text-sm">{formatDate(game.date)}</span>
                        )}
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

            {filteredGames.length === 0 && (
              <div className="text-center py-12 text-slate-500">
                No games match the selected filters
              </div>
            )}
          </div>
        </>
      )}

      {/* Empty State */}
      {!loading && games.length === 0 && !error && (
        <div className="text-center py-12 text-slate-500">
          Enter your Chess.com username and click Sync to load your games
        </div>
      )}
    </div>
  );
}
