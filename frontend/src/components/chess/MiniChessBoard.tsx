import { useMemo } from 'react';
import { Chess } from 'chess.js';

interface MiniChessBoardProps {
  moves: string[];
  orientation?: 'white' | 'black';
  size?: number;
}

const pieceUnicode: Record<string, string> = {
  'K': '♔', 'Q': '♕', 'R': '♖', 'B': '♗', 'N': '♘', 'P': '♙',
  'k': '♚', 'q': '♛', 'r': '♜', 'b': '♝', 'n': '♞', 'p': '♟',
};

export function MiniChessBoard({ moves, orientation = 'white', size = 120 }: MiniChessBoardProps) {
  const board = useMemo(() => {
    const chess = new Chess();

    // Play through all moves to get final position
    for (const move of moves) {
      try {
        // Clean the move (remove move numbers, annotations)
        const cleanMove = move.replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
        if (cleanMove && cleanMove !== '1-0' && cleanMove !== '0-1' && cleanMove !== '1/2-1/2') {
          chess.move(cleanMove);
        }
      } catch {
        // Invalid move, stop here
        break;
      }
    }

    return chess.board();
  }, [moves]);

  const squareSize = size / 8;

  // Get rows in correct order based on orientation
  const rows = orientation === 'white'
    ? board
    : [...board].reverse().map(row => [...row].reverse());

  return (
    <div
      className="grid grid-cols-8 border border-slate-700 rounded overflow-hidden flex-shrink-0"
      style={{ width: size, height: size }}
    >
      {rows.map((row, rowIndex) =>
        row.map((square, colIndex) => {
          const isLight = (rowIndex + colIndex) % 2 === 0;
          const piece = square ? pieceUnicode[square.color === 'w' ? square.type.toUpperCase() : square.type.toLowerCase()] : null;

          return (
            <div
              key={`${rowIndex}-${colIndex}`}
              className={`flex items-center justify-center ${
                isLight ? 'bg-amber-100' : 'bg-amber-700'
              }`}
              style={{
                width: squareSize,
                height: squareSize,
                fontSize: squareSize * 0.75,
                lineHeight: 1,
              }}
            >
              {piece && (
                <span className={square?.color === 'w' ? 'text-slate-800' : 'text-slate-900'}>
                  {piece}
                </span>
              )}
            </div>
          );
        })
      )}
    </div>
  );
}
