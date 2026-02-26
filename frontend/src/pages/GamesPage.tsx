import { useState, useEffect } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { MiniChessBoard } from '../components/chess';
import { useAuthStore } from '../stores/authStore';
import { API_BASE_URL } from '../config/api';
import { gameService, type AnalyzeServerResponse, type BackfillResponse } from '../services/gameService';
import { tagDisplayName, isGameTag } from '../utils/tagDisplay';

const API_BASE = API_BASE_URL;
const GAMES_PER_PAGE = 10;

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
  source: 'chess_com';
  hasAnalysis?: boolean;
  whiteAccuracy?: number;
  blackAccuracy?: number;
}

const resultLabels = {
  W: 'Won',
  L: 'Lost',
  D: 'Draw',
};

type GameType = 'bullet' | 'blitz' | 'rapid' | 'classical' | 'daily';

function getGameType(timeControl: string): GameType {
  if (!timeControl) return 'rapid';

  // Handle daily/correspondence games
  if (timeControl.includes('d') || timeControl.includes('day')) return 'daily';

  // Parse time control like "180" or "180+2" or "3|2" or "5 min"
  const match = timeControl.match(/^(\d+)/);
  if (!match) return 'rapid';

  let baseTime = parseInt(match[1]);

  // If the number is small (like 3, 5, 10), it's likely minutes
  // If it's large (like 180, 300, 600), it's likely seconds
  if (baseTime > 60) {
    baseTime = baseTime / 60; // Convert seconds to minutes
  }

  // Bullet: < 3 min
  // Blitz: 3-9 min
  // Rapid: 10-29 min
  // Classical: 30+ min
  if (baseTime < 3) return 'bullet';
  if (baseTime < 10) return 'blitz';
  if (baseTime < 30) return 'rapid';
  return 'classical';
}

const gameTypeConfig: Record<GameType, { label: string; color: string; icon: React.ReactNode }> = {
  bullet: {
    label: 'Bullet',
    color: 'text-yellow-400',
    icon: (
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
        <path d="M13 3L4 14h7v7l9-11h-7V3z" />
      </svg>
    ),
  },
  blitz: {
    label: 'Blitz',
    color: 'text-orange-400',
    icon: (
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
        <path d="M17.66 11.2c-.23-.3-.51-.56-.77-.82-.67-.6-1.43-1.03-2.07-1.66C13.33 7.26 13 4.85 13.95 3c-.95.23-1.78.75-2.49 1.32-2.59 2.08-3.61 5.75-2.39 8.9.04.1.08.2.08.33 0 .22-.15.42-.35.5-.23.1-.47.04-.66-.12a.58.58 0 01-.14-.17c-1.13-1.43-1.31-3.48-.55-5.12C5.78 10 4.87 12.3 5 14.47c.06.5.12 1 .29 1.5.14.6.41 1.2.71 1.73 1.08 1.73 2.95 2.97 4.96 3.22 2.14.27 4.43-.12 6.07-1.6 1.83-1.64 2.53-4.27 1.63-6.58l-.15-.36c-.16-.34-.34-.68-.58-.96l-.03-.03zM14.5 17.5c-.42.42-1.03.68-1.5.74-.47.06-1.06-.16-1.43-.42-.38-.28-.68-.62-.83-.96-.15-.34-.17-.76-.1-1.1.07-.36.26-.7.5-.96.76-.82 1.81-.66 2.55.05.76.72.88 1.93.61 2.65h.2z" />
      </svg>
    ),
  },
  rapid: {
    label: 'Rapid',
    color: 'text-green-400',
    icon: (
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="9" />
        <path d="M12 6v6l4 2" />
      </svg>
    ),
  },
  classical: {
    label: 'Classical',
    color: 'text-blue-400',
    icon: (
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="3" y="4" width="18" height="16" rx="2" />
        <path d="M12 8v4l2 2" />
        <path d="M7 4v-2" />
        <path d="M17 4v-2" />
      </svg>
    ),
  },
  daily: {
    label: 'Daily',
    color: 'text-purple-400',
    icon: (
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="3" y="4" width="18" height="18" rx="2" />
        <path d="M16 2v4M8 2v4M3 10h18" />
        <path d="M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01" />
      </svg>
    ),
  },
};

