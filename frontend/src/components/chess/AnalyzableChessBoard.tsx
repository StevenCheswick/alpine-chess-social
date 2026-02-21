import { useState, useCallback, useMemo } from 'react';
import { Chess } from 'chess.js';
import ChessBoard from './ChessBoard';
import { EvalBar } from './EvalBar';
import { EngineLines } from './EngineLines';
import { useStockfish } from '../../hooks/useStockfish';
import type { GameAnalysis } from '../../types/analysis';
import { CLASSIFICATION_COLORS } from '../../types/analysis';

function cleanMove(move: string): string | null {
  const cleaned = move.replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
  if (!cleaned || cleaned === '1-0' || cleaned === '0-1' || cleaned === '1/2-1/2') {
    return null;
  }
  return cleaned;
}

function uciToSan(fen: string, uci: string): string {
  try {
    const chess = new Chess(fen);
    const from = uci.slice(0, 2);
    const to = uci.slice(2, 4);
    const promotion = uci.length > 4 ? uci[4] : undefined;
    const move = chess.move({ from, to, promotion });
    return move ? move.san : uci;
  } catch {
    return uci;
  }
}

interface AnalyzableChessBoardProps {
  fen?: string;
  moves?: string[];
  startIndex?: number;
  orientation?: 'white' | 'black';
  whitePlayer?: { username: string; rating?: number };
  blackPlayer?: { username: string; rating?: number };
  gameUrl?: string;
  showControls?: boolean;
  onPositionChange?: (fen: string, moveIndex: number) => void;
  className?: string;
  gameResult?: 'W' | 'L' | 'D';
  showAnalysis?: boolean;
  multiPv?: number;
  analysisDepth?: number;
  /** Full game analysis data (from analyzeGame) */
  analysis?: GameAnalysis;
  /** External control of move index - navigate to this move when set */
  externalMoveIndex?: number;
}

const DEFAULT_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

export default function AnalyzableChessBoard({
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
  gameResult,
  showAnalysis = false,
  multiPv = 3,
  analysisDepth = 20,
  analysis,
  externalMoveIndex,
}: AnalyzableChessBoardProps) {
  const [currentFen, setCurrentFen] = useState<string>(fen || DEFAULT_FEN);
  const [currentMoveIndex, setCurrentMoveIndex] = useState(startIndex);

  const stockfish = useStockfish(
    showAnalysis ? currentFen : null,
    { multiPv, depth: analysisDepth }
  );

  const handlePositionChange = useCallback((newFen: string, moveIndex: number) => {
    setCurrentFen(newFen);
    setCurrentMoveIndex(moveIndex);
    onPositionChange?.(newFen, moveIndex);
  }, [onPositionChange]);

  // Clean moves to get SAN notation
  const cleanedMoves = useMemo(() => {
    return moves.map(cleanMove).filter((m): m is string => m !== null);
  }, [moves]);

  // Get current move's analysis if available
  const currentMoveAnalysis = analysis && currentMoveIndex > 0 && currentMoveIndex <= analysis.moves.length
    ? analysis.moves[currentMoveIndex - 1]
    : null;

  // Get the SAN move played and convert best move to SAN
  const currentMoveSan = currentMoveIndex > 0 ? cleanedMoves[currentMoveIndex - 1] : null;

  // Calculate the FEN before the current move to convert best_move UCI to SAN
  const bestMoveSan = useMemo(() => {
    if (!currentMoveAnalysis?.best_move || currentMoveIndex === 0) return null;

    // Replay game to position before current move
    const chess = new Chess();
    for (let i = 0; i < currentMoveIndex - 1; i++) {
      try {
        chess.move(cleanedMoves[i]);
      } catch {
        break;
      }
    }

    return uciToSan(chess.fen(), currentMoveAnalysis.best_move);
  }, [currentMoveAnalysis?.best_move, currentMoveIndex, cleanedMoves]);

  return (
    <div className={`flex flex-col gap-3 ${className}`}>
      {/* Engine Lines - Above the board */}
      {showAnalysis && (
        <EngineLines
          lines={stockfish.lines}
          currentFen={currentFen}
          isAnalyzing={stockfish.isAnalyzing}
          depth={stockfish.depth}
          targetDepth={stockfish.targetDepth}
        />
      )}

      {showAnalysis && stockfish.error && (
        <div className="bg-red-900/20 border border-red-800 text-red-400 text-sm px-3 py-2 rounded">
          {stockfish.error}
        </div>
      )}

      {/* Board with Eval Bar */}
      <div className="flex gap-2">
        {showAnalysis && (
          <div className="flex-shrink-0" style={{ height: 'auto' }}>
            <div className="h-full">
              <EvalBar
                evaluation={stockfish.evaluation}
                isMate={stockfish.isMate}
                mateIn={stockfish.mateIn}
                orientation={orientation}
              />
            </div>
          </div>
        )}
        <div className="flex-1">
          <ChessBoard
            fen={fen}
            moves={moves}
            startIndex={startIndex}
            orientation={orientation}
            whitePlayer={whitePlayer}
            blackPlayer={blackPlayer}
            gameUrl={gameUrl}
            showControls={showControls}
            onPositionChange={handlePositionChange}
            gameResult={gameResult}
            externalMoveIndex={externalMoveIndex}
          />
        </div>
      </div>

      {/* Move classification indicator when full analysis is available */}
      {analysis && currentMoveAnalysis && currentMoveSan && (
        <div className="bg-slate-800/50 rounded-lg px-3 py-2 text-sm">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className={`font-medium capitalize ${CLASSIFICATION_COLORS[currentMoveAnalysis.classification]}`}>
                {currentMoveAnalysis.classification}
              </span>
              <span className="text-slate-500">
                Move {currentMoveIndex}: <span className="text-slate-300">{currentMoveSan}</span>
              </span>
            </div>
            {currentMoveAnalysis.cp_loss > 0 && (
              <span className="text-slate-400">
                -{(currentMoveAnalysis.cp_loss / 100).toFixed(2)} pawns
              </span>
            )}
          </div>
          {bestMoveSan && bestMoveSan !== currentMoveSan && (
            <div className="flex items-center gap-2 mt-1 text-slate-400">
              <span className="text-emerald-400">Best:</span>
              <span className="text-white">{bestMoveSan}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
