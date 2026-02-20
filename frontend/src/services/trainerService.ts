/**
 * Trainer service for fetching opening mistake puzzles.
 */

import api from './api';

export interface TrainerTreeMove {
  san: string;
  cp?: number;
  games?: number;
  accepted?: boolean;
  engine_best?: boolean;
  result?: TrainerPuzzleTree;
}

export interface TrainerPuzzleTree {
  fen: string;
  type: 'solver' | 'opponent' | 'cutoff' | 'terminal';
  reason?: string;
  status?: string;
  moves?: Record<string, TrainerTreeMove>;
}

export interface TrainerPuzzle {
  id: string;
  eco: string;
  opening_name: string;
  mistake_san: string;
  mistake_uci: string;
  pre_mistake_fen: string;
  solver_color: 'w' | 'b';
  root_eval: number;
  cp_loss: number;
  games: number;
  tree: TrainerPuzzleTree;
}

export interface TrainerOpening {
  opening_name: string;
  eco_codes: string[];
  puzzle_count: number;
  completed_count: number;
  sample_fen: string;
}

export const trainerService = {
  async listOpenings(): Promise<TrainerOpening[]> {
    return api.get<TrainerOpening[]>('/api/trainer/openings');
  },

  async getPuzzles(opening: string): Promise<{ puzzles: TrainerPuzzle[]; completed_ids: string[] }> {
    const params = new URLSearchParams({ opening });
    return api.get<{ puzzles: TrainerPuzzle[]; completed_ids: string[] }>(`/api/trainer/puzzles?${params}`);
  },

  async markComplete(puzzleId: string): Promise<void> {
    await api.post('/api/trainer/progress', { puzzle_id: puzzleId });
  },
};

export default trainerService;
