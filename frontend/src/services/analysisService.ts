/**
 * Game Analysis Service
 *
 * Provides full game analysis using Stockfish WASM.
 *
 * For each position:
 * 1. Analyzes position before the move to get best move and eval
 * 2. Analyzes position after the move to get resulting eval
 * 3. Calculates CP loss and classifies the move
 * 4. Computes accuracy using: 100 / sqrt(1 + avg_cp_loss / 100)
 *
 * All evaluations are from WHITE's perspective.
 * Moves are in UCI format (e2e4, not e4).
 */

import type {
  GameAnalysis,
  MoveAnalysis,
  MoveClassification,
  MoveClassifications,
} from '../types/analysis';
import { Chess } from 'chess.js';

const STOCKFISH_PATH = '/stockfish/stockfish.js';

interface AnalysisResult {
  bestMove: string;
  evaluation: number; // centipawns from white's perspective
  isMate: boolean;
  mateIn: number | null;
}

/**
 * Stockfish worker wrapper for game analysis
 */
class StockfishAnalyzer {
  private worker: Worker | null = null;
  private resolveReady: (() => void) | null = null;
  private resolveAnalysis: ((result: AnalysisResult) => void) | null = null;
  private currentAnalysis: Partial<AnalysisResult> = {};
  private targetDepth: number = 18;

  async init(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.worker = new Worker(STOCKFISH_PATH);
        this.resolveReady = resolve;

        this.worker.onmessage = (e: MessageEvent<string>) => {
          this.handleMessage(e.data);
        };

        this.worker.onerror = (e) => {
          reject(new Error(`Stockfish worker error: ${e.message}`));
        };

        this.worker.postMessage('uci');
      } catch (err) {
        reject(err);
      }
    });
  }

  private handleMessage(line: string): void {
    if (line === 'uciok') {
      this.worker?.postMessage('setoption name MultiPV value 1');
      this.worker?.postMessage('isready');
    }

    if (line === 'readyok') {
      if (this.resolveReady) {
        this.resolveReady();
        this.resolveReady = null;
      }
    }

    // Parse analysis info lines
    if (line.startsWith('info depth') && line.includes(' pv ')) {
      const depth = this.parseValue(line, 'depth');
      if (depth && parseInt(depth) >= this.targetDepth - 2) {
        const scoreType = line.includes('score mate') ? 'mate' : 'cp';
        const scoreValue = parseInt(this.parseValue(line, scoreType) || '0');
        const pv = this.parsePV(line);

        if (pv.length > 0) {
          this.currentAnalysis = {
            bestMove: pv[0],
            evaluation: scoreType === 'cp' ? scoreValue : (scoreValue > 0 ? 10000 : -10000),
            isMate: scoreType === 'mate',
            mateIn: scoreType === 'mate' ? scoreValue : null,
          };
        }
      }
    }

    if (line.startsWith('bestmove')) {
      const parts = line.split(' ');
      const bestMove = parts[1];

      if (this.resolveAnalysis) {
        this.resolveAnalysis({
          bestMove: this.currentAnalysis.bestMove || bestMove,
          evaluation: this.currentAnalysis.evaluation ?? 0,
          isMate: this.currentAnalysis.isMate ?? false,
          mateIn: this.currentAnalysis.mateIn ?? null,
        });
        this.resolveAnalysis = null;
      }
    }
  }

  private parseValue(line: string, key: string): string | undefined {
    const parts = line.split(' ');
    const idx = parts.indexOf(key);
    return idx !== -1 && idx + 1 < parts.length ? parts[idx + 1] : undefined;
  }

  private parsePV(line: string): string[] {
    const parts = line.split(' ');
    const pvIdx = parts.indexOf('pv');
    return pvIdx !== -1 ? parts.slice(pvIdx + 1) : [];
  }

  async analyze(fen: string, nodes: number = 100000): Promise<AnalysisResult> {
    if (!this.worker) throw new Error('Worker not initialized');

    this.targetDepth = 10; // Lower threshold for node-based search
    this.currentAnalysis = {};

    return new Promise((resolve) => {
      this.resolveAnalysis = resolve;
      this.worker!.postMessage(`position fen ${fen}`);
      this.worker!.postMessage(`go nodes ${nodes}`);
    });
  }

  destroy(): void {
    if (this.worker) {
      this.worker.postMessage('quit');
      this.worker.terminate();
      this.worker = null;
    }
  }
}

export interface AnalysisOptions {
  /** Nodes to search (default: 100000) */
  nodes?: number;
  /** Progress callback (0-100) */
  onProgress?: (progress: number) => void;
}

/**
 * Analyze a complete game
 *
 * @param moves - Array of moves in SAN notation (e.g., ["e4", "e5", "Nf3"])
 * @param userColor - The user's color ('white' | 'black')
 * @param options - Analysis options
 * @returns Promise<GameAnalysis> - Complete analysis data
 */
