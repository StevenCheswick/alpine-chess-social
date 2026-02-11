/**
 * Opening tree service for fetching user's opening repertoire data.
 */

import api from './api';

export interface TreeNode {
  move: string;
  fen: string;
  games: number;
  wins: number;
  losses: number;
  draws: number;
  winRate: number;
  children?: TreeNode[];
  evalCp?: number;
}

export interface OpeningTreeResponse {
  color: 'white' | 'black';
  fen: string;
  games: number;
  wins: number;
  losses: number;
  draws: number;
  winRate: number;
  children: TreeNode[];
  totalGames: number;
  depth: number;
}

export const openingService = {
  /**
   * Get children of a position for a specific color.
   * If no fen is provided, returns children of the starting position.
   */
  async getOpeningTree(color: 'white' | 'black', fen?: string): Promise<OpeningTreeResponse> {
    const params = new URLSearchParams({ color });
    if (fen) params.set('fen', fen);
    return api.get<OpeningTreeResponse>(`/api/opening-tree?${params}`);
  },
};

export default openingService;
