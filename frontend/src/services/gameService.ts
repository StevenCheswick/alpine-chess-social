/**
 * Game service for fetching user games.
 */

import api from './api';

export interface Game {
  id: number;
  chessComGameId: string;
  opponent: string;
  opponentRating: number | null;
  userRating: number | null;
  result: string;
  userColor: string;
  timeControl: string | null;
  date: string | null;
  tags: string[];
  moves: string[];
}

export interface GamesResponse {
  games: Game[];
  total: number;
}

export interface SyncResponse {
  username: string;
  synced: number;
  total: number;
  lastSyncedAt: string | null;
  isFirstSync: boolean;
}

export interface AnalyzeResponse {
  analyzed: number;
  remaining: number;
  total: number;
}

export interface AnalyzeServerResponse {
  queued: number;
  message: string;
}

export type ChessPlatform = 'chess_com' | 'lichess';

export const gameService = {
  /**
   * Get current user's synced games.
   */
  async getMyGames(limit: number = 50): Promise<GamesResponse> {
    return api.get<GamesResponse>(`/api/users/me/games?limit=${limit}`);
  },

  /**
   * Sync games from Chess.com (download only, no analysis).
   * First sync fetches all games, subsequent syncs only fetch new games.
   */
  async syncGames(): Promise<SyncResponse> {
    return api.post<SyncResponse>('/api/games/sync', {});
  },

  /**
   * Sync games from Lichess.
   */
  async syncLichessGames(): Promise<SyncResponse> {
    return api.post<SyncResponse>('/api/games/sync/lichess', {});
  },

  /**
   * Analyze unanalyzed games and add tags.
   * Processes in batches of 100, saving after each batch.
   */
  async analyzeGames(limit: number = 1000, platform: ChessPlatform = 'chess_com'): Promise<AnalyzeResponse> {
    return api.post<AnalyzeResponse>(`/api/games/analyze?limit=${limit}&platform=${platform}`, {});
  },

  /**
   * Queue unanalyzed games for server-side analysis via AWS Batch.
   * Games are processed in the background - no need to keep tab open.
   */
  async analyzeServerAll(limit?: number): Promise<AnalyzeServerResponse> {
    return api.post<AnalyzeServerResponse>('/api/games/analyze-server', {
      all_unanalyzed: true,
      limit: limit ?? 100,
    });
  },
};

export default gameService;
