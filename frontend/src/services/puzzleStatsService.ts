import api from './api';

export interface PuzzleSideStats {
  found: number;
  missed: number;
  total: number;
  rate: number; // percentage
}

export interface ThemeStats {
  theme: string;
  user: PuzzleSideStats;
  opponent: PuzzleSideStats;
}

export interface PositionStats {
  position: 'winning' | 'equal' | 'losing';
  user: PuzzleSideStats;
  opponent: PuzzleSideStats;
}

export interface PuzzleStats {
  user: PuzzleSideStats;
  opponent: PuzzleSideStats;
  byTheme: ThemeStats[];
  byPosition: PositionStats[];
}

export async function getPuzzleStats(): Promise<PuzzleStats> {
  return api.get<PuzzleStats>('/api/puzzles/stats');
}
