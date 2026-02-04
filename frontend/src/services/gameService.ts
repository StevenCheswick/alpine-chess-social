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

export const gameService = {
  /**
   * Get current user's synced games.
   */
  async getMyGames(limit: number = 50): Promise<GamesResponse> {
    return api.get<GamesResponse>(`/api/users/me/games?limit=${limit}`);
  },
};

export default gameService;
