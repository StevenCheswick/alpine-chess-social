import { Chessboard } from 'react-chessboard';
import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { Chess } from 'chess.js';

/**
 * Clean a move string by removing move numbers (e.g., "1.", "1...") and annotations (e.g., "!", "?", "!!", "??", "!?", "?!")
 * Also handles result strings that shouldn't be played as moves.
 */
function cleanMove(move: string): string | null {
  const cleaned = move.replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
  // Skip result strings
  if (!cleaned || cleaned === '1-0' || cleaned === '0-1' || cleaned === '1/2-1/2') {
    return null;
  }
  return cleaned;
}

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
  /** Game result: 'W' = user won, 'L' = user lost, 'D' = draw */
  gameResult?: 'W' | 'L' | 'D';
  /** External control of move index - when set, navigates to this move */
  externalMoveIndex?: number;
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
  gameResult: _gameResult,
  externalMoveIndex,
}: ChessBoardProps) {
  // Clean moves once - removes move numbers, annotations, and result strings
  const cleanedMoves = useMemo(() => {
    return moves.map(cleanMove).filter((m): m is string => m !== null);
  }, [moves]);

  const [currentPosition, setCurrentPosition] = useState<string>(
    fen || 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1'
  );
  const [currentMoveIndex, setCurrentMoveIndex] = useState(startIndex);
  const [lastMove, setLastMove] = useState<{ from: string; to: string } | null>(null);
  const [boardWidth, setBoardWidth] = useState(400);
  const [isReady, setIsReady] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const chessRef = useRef<Chess>(new Chess());

  // Initialize position
  useEffect(() => {
    const chess = new Chess();

    // If FEN provided, use it directly
    if (fen && cleanedMoves.length === 0) {
      try {
        chess.load(fen);
        setCurrentPosition(fen);
      } catch {
        setCurrentPosition(chess.fen());
      }
    }
    // If moves provided, replay to startIndex
    else if (cleanedMoves.length > 0) {
      chess.reset();
      let lastMoveData: { from: string; to: string } | null = null;
      for (let i = 0; i < startIndex && i < cleanedMoves.length; i++) {
        try {
          const move = chess.move(cleanedMoves[i]);
          if (move) {
            lastMoveData = { from: move.from, to: move.to };
          }
        } catch {
          break;
        }
      }
      setCurrentPosition(chess.fen());
      setCurrentMoveIndex(startIndex);
      setLastMove(lastMoveData);
    }

    chessRef.current = chess;
  }, [fen, cleanedMoves, startIndex]);

  // Measure container width
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let dimensionsSet = false;

    const updateWidth = () => {
      const width = container.offsetWidth;
      if (width > 0) {
        setBoardWidth(width);
        if (!dimensionsSet) {
          dimensionsSet = true;
          // Wait for browser to paint before showing board
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              setIsReady(true);
            });
          });
        }
      }
    };

    // Initial measurement with small delay
    const timeout = setTimeout(updateWidth, 50);

    const resizeObserver = new ResizeObserver(updateWidth);
    resizeObserver.observe(container);

    return () => {
      clearTimeout(timeout);
      resizeObserver.disconnect();
    };
  }, []);

  const goToMove = useCallback((targetIndex: number) => {
    if (targetIndex < 0 || targetIndex > cleanedMoves.length) return;
    if (targetIndex === currentMoveIndex) return;

    const chess = chessRef.current;

    // Always replay from beginning to ensure consistent state and get last move info
    chess.reset();
    let lastMoveData: { from: string; to: string } | null = null;
    
    for (let i = 0; i < targetIndex; i++) {
      const move = chess.move(cleanedMoves[i]);
      if (!move) break;
      lastMoveData = { from: move.from, to: move.to };
    }

    const newFen = chess.fen();
    setCurrentPosition(newFen);
    setCurrentMoveIndex(targetIndex);
    setLastMove(targetIndex === 0 ? null : lastMoveData);
    onPositionChange?.(newFen, targetIndex);
  }, [cleanedMoves, currentMoveIndex, onPositionChange]);

  const goToStart = () => goToMove(0);
  const goToPrevious = () => goToMove(Math.max(0, currentMoveIndex - 1));
  const goToNext = () => goToMove(Math.min(cleanedMoves.length, currentMoveIndex + 1));
  const goToEnd = () => goToMove(cleanedMoves.length);

  const canGoPrevious = currentMoveIndex > 0;
  const canGoNext = currentMoveIndex < cleanedMoves.length;

  // Handle external move index changes
  useEffect(() => {
    if (externalMoveIndex !== undefined && externalMoveIndex !== currentMoveIndex) {
      goToMove(externalMoveIndex);
    }
  }, [externalMoveIndex]);

  // Highlight squares for last move
  const customSquareStyles = useMemo(() => {
    if (!lastMove) return {};
    const highlightColor = 'rgba(255, 255, 0, 0.4)';
    const styles = {
      [lastMove.from]: { backgroundColor: highlightColor },
      [lastMove.to]: { backgroundColor: highlightColor },
    };
    console.log('Square styles:', styles, 'lastMove:', lastMove);
    return styles;
  }, [lastMove]);

  // Determine top/bottom players based on orientation
  const topPlayer = orientation === 'white' ? blackPlayer : whitePlayer;
  const bottomPlayer = orientation === 'white' ? whitePlayer : blackPlayer;
  const topIsBlack = orientation === 'white';

  return (
    <div className={`w-full ${className}`}>
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
          {isReady && boardWidth > 0 && (
            <Chessboard
              key={`board-${orientation}`}
              options={{
                position: currentPosition,
                boardOrientation: orientation,
                squareStyles: customSquareStyles,
                animationDurationInMs: 200,
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
        {showControls && cleanedMoves.length > 0 && (
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
                className="flex-1 p-3 text-emerald-400 hover:text-emerald-300 hover:bg-slate-800 transition-colors border-l border-slate-800 flex items-center justify-center"
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
