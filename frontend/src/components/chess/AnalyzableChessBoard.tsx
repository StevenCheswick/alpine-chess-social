import { useState, useCallback } from 'react';
import ChessBoard from './ChessBoard';
import { EvalBar } from './EvalBar';
import { EngineLines } from './EngineLines';
import { useStockfish } from '../../hooks/useStockfish';

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
}: AnalyzableChessBoardProps) {
  const [currentFen, setCurrentFen] = useState<string>(fen || DEFAULT_FEN);

  const stockfish = useStockfish(
    showAnalysis ? currentFen : null,
    { multiPv, depth: analysisDepth }
  );

  const handlePositionChange = useCallback((newFen: string, moveIndex: number) => {
    setCurrentFen(newFen);
    onPositionChange?.(newFen, moveIndex);
  }, [onPositionChange]);

  return (
    <div className={`flex flex-col gap-3 ${className}`}>
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
          />
        </div>
      </div>

      {showAnalysis && (
        <EngineLines
          lines={stockfish.lines}
          currentFen={currentFen}
          isAnalyzing={stockfish.isAnalyzing}
          depth={stockfish.depth}
        />
      )}

      {showAnalysis && stockfish.error && (
        <div className="bg-red-900/20 border border-red-800 text-red-400 text-sm px-3 py-2 rounded">
          {stockfish.error}
        </div>
      )}
    </div>
  );
}