export async function analyzeGame(
  moves: string[],
  userColor: 'white' | 'black',
  options: AnalysisOptions = {}
): Promise<GameAnalysis> {
  const { nodes = 100000, onProgress } = options;

  const analyzer = new StockfishAnalyzer();
  await analyzer.init();

  try {
    const chess = new Chess();
    const moveAnalyses: MoveAnalysis[] = [];

    const whiteClassifications: MoveClassifications = {
      best: 0, excellent: 0, good: 0, inaccuracy: 0, mistake: 0, blunder: 0, book: 0, forced: 0
    };
    const blackClassifications: MoveClassifications = {
      best: 0, excellent: 0, good: 0, inaccuracy: 0, mistake: 0, blunder: 0, book: 0, forced: 0
    };

    let whiteTotalCpLoss = 0;
    let blackTotalCpLoss = 0;
    let whiteMoveCount = 0;
    let blackMoveCount = 0;

    // Track book moves - consecutive moves from start with minimal cp loss
    const BOOK_CP_THRESHOLD = 20; // Max cp loss to still be considered book
    let whiteStillInBook = true;
    let blackStillInBook = true;

    // Analyze starting position first
    let prevAnalysis = await analyzer.analyze(chess.fen(), nodes);
    // Convert to white's perspective (white to move at start)
    let prevEvalWhitePerspective = prevAnalysis.evaluation;

    for (let i = 0; i < moves.length; i++) {
      const isWhiteMove = i % 2 === 0;
      const sanMove = moves[i];

      // Get best move and eval from previous analysis (position before this move)
      const bestMove = prevAnalysis.bestMove;
      const bestEval = prevEvalWhitePerspective;

      // Check for forced move (only one legal move)
      const legalMoves = chess.moves({ verbose: true });
      const isForced = legalMoves.length === 1;

      // Make the move to get UCI notation
      const moveResult = chess.move(sanMove);
      if (!moveResult) {
        throw new Error(`Invalid move: ${sanMove} at position ${i}`);
      }
      const uciMove = moveResult.from + moveResult.to + (moveResult.promotion || '');

      // Analyze position after the move
      const analysisAfter = await analyzer.analyze(chess.fen(), nodes);

      // Convert to white's perspective
      // After white moves, it's black to move, so flip. After black moves, it's white to move, no flip.
      const moveEval = isWhiteMove ? -analysisAfter.evaluation : analysisAfter.evaluation;

      // Calculate CP loss (always positive)
      // White wants higher eval, black wants lower eval
      let cpLoss: number;
      if (isWhiteMove) {
        cpLoss = Math.max(0, bestEval - moveEval);
      } else {
        cpLoss = Math.max(0, moveEval - bestEval);
      }

      // Check if player found the best move
      if (uciMove === bestMove) {
        cpLoss = 0;
      }

      // Classify the move
      let classification: MoveClassification;

      if (isForced) {
        classification = 'forced';
      } else if (isWhiteMove && whiteStillInBook && cpLoss <= BOOK_CP_THRESHOLD) {
        classification = 'book';
      } else if (!isWhiteMove && blackStillInBook && cpLoss <= BOOK_CP_THRESHOLD) {
        classification = 'book';
      } else {
        classification = classifyMove(cpLoss);
        // Once out of book, stay out of book
        if (isWhiteMove) {
          whiteStillInBook = false;
        } else {
          blackStillInBook = false;
        }
      }

      moveAnalyses.push({
        move: uciMove,
        move_eval: Math.round(moveEval),
        best_move: bestMove,
        best_eval: Math.round(bestEval),
        cp_loss: Math.round(cpLoss),
        classification,
      });

      // Update counts (skip book/forced for accuracy calc)
      if (classification !== 'book' && classification !== 'forced') {
        if (isWhiteMove) {
          whiteClassifications[classification]++;
          whiteTotalCpLoss += cpLoss;
          whiteMoveCount++;
        } else {
          blackClassifications[classification]++;
          blackTotalCpLoss += cpLoss;
          blackMoveCount++;
        }
      } else {
        if (isWhiteMove) {
          whiteClassifications[classification]++;
        } else {
          blackClassifications[classification]++;
        }
      }

      // Store this analysis for the next iteration
      prevAnalysis = analysisAfter;
      prevEvalWhitePerspective = moveEval;

      // Report progress
      if (onProgress) {
        onProgress(Math.round(((i + 1) / moves.length) * 100));
      }
    }

    const whiteAvgCpLoss = whiteMoveCount > 0 ? whiteTotalCpLoss / whiteMoveCount : 0;
    const blackAvgCpLoss = blackMoveCount > 0 ? blackTotalCpLoss / blackMoveCount : 0;

    return {
      white_accuracy: calculateAccuracy(whiteAvgCpLoss),
      black_accuracy: calculateAccuracy(blackAvgCpLoss),
      white_avg_cp_loss: whiteAvgCpLoss,
      black_avg_cp_loss: blackAvgCpLoss,
      white_classifications: whiteClassifications,
      black_classifications: blackClassifications,
      moves: moveAnalyses,
      isComplete: true,
    };
  } finally {
    analyzer.destroy();
  }
}

/**
 * Calculate accuracy from average centipawn loss
 * Formula: 100 / sqrt(1 + ACPL / 100)
 */
export function calculateAccuracy(avgCpLoss: number): number {
  return 100 / Math.sqrt(1 + avgCpLoss / 100);
}

/**
 * Classify a move based on centipawn loss (Chess.com-style thresholds)
 */
export function classifyMove(cpLoss: number, isForced: boolean = false): MoveClassification {
  if (isForced) return 'forced';
  if (cpLoss === 0) return 'best';
  if (cpLoss < 10) return 'excellent';
  if (cpLoss < 50) return 'good';
  if (cpLoss < 100) return 'inaccuracy';
  if (cpLoss < 200) return 'mistake';
  return 'blunder';
}

