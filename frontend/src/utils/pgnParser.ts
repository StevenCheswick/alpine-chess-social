import { Chess } from 'chess.js';

/**
 * Parses a PGN string and extracts all moves.
 */
export function parsePGNToMoves(pgnString: string): string[] | null {
  if (!pgnString) {
    return null;
  }

  try {
    const chess = new Chess();
    const loadResult = chess.loadPgn(pgnString);

    if (loadResult === null) {
      console.error('[PGN Parser] Failed to load PGN');
      return null;
    }

    return chess.history({ verbose: false });
  } catch (error) {
    console.error('[PGN Parser] Failed to parse PGN:', error);
    return null;
  }
}

/**
 * Gets the FEN position from a PGN at a specific move index.
 * @param pgnString - The full PGN string
 * @param moveIndex - The 0-indexed move to get FEN at
 */
export function getFENAtMove(pgnString: string, moveIndex: number): string | null {
  if (!pgnString) {
    return null;
  }

  try {
    const chess = new Chess();
    chess.loadPgn(pgnString);
    const history = chess.history({ verbose: false });

    // Reset and replay to the desired position
    chess.reset();
    for (let i = 0; i <= moveIndex && i < history.length; i++) {
      chess.move(history[i]);
    }

    return chess.fen();
  } catch (error) {
    console.error('[PGN Parser] Failed to get FEN:', error);
    return null;
  }
}

/**
 * Gets the starting FEN (standard chess position)
 */
export function getStartingFEN(): string {
  return 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
}

/**
 * Validates if a FEN string is valid
 */
export function isValidFEN(fen: string): boolean {
  try {
    new Chess(fen);
    return true;
  } catch {
    return false;
  }
}
