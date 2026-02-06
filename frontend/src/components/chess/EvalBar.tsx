import { useRef, useMemo, memo } from 'react';

interface EvalBarProps {
  evaluation: number | null;
  isMate: boolean;
  mateIn: number | null;
  orientation?: 'white' | 'black';
}

// Threshold: don't visually update for changes smaller than this (in win%)
const MIN_VISUAL_CHANGE = 0.5;

export const EvalBar = memo(function EvalBar({ evaluation, isMate, mateIn, orientation = 'white' }: EvalBarProps) {
  const lastPercentRef = useRef<number>(50);

  const getWhitePercentage = (): number => {
    if (isMate && mateIn !== null) {
      return mateIn > 0 ? 98 : 2;
    }

    if (evaluation === null) return 50;

    // Lichess win probability formula
    const winPercent = 50 + 50 * (2 / (1 + Math.exp(-0.00368208 * evaluation)) - 1);
    return Math.max(2, Math.min(98, winPercent));
  };

  // Apply threshold to reduce visual flutter
  const whitePercent = useMemo(() => {
    const newPercent = getWhitePercentage();
    const diff = Math.abs(newPercent - lastPercentRef.current);

    // Only update if change is significant OR it's a mate situation
    if (diff >= MIN_VISUAL_CHANGE || isMate) {
      lastPercentRef.current = newPercent;
      return newPercent;
    }

    return lastPercentRef.current;
  }, [evaluation, isMate, mateIn]);

  const formatScore = (): string => {
    if (isMate && mateIn !== null) {
      const prefix = mateIn > 0 ? '+' : '-';
      return `${prefix}M${Math.abs(mateIn)}`;
    }

    if (evaluation === null) return '0.0';

    const pawns = Math.abs(evaluation) / 100;
    const prefix = evaluation > 0 ? '+' : evaluation < 0 ? '-' : '';
    return `${prefix}${pawns.toFixed(1)}`;
  };

  const displayScore = formatScore();
  const isWhiteAdvantage = (evaluation !== null && evaluation > 0) || (isMate && mateIn !== null && mateIn > 0);

  // If board is flipped, flip the bar too
  const topPercent = orientation === 'white' ? (100 - whitePercent) : whitePercent;

  return (
    <div className="w-6 h-full bg-slate-800 rounded overflow-hidden flex flex-col relative border border-slate-700">
      {/* Dark side (top when white orientation) */}
      <div
        className="bg-slate-600 transition-all duration-300 ease-out"
        style={{ height: `${topPercent}%` }}
      />
      {/* Light side (bottom when white orientation) */}
      <div
        className="bg-slate-200 transition-all duration-300 ease-out"
        style={{ height: `${100 - topPercent}%` }}
      />
      {/* Score label */}
      <div
        className={`absolute left-1/2 -translate-x-1/2 text-[10px] font-bold font-mono px-0.5 rounded whitespace-nowrap ${
          isWhiteAdvantage
            ? 'bottom-1 text-slate-800 bg-slate-200/90'
            : 'top-1 text-white bg-slate-800/80'
        }`}
      >
        {displayScore}
      </div>
    </div>
  );
});
