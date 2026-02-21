import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess, type Square } from 'chess.js';

export type PuzzleStatus = 'solving' | 'solved' | 'failed';

interface PuzzleBoardProps {
  fen: string;
  solutionMoves: string[]; // UCI moves like "e2e4", "e7e8q"
  onStatusChange?: (status: PuzzleStatus) => void;
  showSolution?: boolean;
  retryKey?: number; // increment to reset the puzzle
}

function uciToMove(uci: string): { from: string; to: string; promotion?: string } {
  return {
    from: uci.slice(0, 2),
    to: uci.slice(2, 4),
    promotion: uci.length === 5 ? uci[4] : undefined,
  };
}

export function PuzzleBoard({ fen, solutionMoves, onStatusChange, showSolution, retryKey }: PuzzleBoardProps) {
  const [game, setGame] = useState(() => new Chess(fen));
  const [moveIndex, setMoveIndex] = useState(0);
  const [status, setStatus] = useState<PuzzleStatus>('solving');
  const [lastMove, setLastMove] = useState<{ from: string; to: string } | null>(null);
  const [feedbackSquare, setFeedbackSquare] = useState<{ square: string; color: string } | null>(null);
  const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
  const feedbackTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Solver's color is OPPOSITE of side to move in the initial FEN.
  // The FEN is the position before the opponent's blunder (moves[0]).
  // After the blunder auto-plays, it's the solver's turn.
  const solverColor = useMemo((): 'white' | 'black' => {
    const parts = fen.split(' ');
    return parts[1] === 'w' ? 'black' : 'white';
  }, [fen]);

  // Reset when puzzle changes or retry is triggered.
  // Auto-play the opponent's blunder (moves[0]) after a short delay.
  useEffect(() => {
    const newGame = new Chess(fen);
    setGame(newGame);
    setMoveIndex(0);
    setStatus('solving');
    setLastMove(null);
    setFeedbackSquare(null);
    setSelectedSquare(null);

    // Auto-play opponent's blunder (first move) after a brief delay
    if (solutionMoves.length > 0) {
      const timer = setTimeout(() => {
        const g = new Chess(fen);
        const move = uciToMove(solutionMoves[0]);
        try {
          g.move({ from: move.from as Square, to: move.to as Square, promotion: move.promotion });
        } catch {
          // Invalid move
        }
        setGame(g);
        setLastMove({ from: move.from, to: move.to });
        setMoveIndex(1);
      }, 600);
      return () => clearTimeout(timer);
    }
  }, [fen, solutionMoves, retryKey]);

  const updateStatus = useCallback(
    (s: PuzzleStatus) => {
      setStatus(s);
      onStatusChange?.(s);
    },
    [onStatusChange],
  );

  // After solver's correct move, auto-play opponent's reply
  const playOpponentReply = useCallback(
    (nextIndex: number) => {
      if (nextIndex >= solutionMoves.length) {
        updateStatus('solved');
        return;
      }

      // Next move is opponent's reply
      setTimeout(() => {
        setGame(prev => {
          const copy = new Chess(prev.fen());
          const move = uciToMove(solutionMoves[nextIndex]);
          try {
            copy.move({ from: move.from as Square, to: move.to as Square, promotion: move.promotion });
          } catch {
            // Invalid move — shouldn't happen in correct puzzle data
          }
          return copy;
        });
        setLastMove(uciToMove(solutionMoves[nextIndex]));
        setMoveIndex(nextIndex + 1);

        // Check if that was the last move
        if (nextIndex + 1 >= solutionMoves.length) {
          updateStatus('solved');
        }
      }, 400);
    },
    [solutionMoves, updateStatus],
  );

  // Shared move attempt logic for both drag-drop and click-to-move
  const tryMove = useCallback(
    (sourceSquare: string, targetSquare: string, pieceType: string): boolean => {
      if (status !== 'solving') return false;
      if (moveIndex >= solutionMoves.length) return false;

      const expectedUci = solutionMoves[moveIndex];
      const expected = uciToMove(expectedUci);

      // Determine promotion: if expected move has promotion, use it
      const promotion = expected.promotion;

      // Build the UCI string for the attempted move
      let attemptedUci = sourceSquare + targetSquare;
      if (promotion) {
        attemptedUci += promotion;
      } else if (
        pieceType.toLowerCase().includes('p') &&
        (targetSquare[1] === '8' || targetSquare[1] === '1')
      ) {
        attemptedUci += 'q';
      }

      if (attemptedUci === expectedUci) {
        // Correct move
        const copy = new Chess(game.fen());
        try {
          copy.move({
            from: sourceSquare as Square,
            to: targetSquare as Square,
            promotion: promotion || (
              pieceType.toLowerCase().includes('p') && (targetSquare[1] === '8' || targetSquare[1] === '1')
                ? 'q'
                : undefined
            ),
          });
        } catch {
          return false;
        }

        setGame(copy);
        setLastMove({ from: sourceSquare, to: targetSquare });
        setFeedbackSquare({ square: targetSquare, color: 'rgba(0, 200, 83, 0.5)' });
        setSelectedSquare(null);

        clearTimeout(feedbackTimeout.current);
        feedbackTimeout.current = setTimeout(() => setFeedbackSquare(null), 600);

        const nextIndex = moveIndex + 1;
        setMoveIndex(nextIndex);

        // Play opponent's reply (if any)
        playOpponentReply(nextIndex);

        return true;
      } else {
        // Wrong move
        setFeedbackSquare({ square: targetSquare, color: 'rgba(220, 38, 38, 0.5)' });
        setSelectedSquare(null);
        clearTimeout(feedbackTimeout.current);
        feedbackTimeout.current = setTimeout(() => setFeedbackSquare(null), 800);
        updateStatus('failed');
        return false;
      }
    },
    [game, moveIndex, solutionMoves, status, playOpponentReply, updateStatus],
  );

  const handlePieceDrop = useCallback(
    ({ piece, sourceSquare, targetSquare }: {
      piece: { isSparePiece: boolean; position: string; pieceType: string };
      sourceSquare: string;
      targetSquare: string | null;
    }): boolean => {
      if (!targetSquare) return false;
      setSelectedSquare(null);
      return tryMove(sourceSquare, targetSquare, piece.pieceType);
    },
    [tryMove],
  );

  const canDragPiece = useCallback(
    ({ piece }: { isSparePiece: boolean; piece: { pieceType: string }; square: string | null }) => {
      if (status !== 'solving') return false;
      // pieceType is like "wP", "bN" etc.
      const isWhitePiece = piece.pieceType[0] === 'w';
      return solverColor === 'white' ? isWhitePiece : !isWhitePiece;
    },
    [solverColor, status],
  );

  // Check if a piece belongs to the solver
  const isSolverPiece = useCallback(
    (pieceType: string) => {
      const isWhitePiece = pieceType[0] === 'w';
      return solverColor === 'white' ? isWhitePiece : !isWhitePiece;
    },
    [solverColor],
  );

  // Compute legal move squares for the selected piece
  const legalMoveSquares = useMemo(() => {
    if (!selectedSquare) return new Set<string>();
    const moves = game.moves({ square: selectedSquare as Square, verbose: true });
    return new Set(moves.map(m => m.to));
  }, [selectedSquare, game]);

  // Click-to-move: click a piece to select it, then click a square to move
  const handlePieceClick = useCallback(
    ({ piece, square }: { isSparePiece: boolean; piece: { pieceType: string }; square: string | null }) => {
      if (status !== 'solving' || !square) return;

      // If clicking a solver's piece, always select/switch to it
      if (isSolverPiece(piece.pieceType)) {
        setSelectedSquare(prev => prev === square ? null : square);
        return;
      }

      // A piece is already selected and clicked an opponent's piece — try to capture
      // Only attempt if it's a legal move; otherwise just deselect (misclick)
      if (selectedSquare && selectedSquare !== square && legalMoveSquares.has(square)) {
        const selectedPiece = game.get(selectedSquare as Square);
        if (selectedPiece) {
          tryMove(selectedSquare, square, selectedPiece.type);
        }
      } else {
        setSelectedSquare(null);
      }
    },
    [status, selectedSquare, game, isSolverPiece, tryMove, legalMoveSquares],
  );

  const handleSquareClick = useCallback(
    ({ square }: { piece: { pieceType: string } | null; square: string }) => {
      if (status !== 'solving' || !selectedSquare) return;
      if (square === selectedSquare) {
        // Deselect
        setSelectedSquare(null);
        return;
      }

      // Check if clicked square has a solver piece — if so, switch selection
      const pieceOnSquare = game.get(square as Square);
      if (pieceOnSquare) {
        const pt = (pieceOnSquare.color === 'w' ? 'w' : 'b') + pieceOnSquare.type.toUpperCase();
        if (isSolverPiece(pt)) {
          setSelectedSquare(square);
          return;
        }
      }

      // Only try to move if it's a legal destination; otherwise just deselect (misclick)
      if (!legalMoveSquares.has(square)) {
        setSelectedSquare(null);
        return;
      }

      const selectedPiece = game.get(selectedSquare as Square);
      if (selectedPiece) {
        tryMove(selectedSquare, square, selectedPiece.type);
      }
    },
    [status, selectedSquare, game, isSolverPiece, tryMove, legalMoveSquares],
  );

  // Build square styles
  const squareStyles: Record<string, React.CSSProperties> = {};

  if (lastMove) {
    squareStyles[lastMove.from] = { backgroundColor: 'rgba(255, 255, 0, 0.3)' };
    squareStyles[lastMove.to] = { backgroundColor: 'rgba(255, 255, 0, 0.3)' };
  }

  // Selected piece highlight
  if (selectedSquare) {
    squareStyles[selectedSquare] = { backgroundColor: 'rgba(20, 85, 200, 0.5)' };

    // Legal move indicators
    for (const sq of legalMoveSquares) {
      const hasPiece = game.get(sq as Square);
      if (hasPiece) {
        // Capture: ring border visible even under piece images
        squareStyles[sq] = {
          boxShadow: 'inset 0 0 0 4px rgba(20, 85, 200, 0.6)',
        };
      } else {
        // Empty square: centered dot
        squareStyles[sq] = {
          background: 'radial-gradient(circle, rgba(20, 85, 200, 0.4) 24%, transparent 24%)',
        };
      }
    }
  }

  if (feedbackSquare) {
    squareStyles[feedbackSquare.square] = {
      backgroundColor: feedbackSquare.color,
    };
  }

  // Show solution arrows
  const arrows: { startSquare: string; endSquare: string; color: string }[] = [];
  if (showSolution && moveIndex < solutionMoves.length) {
    for (let i = moveIndex; i < solutionMoves.length; i++) {
      const m = uciToMove(solutionMoves[i]);
      arrows.push({
        startSquare: m.from,
        endSquare: m.to,
        color: i === moveIndex ? 'rgb(0, 200, 83)' : 'rgba(0, 200, 83, 0.4)',
      });
    }
  }

  const movesRemaining = Math.ceil((solutionMoves.length - moveIndex) / 2);

  return (
    <div className="flex flex-col items-center gap-3">
      <div className="w-full max-w-[560px] aspect-square">
        <Chessboard
          options={{
            position: game.fen(),
            boardOrientation: solverColor,
            onPieceDrop: handlePieceDrop,
            onPieceClick: handlePieceClick,
            onSquareClick: handleSquareClick,
            canDragPiece,
            squareStyles,
            arrows,
            animationDurationInMs: 200,
          }}
        />
      </div>

      {/* Status indicator */}
      <div className="text-center">
        {status === 'solving' && (
          <p className="text-slate-300 text-sm">
            {moveIndex === 0
              ? 'Find the best move'
              : `${movesRemaining} move${movesRemaining !== 1 ? 's' : ''} to find`}
          </p>
        )}
        {status === 'solved' && (
          <p className="text-emerald-400 font-semibold">Puzzle solved!</p>
        )}
        {status === 'failed' && (
          <p className="text-red-400 font-semibold">Incorrect — puzzle failed</p>
        )}
      </div>
    </div>
  );
}
