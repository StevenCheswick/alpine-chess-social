import { useState, useEffect, useRef } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess } from 'chess.js';

interface MiniChessBoardProps {
  moves: string[];
  orientation?: 'white' | 'black';
  size?: number;
}

export function MiniChessBoard({ moves, orientation = 'white', size = 120 }: MiniChessBoardProps) {
  const [currentPosition, setCurrentPosition] = useState<string>(
    'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
  );
  const [boardWidth, setBoardWidth] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  // Initialize position - replay all moves to get final position
  useEffect(() => {
    const chess = new Chess();

    if (moves && moves.length > 0) {
      for (let i = 0; i < moves.length; i++) {
        try {
          const cleanMove = moves[i].replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
          if (cleanMove && cleanMove !== '1-0' && cleanMove !== '0-1' && cleanMove !== '1/2-1/2') {
            chess.move(cleanMove);
          }
        } catch {
          break;
        }
      }
    }

    setCurrentPosition(chess.fen());
  }, [moves]);

  // Measure container width
  useEffect(() => {
    const updateWidth = () => {
      if (containerRef.current) {
        const width = containerRef.current.offsetWidth;
        if (width > 0) {
          setBoardWidth(width);
        }
      }
    };

    updateWidth();
    const resizeObserver = new ResizeObserver(updateWidth);
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }

    return () => resizeObserver.disconnect();
  }, []);

  return (
    <div
      ref={containerRef}
      className="flex-shrink-0 bg-slate-900 rounded overflow-hidden"
      style={{ width: size, height: size }}
    >
      {boardWidth > 0 && (
        <Chessboard
          key={`mini-${orientation}`}
          position={currentPosition}
          boardOrientation={orientation}
          arePiecesDraggable={false}
          boardWidth={boardWidth}
        />
      )}
    </div>
  );
}
