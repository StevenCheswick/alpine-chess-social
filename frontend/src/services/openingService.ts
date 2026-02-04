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
  children: TreeNode[];
}

export interface OpeningTreeResponse {
  color: 'white' | 'black';
  rootNode: TreeNode;
  totalGames: number;
  depth: number;
}

export const openingService = {
  /**
   * Get opening tree for a specific color.
   */
  async getOpeningTree(color: 'white' | 'black'): Promise<OpeningTreeResponse> {
    return api.get<OpeningTreeResponse>(`/api/opening-tree?color=${color}`);
  },
};

export default openingService;
