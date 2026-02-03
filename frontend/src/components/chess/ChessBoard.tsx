import { Chessboard } from 'react-chessboard';
import { useState, useEffect, useRef, useCallback } from 'react';
import { Chess } from 'chess.js';
import { getMoveType, playMoveSound } from '../../utils/chessSounds';

interface ChessBoardProps {
  /** Initial FEN position to display */
  fen?: string;
  /** All moves in the game (SAN notation) */
  moves?: string[];
  /** Index to start from (0-indexed). Default shows position after this many moves */
  startIndex?: number;
  /** Board orientation */
  orientation?: 'white' | 'black';
  /** White player info */
  whitePlayer?: { username: string; rating?: number };
  /** Black player info */
  blackPlayer?: { username: string; rating?: number };
  /** Link to view game on chess platform */
  gameUrl?: string;
  /** Show navigation controls */
  showControls?: boolean;
  /** Callback when position changes */
  onPositionChange?: (fen: string, moveIndex: number) => void;
  /** Additional CSS classes */
  className?: string;
}

export default function ChessBoard({
  fen,
  moves = [],
  startIndex = 0,
  orientation = 'white',
  whitePlayer,
  blackPlayer,
  gameUrl,
  showControls = true,
  onPositionChange,
  className = '',
}: ChessBoardProps) {
  const [currentPosition, setCurrentPosition] = useState<string>(
    fen || 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
  );
  const [currentMoveIndex, setCurrentMoveIndex] = useState(startIndex);
  const [boardWidth, setBoardWidth] = useState(400);
  const containerRef = useRef<HTMLDivElement>(null);
  const chessRef = useRef<Chess>(new Chess());

  // Initialize position
  useEffect(() => {
    const chess = new Chess();

    // If FEN provided, use it directly
    if (fen && moves.length === 0) {
      try {
        chess.load(fen);
        setCurrentPosition(fen);
      } catch {
        setCurrentPosition(chess.fen());
      }
    }
    // If moves provided, replay to startIndex
    else if (moves.length > 0) {
      chess.reset();
      for (let i = 0; i < startIndex && i < moves.length; i++) {
        try {
          chess.move(moves[i]);
        } catch {
          break;
        }
      }
      setCurrentPosition(chess.fen());
      setCurrentMoveIndex(startIndex);
    }

    chessRef.current = chess;
  }, [fen, moves, startIndex]);

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

  const goToMove = useCallback((targetIndex: number) => {
    if (targetIndex < 0 || targetIndex > moves.length) return;

    const chess = new Chess();
    for (let i = 0; i < targetIndex && i < moves.length; i++) {
      const move = chess.move(moves[i]);
      if (!move) break;
    }

    // Play sound for the move we just made
    if (targetIndex > 0 && targetIndex <= moves.length) {
      const tempChess = new Chess();
      for (let i = 0; i < targetIndex - 1; i++) {
        tempChess.move(moves[i]);
      }
      const lastMove = tempChess.move(moves[targetIndex - 1]);
      if (lastMove) {
        playMoveSound(getMoveType(lastMove), 0.6);
      }
    }

    chessRef.current = chess;
    const newFen = chess.fen();
    setCurrentPosition(newFen);
    setCurrentMoveIndex(targetIndex);
    onPositionChange?.(newFen, targetIndex);
  }, [moves, onPositionChange]);

  const goToStart = () => goToMove(0);
  const goToPrevious = () => goToMove(Math.max(0, currentMoveIndex - 1));
  const goToNext = () => goToMove(Math.min(moves.length, currentMoveIndex + 1));
  const goToEnd = () => goToMove(moves.length);

  const canGoPrevious = currentMoveIndex > 0;
  const canGoNext = currentMoveIndex < moves.length;

  // Determine top/bottom players based on orientation
  const topPlayer = orientation === 'white' ? blackPlayer : whitePlayer;
  const bottomPlayer = orientation === 'white' ? whitePlayer : blackPlayer;
  const topIsBlack = orientation === 'white';

  return (
    <div className={`w-full lg:max-w-sm ${className}`}>
      <div className="card overflow-hidden">
        {/* Top Player */}
        {topPlayer && (
          <div className={`flex items-center justify-between px-3 py-2 ${
            topIsBlack ? 'bg-slate-800' : 'bg-slate-700'
          }`}>
            <div className="flex items-center gap-2">
              <div className={`w-2.5 h-2.5 rounded-full ${topIsBlack ? 'bg-slate-600' : 'bg-white'}`} />
              <span className="font-medium text-sm text-white">{topPlayer.username}</span>
              {topPlayer.rating && (
                <span className="text-xs text-slate-400">({topPlayer.rating})</span>
              )}
            </div>
          </div>
        )}

        {/* Chess Board */}
        <div ref={containerRef} className="w-full aspect-square bg-slate-900">
          {boardWidth > 0 && (
            <Chessboard
              options={{
                position: currentPosition,
                boardOrientation: orientation,
                allowDragging: false,
              }}
            />
          )}
        </div>

        {/* Bottom Player */}
        {bottomPlayer && (
          <div className={`flex items-center justify-between px-3 py-2 ${
            !topIsBlack ? 'bg-slate-800' : 'bg-slate-700'
          }`}>
            <div className="flex items-center gap-2">
              <div className={`w-2.5 h-2.5 rounded-full ${!topIsBlack ? 'bg-slate-600' : 'bg-white'}`} />
              <span className="font-medium text-sm text-white">{bottomPlayer.username}</span>
              {bottomPlayer.rating && (
                <span className="text-xs text-slate-400">({bottomPlayer.rating})</span>
              )}
            </div>
          </div>
        )}

        {/* Navigation Controls */}
        {showControls && moves.length > 0 && (
          <div className="flex border-t border-slate-800">
            <button
              onClick={goToStart}
              disabled={!canGoPrevious}
              className="flex-1 p-3 text-slate-400 hover:text-white hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              aria-label="Go to start"
            >
              <svg className="w-5 h-5 mx-auto" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="6" y1="6" x2="6" y2="18" />
                <polyline points="18 6 10 12 18 18" />
              </svg>
            </button>
            <button
              onClick={goToPrevious}
              disabled={!canGoPrevious}
              className="flex-1 p-3 text-slate-400 hover:text-white hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors border-l border-slate-800"
              aria-label="Previous move"
            >
              <svg className="w-5 h-5 mx-auto" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="16 6 8 12 16 18" />
              </svg>
            </button>
            {gameUrl && (
              <a
                href={gameUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="flex-1 p-3 text-primary-400 hover:text-primary-300 hover:bg-slate-800 transition-colors border-l border-slate-800 flex items-center justify-center"
              >
                <span className="text-xs font-medium">View</span>
              </a>
            )}
            <button
              onClick={goToNext}
              disabled={!canGoNext}
              className="flex-1 p-3 text-slate-400 hover:text-white hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors border-l border-slate-800"
              aria-label="Next move"
            >
              <svg className="w-5 h-5 mx-auto" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="8 6 16 12 8 18" />
              </svg>
            </button>
            <button
              onClick={goToEnd}
              disabled={!canGoNext}
              className="flex-1 p-3 text-slate-400 hover:text-white hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors border-l border-slate-800"
              aria-label="Go to end"
            >
              <svg className="w-5 h-5 mx-auto" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="6 6 14 12 6 18" />
                <line x1="18" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