export default function GamesPage() {
  const { user, token } = useAuthStore();
  const chessComUsername = user?.chessComUsername;

  const [games, setGames] = useState<Game[]>([]);
  const [totalGames, setTotalGames] = useState(0);
  const [loading, setLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [searchParams, setSearchParams] = useSearchParams();
  const currentPage = Math.max(1, parseInt(searchParams.get('page') || '1', 10));
  const setCurrentPage = (page: number) => {
    setSearchParams(prev => {
      const next = new URLSearchParams(prev);
      if (page <= 1) next.delete('page');
      else next.set('page', String(page));
      return next;
    }, { replace: true });
  };
  const [allTags, setAllTags] = useState<Map<string, number>>(new Map());

  const [totalUnanalyzed, setTotalUnanalyzed] = useState(0);
  const [totalAnalyzed, setTotalAnalyzed] = useState(0);

  // Server-side analysis state
  const [serverAnalyzing, setServerAnalyzing] = useState(false);
  const [serverResult, setServerResult] = useState<AnalyzeServerResponse | null>(null);

  // Backfill (load more history) state
  const [backfilling, setBackfilling] = useState(false);
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [backfillResult, setBackfillResult] = useState<BackfillResponse | null>(null);

  const hasAnyLinkedAccount = !!chessComUsername;

  // Load unanalyzed count
  const loadUnanalyzedCount = async () => {
    try {
      const response = await fetch(`${API_BASE}/api/games/stored?limit=0&analyzed=false`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (response.ok) {
        const data = await response.json();
        setTotalUnanalyzed(data.total || 0);
      }
    } catch { /* ignore */ }
  };

  // Load analyzed count
  const loadAnalyzedCount = async () => {
    try {
      const response = await fetch(`${API_BASE}/api/games/stored?limit=0&analyzed=true`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (response.ok) {
        const data = await response.json();
        setTotalAnalyzed(data.total || 0);
      }
    } catch { /* ignore */ }
  };

  // Check backfill status
  const checkBackfillStatus = async () => {
    try {
      const status = await gameService.getBackfillStatus();
      setHasMoreHistory(status.hasMoreHistory);
    } catch { /* ignore */ }
  };

  // Load tags and counts on mount
  useEffect(() => {
    if (hasAnyLinkedAccount) {
      loadAllTags([]);
      loadUnanalyzedCount();
      loadAnalyzedCount();
      checkBackfillStatus();
    }
  }, [hasAnyLinkedAccount]);

  const selectedTagsArray = Array.from(selectedTags);

  // Load games when page or tag filter changes
  useEffect(() => {
    if (hasAnyLinkedAccount && currentPage > 0) {
      loadStoredGames(currentPage, selectedTagsArray);
    }
  }, [currentPage, JSON.stringify(selectedTagsArray)]);

  // Load games from database with pagination and optional tag filters
  const loadStoredGames = async (page: number, tags: string[]) => {
    setLoading(true);
    setError(null);

    const offset = (page - 1) * GAMES_PER_PAGE;
    let url = `${API_BASE}/api/games/stored?limit=${GAMES_PER_PAGE}&offset=${offset}`;
    if (tags.length > 0) {
      url += `&tags=${encodeURIComponent(tags.join(','))}`;
    }

    try {
      const response = await fetch(url, {
        headers: { Authorization: `Bearer ${token}` },
      });
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
  const loadAllTags = async (selectedTags: string[] = []) => {
    try {
      let url = `${API_BASE}/api/games/tags`;
      if (selectedTags.length > 0) {
        url += `?selected_tags=${encodeURIComponent(selectedTags.join(','))}`;
      }
      const response = await fetch(url, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!response.ok) return;
      const data = await response.json();
      console.log('Tags response:', data);
      console.log('Tags object:', data.tags);
      setAllTags(new Map(Object.entries(data.tags || {})));
    } catch (err) {
      console.error('Error loading tags:', err);
    }
  };

  const syncAllGames = async () => {
    if (!hasAnyLinkedAccount) return;

    setSyncing(true);
    setError(null);

    try {
      // Sync Chess.com games
      if (chessComUsername) {
        const syncResult = await gameService.syncGames();
        if (syncResult.hasMoreHistory !== undefined) {
          setHasMoreHistory(syncResult.hasMoreHistory);
        }
      }

      // Reload games after sync
      setCurrentPage(1);
      setSelectedTags(new Set());
      await loadStoredGames(1, []);
      await loadAllTags([]);
      await loadUnanalyzedCount();
      await loadAnalyzedCount();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sync games');
    } finally {
      setSyncing(false);
    }
  };

  const sortedTags = Array.from(allTags.entries())
    .filter(([tag]) => isGameTag(tag))
    .sort((a, b) => b[1] - a[1])  // Sort by count descending
    .map(([tag]) => tag);

  const totalPages = Math.ceil(totalGames / GAMES_PER_PAGE);
  const startIndex = (currentPage - 1) * GAMES_PER_PAGE;
  const endIndex = Math.min(startIndex + GAMES_PER_PAGE, totalGames);

  const goToPage = (page: number) => {
    const newPage = Math.max(1, Math.min(page, totalPages));
    if (newPage !== currentPage) {
      setCurrentPage(newPage);
    }
  };

  const toggleTag = (tag: string) => {
    let newSelectedTags: string[];

    if (selectedTags.has(tag)) {
      newSelectedTags = Array.from(selectedTags).filter(t => t !== tag);
      setSelectedTags(new Set(newSelectedTags));
    } else {
      newSelectedTags = [...Array.from(selectedTags), tag];
      setSelectedTags(new Set(newSelectedTags));
    }

    setCurrentPage(1);
    loadAllTags(newSelectedTags);
  };

  const clearTags = () => {
    setSelectedTags(new Set());
    setCurrentPage(1);
    loadAllTags([]);
  };

  // Queue all unanalyzed games for server-side analysis (AWS Batch)
  const serverAnalyzeGames = async () => {
    setServerAnalyzing(true);
    setServerResult(null);
    try {
      const result = await gameService.analyzeServerAll();
      setServerResult(result);
      // Clear success message after 10 seconds
      setTimeout(() => setServerResult(null), 10000);
    } catch (err) {
      setServerResult({ queued: 0, message: err instanceof Error ? err.message : 'Failed to queue games' });
      setTimeout(() => setServerResult(null), 5000);
    } finally {
      setServerAnalyzing(false);
    }
  };


  // Load more history (backfill older games)
  const loadMoreHistory = async () => {
    setBackfilling(true);
    setBackfillResult(null);
    try {
      const result = await gameService.backfillGames();
      setBackfillResult(result);
      setHasMoreHistory(result.hasMoreHistory);

      // Reload games list and counts
      await loadStoredGames(currentPage, selectedTagsArray);
      await loadAllTags(selectedTagsArray);
      await loadUnanalyzedCount();
      await loadAnalyzedCount();

      // Clear result message after 10 seconds
      setTimeout(() => setBackfillResult(null), 10000);
    } catch (err) {
      setBackfillResult({ synced: 0, total: 0, oldestSyncedMonth: '', hasMoreHistory: false, message: err instanceof Error ? err.message : 'Failed to load history' });
      setTimeout(() => setBackfillResult(null), 5000);
    } finally {
      setBackfilling(false);
    }
  };

  // Show link account prompt if no accounts linked at all
  if (!hasAnyLinkedAccount) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-white">My Games</h1>
          <p className="text-muted text-sm mt-1">
            Analyze your games to discover patterns and tag notable moments
          </p>
        </div>

        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-3xl">&#9823;</span>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">Link your Chess.com account</h2>
          <p className="text-muted mb-6">
            Connect your Chess.com account to sync and analyze your games.
          </p>
          <Link
            to={user ? `/u/${user.username}?settings=true` : '/'}
            className="btn btn-primary"
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
        <p className="text-muted text-sm mt-1">
          Analyze your games to discover patterns and tag notable moments
        </p>
      </div>

      {/* Action buttons + stats */}
      <div className="flex flex-wrap items-center gap-3">
        <button
          onClick={syncAllGames}
          disabled={syncing || loading}
          className="px-3 py-1.5 text-sm font-medium rounded-lg border border-slate-700 text-slate-300 hover:text-white hover:border-slate-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
        >
          {syncing ? (
            <>
              <span className="w-4 h-4 border-2 border-slate-600 border-t-sky-400 rounded-full animate-spin"></span>
              Syncing...
            </>
          ) : (
            <>
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              Sync Games
            </>
          )}
        </button>

        {chessComUsername && totalGames > 0 && hasMoreHistory && (
          <button
            onClick={loadMoreHistory}
            disabled={backfilling || syncing || loading}
            className="px-3 py-1.5 text-sm font-medium rounded-lg border border-slate-700 text-slate-300 hover:text-white hover:border-slate-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
          >
            {backfilling ? (
              <>
                <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                Loading older games...
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Load More History
              </>
            )}
          </button>
        )}

        {totalUnanalyzed > 0 && (
          <button
            onClick={serverAnalyzeGames}
            disabled={serverAnalyzing || syncing || loading}
            title="Queue for server analysis - no need to keep tab open"
            className="btn btn-primary px-3 py-1.5 text-sm disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {serverAnalyzing ? (
              <>
                <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                Queueing...
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
                </svg>
                Analyze ({totalUnanalyzed})
              </>
            )}
          </button>
        )}

        {chessComUsername && totalGames > 0 && !hasMoreHistory && (
          <span className="flex items-center gap-2 text-sm text-dim">
            <svg className="w-4 h-4 text-good" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
            All history loaded
          </span>
        )}

        {!chessComUsername && (
          <Link
            to={user ? `/u/${user.username}?settings=true` : '/'}
            className="text-sm text-accent hover:text-accent-bright"
          >
            + Link Chess.com account
          </Link>
        )}
      </div>

      {/* Feedback messages */}
      {error && (
        <p className="text-red-500 text-sm">{error}</p>
      )}
      {backfillResult && (
        <div className={`px-4 py-2 rounded-lg text-sm flex items-center gap-2 ${
          backfillResult.synced > 0
            ? 'bg-violet-500/20 border border-violet-500/30 text-violet-300'
            : backfillResult.message
            ? 'bg-red-500/20 border border-red-500/30 text-red-300'
            : 'bg-slate-500/20 border border-slate-500/30 text-slate-300'
        }`}>
          {backfillResult.synced > 0 ? (
            <>
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
              Loaded {backfillResult.synced} older games
            </>
          ) : (
            backfillResult.message || 'No additional games found'
          )}
        </div>
      )}
      {serverResult && (
        <div className={`px-4 py-2 rounded-lg text-sm flex items-center gap-2 ${
          serverResult.queued > 0
            ? 'bg-cyan-500/20 border border-cyan-500/30 text-cyan-300'
            : 'bg-red-500/20 border border-red-500/30 text-red-300'
        }`}>
          {serverResult.queued > 0 ? (
            <>
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
              Queued {serverResult.queued} games for analysis. Results will appear automatically.
            </>
          ) : (
            serverResult.message
          )}
        </div>
      )}

      {/* Games Content */}
      {(games.length > 0 || totalGames > 0) && (
        <>
          {/* Stats */}
          <p className="text-muted text-sm">
            {totalGames} {totalGames === 1 ? 'game' : 'games'}
            {totalAnalyzed > 0 && (
              <span className="ml-2 text-good">
                ({totalAnalyzed} analyzed)
              </span>
            )}
          </p>

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
                      className={`px-2 py-1 text-xs font-medium rounded-md border transition-colors ${
                        isSelected
                          ? 'bg-sky-500/20 text-white border-sky-400/50'
                          : 'bg-transparent text-slate-300 border-slate-700 hover:border-slate-500'
                      }`}
                    >
                      {tagDisplayName(tag)} ({allTags.get(tag)})
                    </button>
                  );
                })}
              </div>
              {selectedTags.size > 0 && (
                <button
                  onClick={clearTags}
                  className="text-sm text-muted hover:text-white transition-colors"
                >
                  Clear filters
                </button>
              )}
            </div>
          )}

          {/* Pagination Info */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between text-sm text-muted">
              <span>
                Showing {startIndex + 1}-{endIndex} of {totalGames}
              </span>
              <span>Page {currentPage} of {totalPages}</span>
            </div>
          )}

          {/* Games List */}
          <div className="space-y-3">
            {games.map(game => (
              <Link
                key={game.id}
                to={`/games/${game.id}`}
                className="card block p-4 hover:border-sky-400/50 transition-all duration-200 group"
              >
                <div className="flex items-start gap-3 sm:gap-5">
                  {/* Mini Board — hidden on very small screens to prevent overflow */}
                  <div className="flex-shrink-0 hidden sm:block">
                    <MiniChessBoard
                      moves={game.moves}
                      orientation={game.userColor}
                      size={200}
                    />
                  </div>

                  {/* Game Info */}
                  <div className="flex-1 min-w-0 py-1">
                    {/* Top row: Opponent + Result */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-3">
                        <h3 className="text-lg font-semibold text-white group-hover:text-accent transition-colors">
                          vs {game.opponent}
                        </h3>
                        {game.opponentRating && (
                          <span className="px-2 py-0.5 bg-slate-800 rounded text-sm text-muted font-medium">
                            {game.opponentRating}
                          </span>
                        )}
                        {/* Platform indicator */}
                        <span className={`w-5 h-5 rounded flex items-center justify-center text-[10px] font-bold ${
                          game.source === 'chess_com'
                            ? 'bg-green-600 text-white'
                            : 'bg-white text-black'
                        }`}>
                          {game.source === 'chess_com' ? 'C' : 'L'}
                        </span>
                      </div>
                      <div className="flex items-center gap-3">
                        <span className={`px-3 py-1 rounded-full text-sm font-semibold ${
                          game.result === 'W'
                            ? 'bg-green-500/20 text-green-400'
                            : game.result === 'L'
                            ? 'bg-red-500/20 text-red-400'
                            : 'bg-slate-500/20 text-muted'
                        }`}>
                          {resultLabels[game.result]}
                        </span>
                        <svg
                          className="w-5 h-5 text-dim group-hover:text-accent transition-colors"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                        </svg>
                      </div>
                    </div>

                    {/* Meta info row */}
                    <div className="flex items-center gap-4 text-sm mb-3">
                      {game.timeControl && (() => {
                        const gameType = getGameType(game.timeControl);
                        const config = gameTypeConfig[gameType];
                        return (
                          <span className={`flex items-center gap-1.5 ${config.color}`}>
                            {config.icon}
                            <span className="font-medium">{config.label}</span>
                          </span>
                        );
                      })()}
                      <span className="flex items-center gap-1.5 text-muted">
                        <svg className="w-4 h-4 text-dim" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
                        </svg>
                        {Math.ceil(game.moves.length / 2)} moves
                      </span>
                      <span className="text-dim">
                        as <span className="text-slate-300 capitalize">{game.userColor}</span>
                      </span>
                      {/* Analysis accuracy badge */}
                      {game.hasAnalysis && (
                        <span className="flex items-center gap-1.5 px-2 py-0.5 bg-purple-500/20 rounded text-purple-300 text-xs font-medium">
                          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                          </svg>
                          {Math.round(game.userColor === 'white' ? game.whiteAccuracy! : game.blackAccuracy!)}%
                        </span>
                      )}
                    </div>

                    {/* Tags */}
                    {game.tags.filter(isGameTag).length > 0 && (
                      <div className="flex flex-wrap gap-2">
                        {game.tags.filter(isGameTag).map(tag => (
                          <span
                            key={tag}
                            className="px-2.5 py-1 bg-gradient-to-r from-amber-500/10 to-orange-500/10 border border-amber-500/30 rounded-full text-xs font-medium text-amber-400"
                          >
                            {tagDisplayName(tag)}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </Link>
            ))}

            {games.length === 0 && (
              <div className="text-center py-12 text-dim">
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
              <span className="px-4 py-2 text-muted">
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
      {totalGames === 0 && !error && (
        <div className="text-center py-12 text-dim">
          No games found. Click a sync button above to download your games.
        </div>
      )}
    </div>
  );
}
