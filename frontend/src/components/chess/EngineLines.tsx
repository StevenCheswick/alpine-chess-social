import { useMemo, memo } from 'react';
import { Chess } from 'chess.js';
import type { EngineLine } from '../../hooks/useStockfish';

interface EngineLinesProps {
  lines: EngineLine[];
  currentFen: string;
  isAnalyzing: boolean;
  depth: number;
  targetDepth: number;
}

export const EngineLines = memo(function EngineLines({ lines, currentFen, isAnalyzing, depth, targetDepth }: EngineLinesProps) {
  // Convert UCI moves to SAN notation
  const linesWithSan = useMemo(() => {
    return lines.map(line => ({
      ...line,
      pvSan: uciToSan(currentFen, line.pv),
    }));
  }, [lines, currentFen]);

  const formatScore = (line: EngineLine): string => {
    if (line.score.type === 'mate') {
      const prefix = line.score.value > 0 ? '+' : '';
      return `${prefix}M${line.score.value}`;
    }
    const pawns = line.score.value / 100;
    const prefix = pawns > 0 ? '+' : '';
    return `${prefix}${pawns.toFixed(2)}`;
  };

  const getScoreColor = (line: EngineLine): string => {
    if (line.score.type === 'mate') {
      return line.score.value > 0 ? 'bg-green-500' : 'bg-red-500';
    }
    const cp = line.score.value;
    if (cp > 200) return 'bg-green-500';
    if (cp > 50) return 'bg-green-400';
    if (cp < -200) return 'bg-red-500';
    if (cp < -50) return 'bg-red-400';
    return 'bg-slate-500';
  };

  if (lines.length === 0) {
    return (
      <div className="bg-slate-900 rounded-lg p-3 border border-slate-800">
        <div className="text-slate-500 text-sm text-center py-2">
          {isAnalyzing ? 'Analyzing...' : 'No analysis'}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-slate-900 rounded-lg border border-slate-800 overflow-hidden">
      {linesWithSan.map((line, idx) => (
        <div
          key={idx}
          className="flex items-center gap-2 px-3 py-2 border-b border-slate-800 last:border-b-0 font-mono text-sm"
        >
          <span className="text-slate-600 w-4 text-xs">{idx + 1}</span>
          <span
            className={`${getScoreColor(line)} text-white px-2 py-0.5 rounded text-xs font-bold min-w-[52px] text-center`}
          >
            {formatScore(line)}
          </span>
          <span className="text-slate-500 text-xs">d{line.depth}</span>
          <span className="text-slate-300 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
            {line.pvSan.slice(0, 8).join(' ')}
            {line.pvSan.length > 8 && '...'}
          </span>
        </div>
      ))}
      <div className="flex items-center justify-between px-3 py-1.5 bg-slate-800/50 text-xs">
        <span className="text-slate-500">
          Depth {depth}/{targetDepth}
        </span>
        {isAnalyzing && (
          <span className="text-blue-400 flex items-center gap-1">
            <span className="w-1.5 h-1.5 bg-blue-400 rounded-full animate-pulse" />
            Analyzing
          </span>
        )}
      </div>
    </div>
  );
});

function uciToSan(fen: string, uciMoves: string[]): string[] {
  try {
    const chess = new Chess(fen);
    const sanMoves: string[] = [];

    for (const uci of uciMoves) {
      if (uci.length < 4) break;

      const from = uci.slice(0, 2);
      const to = uci.slice(2, 4);
      const promotion = uci.length > 4 ? uci[4] : undefined;

      try {
        const move = chess.move({ from, to, promotion });
        if (move) {
          sanMoves.push(move.san);
        } else {
          break;
        }
      } catch {
        break;
      }
    }

    return sanMoves;
  } catch {
    return uciMoves;
  }
}
