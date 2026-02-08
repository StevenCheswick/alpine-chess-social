/**
 * Game Analysis Types
 *
 * These types match the Lambda API output format for game analysis.
 * See: Chess Analyzer Lambda API Guide
 */

export type MoveClassification =
  | 'book'       // Known opening move
  | 'best'       // Best move found (0 cp loss)
  | 'excellent'  // CP loss < 10
  | 'good'       // CP loss < 50
  | 'inaccuracy' // CP loss < 100
  | 'mistake'    // CP loss < 200
  | 'blunder'    // CP loss >= 200
  | 'forced';    // Only legal move

export interface MoveClassifications {
  best: number;
  excellent: number;
  good: number;
  inaccuracy: number;
  mistake: number;
  blunder: number;
  book: number;
  forced: number;
}

export interface MoveAnalysis {
  /** The move played (UCI format, e.g., "e2e4") */
  move: string;
  /** Evaluation after the move was played (centipawns, white perspective) */
  move_eval: number;
  /** The best move in the position (UCI format) */
  best_move: string;
  /** Evaluation of the best move (centipawns, white perspective) */
  best_eval: number;
  /** Centipawn loss from playing this move vs best move */
  cp_loss: number;
  /** Classification based on cp_loss thresholds */
  classification: MoveClassification;
}

export interface GameAnalysis {
  /** White player's accuracy (0-100) */
  white_accuracy: number;
  /** Black player's accuracy (0-100) */
  black_accuracy: number;
  /** White's average centipawn loss per move */
  white_avg_cp_loss: number;
  /** Black's average centipawn loss per move */
  black_avg_cp_loss: number;
  /** Count of each classification type for white */
  white_classifications: MoveClassifications;
  /** Count of each classification type for black */
  black_classifications: MoveClassifications;
  /** Per-move analysis data */
  moves: MoveAnalysis[];
  /** Whether analysis is complete */
  isComplete: boolean;
  /** Current progress (0-100) during analysis */
  progress?: number;
}

/**
 * Classification thresholds (centipawn loss) - Chess.com-style
 */
export const CLASSIFICATION_THRESHOLDS = {
  best: 0,
  excellent: 10,
  good: 50,
  inaccuracy: 100,
  mistake: 200,
  blunder: Infinity,
} as const;

/**
 * Colors for each classification type
 */
export const CLASSIFICATION_COLORS: Record<MoveClassification, string> = {
  book: 'text-yellow-800',
  best: 'text-emerald-400',
  excellent: 'text-green-400',
  good: 'text-green-300',
  inaccuracy: 'text-yellow-400',
  mistake: 'text-orange-400',
  blunder: 'text-red-400',
  forced: 'text-slate-400',
};

export const CLASSIFICATION_BG_COLORS: Record<MoveClassification, string> = {
  book: 'bg-yellow-800/20',
  best: 'bg-emerald-500/20',
  excellent: 'bg-green-500/20',
  good: 'bg-green-500/10',
  inaccuracy: 'bg-yellow-500/20',
  mistake: 'bg-orange-500/20',
  blunder: 'bg-red-500/20',
  forced: 'bg-slate-500/20',
};

/** Progress state for batch game analysis */
export interface BatchProgress {
  gamesCompleted: number;
  gamesTotal: number;
  gamesSucceeded: number;
  gamesFailed: number;
  activeWorkers: number;
}

/** Result of analyzing a single game in a batch */
export interface BatchGameResult {
  gameId: string;
  analysis: GameAnalysis | null;
  error: string | null;
}

/** Input for a single game in a batch */
export interface BatchGameInput {
  id: string;
  moves: string[];
  userColor: 'white' | 'black';
}
