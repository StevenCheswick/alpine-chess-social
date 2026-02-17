import api from './api';

export interface AccuracyDataPoint {
  date: string;
  accuracy: number;
  gameId: number;
}

export interface PhaseAccuracyDataPoint {
  date: string;
  gameId: number;
  opening: number | null;
  middlegame: number | null;
  endgame: number | null;
}

export interface FirstInaccuracyDataPoint {
  date: string;
  moveNumber: number;
  mistakeMoveNumber: number;
  blunderMoveNumber: number;
  gameId: number;
}

export interface RatingDataPoint {
  date: string;
  rating: number;
  gameId: number;
}

export interface MoveQualityBreakdown {
  best: number;
  excellent: number;
  good: number;
  inaccuracy: number;
  mistake: number;
  blunder: number;
}

export interface GameSummary {
  gameId: number;
  date: string;
  accuracy: number;
  opponent: string;
  opponentRating: number | null;
  result: string;
  userColor: string;
}

export interface OpeningBlunder {
  line: string;
  moves: string[];
  ply: number;
  color: string;
  mistakeCount: number;
  avgCpLoss: number;
  bestMove?: string;
  sampleGameId: number;
}

export interface CleanLine {
  line: string;
  moves: string[];
  color: string;
  cleanDepth: number;
  gameCount: number;
  avgCpLoss: number;
  sampleGameId: number;
}

export interface DashboardStats {
  totalAnalyzedGames: number;
  accuracyOverTime: AccuracyDataPoint[];
  phaseAccuracyOverTime: PhaseAccuracyDataPoint[];
  firstInaccuracyOverTime: FirstInaccuracyDataPoint[];
  ratingOverTime: RatingDataPoint[];
  moveQualityBreakdown: MoveQualityBreakdown;
  mostAccurateGames: GameSummary[];
  leastAccurateGames: GameSummary[];
  openingBlunders: OpeningBlunder[];
  cleanestLines: CleanLine[];
}

export async function getStats(): Promise<DashboardStats> {
  return api.get<DashboardStats>('/api/games/stats');
}
