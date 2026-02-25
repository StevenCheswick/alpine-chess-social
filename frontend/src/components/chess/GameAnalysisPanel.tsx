import type { GameAnalysis, MoveClassifications, MoveClassification } from '../../types/analysis';
import { CLASSIFICATION_COLORS } from '../../types/analysis';

interface GameAnalysisPanelProps {
  analysis: GameAnalysis;
  userColor: 'white' | 'black';
  moves?: string[];
  /** Callback when a move is clicked - passes the move index (1-indexed, after move played) */
  onMoveClick?: (moveIndex: number) => void;
  /** Current move index being viewed (1-indexed) */
  currentMoveIndex?: number;
}

function AccuracyCircle({ accuracy, label, isUser }: { accuracy: number; label: string; isUser: boolean }) {
  // Calculate circle progress
  const circumference = 2 * Math.PI * 40; // radius = 40
  const progress = (accuracy / 100) * circumference;
  const color = accuracy >= 90 ? 'text-emerald-400' :
                accuracy >= 80 ? 'text-green-400' :
                accuracy >= 70 ? 'text-yellow-400' :
                accuracy >= 60 ? 'text-amber-400' : 'text-red-400';

  return (
    <div className="flex flex-col items-center">
      <div className="relative w-24 h-24">
        <svg className="w-24 h-24 transform -rotate-90">
          {/* Background circle */}
          <circle
            cx="48"
            cy="48"
            r="40"
            stroke="currentColor"
            strokeWidth="8"
            fill="none"
            className="text-slate-700"
          />
          {/* Progress circle */}
          <circle
            cx="48"
            cy="48"
            r="40"
            stroke="currentColor"
            strokeWidth="8"
            fill="none"
            strokeLinecap="round"
            className={color}
            strokeDasharray={circumference}
            strokeDashoffset={circumference - progress}
            style={{ transition: 'stroke-dashoffset 0.5s ease-out' }}
          />
        </svg>
        <div className="absolute inset-0 flex items-center justify-center">
          <span className={`text-2xl font-bold ${color}`}>
            {accuracy.toFixed(1)}%
          </span>
        </div>
      </div>
      <span className={`mt-2 text-sm font-medium ${isUser ? 'text-white' : 'text-slate-400'}`}>
        {label}
      </span>
    </div>
  );
}

function ClassificationRow({
  label,
  count,
  colorClass,
  isNegative = false
}: {
  label: string;
  count: number;
  colorClass: string;
  isNegative?: boolean;
}) {
  return (
    <div className="flex items-center justify-between py-1">
      <div className="flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full ${isNegative ? 'bg-current opacity-50' : 'bg-current'} ${colorClass}`} />
        <span className="text-sm text-slate-300">{label}</span>
      </div>
      <span className={`text-sm font-medium ${colorClass}`}>{count}</span>
    </div>
  );
}

function ClassificationBreakdown({
  classifications,
  label,
  isUser
}: {
  classifications: MoveClassifications;
  label: string;
  isUser: boolean;
}) {
  return (
    <div className={`flex-1 ${isUser ? '' : 'opacity-75'}`}>
      <h4 className={`text-sm font-medium mb-2 ${isUser ? 'text-white' : 'text-slate-400'}`}>
        {label}
      </h4>
      <div className="space-y-0.5">
        {classifications.book > 0 && (
          <ClassificationRow label="Book" count={classifications.book} colorClass="text-yellow-800" />
        )}
        <ClassificationRow label="Best" count={classifications.best} colorClass="text-emerald-400" />
        <ClassificationRow label="Excellent" count={classifications.excellent} colorClass="text-green-400" />
        <ClassificationRow label="Good" count={classifications.good} colorClass="text-green-300" />
        <ClassificationRow label="Inaccuracy" count={classifications.inaccuracy} colorClass="text-yellow-400" isNegative />
        <ClassificationRow label="Mistake" count={classifications.mistake} colorClass="text-amber-400" isNegative />
        <ClassificationRow label="Blunder" count={classifications.blunder} colorClass="text-red-400" isNegative />
        {classifications.forced > 0 && (
          <ClassificationRow label="Forced" count={classifications.forced} colorClass="text-slate-400" />
        )}
      </div>
    </div>
  );
}

function cleanMove(move: string): string | null {
  const cleaned = move.replace(/^\d+\.+\s*/, '').replace(/[!?]+$/, '').trim();
  if (!cleaned || cleaned === '1-0' || cleaned === '0-1' || cleaned === '1/2-1/2') {
    return null;
  }
  return cleaned;
}

function ColorCodedPGN({
  moves,
  analysis,
  userColor,
  onMoveClick,
  currentMoveIndex,
}: {
  moves: string[];
  analysis: GameAnalysis;
  userColor: 'white' | 'black';
  onMoveClick?: (moveIndex: number) => void;
  currentMoveIndex?: number;
}) {
  const cleanedMoves = moves.map(cleanMove).filter((m): m is string => m !== null);

  // Group moves into pairs (white, black)
  const movePairs: Array<{
    number: number;
    white?: { move: string; classification?: MoveClassification };
    black?: { move: string; classification?: MoveClassification };
  }> = [];

  for (let i = 0; i < cleanedMoves.length; i += 2) {
    const moveNumber = Math.floor(i / 2) + 1;
    const whiteMove = cleanedMoves[i];
    const blackMove = cleanedMoves[i + 1];

    movePairs.push({
      number: moveNumber,
      white: whiteMove ? {
        move: whiteMove,
        classification: analysis.moves[i]?.classification,
      } : undefined,
      black: blackMove ? {
        move: blackMove,
        classification: analysis.moves[i + 1]?.classification,
      } : undefined,
    });
  }

  const getMoveClasses = (classification: MoveClassification | undefined, isUserMove: boolean) => {
    if (!isUserMove || !classification) {
      return 'text-slate-300';
    }
    return CLASSIFICATION_COLORS[classification];
  };

  return (
    <div className="max-h-48 overflow-y-auto scrollbar-hide" style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}>
      <div className="flex flex-wrap gap-x-3 gap-y-1 text-sm font-mono">
        {movePairs.map(({ number, white, black }) => (
          <span key={number} className="flex items-center gap-1">
            <span className="text-slate-500">{number}.</span>
            {white && (
              <span
                onClick={() => onMoveClick?.((number - 1) * 2 + 1)}
                className={`px-1 rounded cursor-pointer hover:ring-2 hover:ring-emerald-500/50 ${getMoveClasses(white.classification, userColor === 'white')} ${currentMoveIndex === (number - 1) * 2 + 1 ? 'ring-2 ring-emerald-500' : ''}`}
                title={userColor === 'white' ? white.classification : undefined}
              >
                {white.move}
              </span>
            )}
            {black && (
              <span
                onClick={() => onMoveClick?.((number - 1) * 2 + 2)}
                className={`px-1 rounded cursor-pointer hover:ring-2 hover:ring-emerald-500/50 ${getMoveClasses(black.classification, userColor === 'black')} ${currentMoveIndex === (number - 1) * 2 + 2 ? 'ring-2 ring-emerald-500' : ''}`}
                title={userColor === 'black' ? black.classification : undefined}
              >
                {black.move}
              </span>
            )}
          </span>
        ))}
      </div>
    </div>
  );
}

export default function GameAnalysisPanel({ analysis, userColor, moves, onMoveClick, currentMoveIndex }: GameAnalysisPanelProps) {
  const userAccuracy = userColor === 'white' ? analysis.white_accuracy : analysis.black_accuracy;
  const opponentAccuracy = userColor === 'white' ? analysis.black_accuracy : analysis.white_accuracy;
  const userClassifications = userColor === 'white' ? analysis.white_classifications : analysis.black_classifications;
  const opponentClassifications = userColor === 'white' ? analysis.black_classifications : analysis.white_classifications;

  return (
    <div className="card p-4 space-y-4 h-full overflow-y-auto">
      {/* Accuracy Section */}
      <div className="flex justify-around items-center py-2">
        <AccuracyCircle accuracy={userAccuracy} label="Your Accuracy" isUser={true} />
        <div className="w-px h-16 bg-slate-800" />
        <AccuracyCircle accuracy={opponentAccuracy} label="Opponent" isUser={false} />
      </div>

      {/* Divider */}
      <div className="border-t border-slate-800" />

      {/* Classification Breakdown */}
      <div className="flex gap-6">
        <ClassificationBreakdown
          classifications={userClassifications}
          label="Your Moves"
          isUser={true}
        />
        <div className="w-px bg-slate-800" />
        <ClassificationBreakdown
          classifications={opponentClassifications}
          label="Opponent Moves"
          isUser={false}
        />
      </div>

      {/* Color-coded PGN */}
      {moves && moves.length > 0 && (
        <>
          <div className="border-t border-slate-800" />
          <div>
            <h4 className="text-sm font-medium text-white mb-2">Moves</h4>
            <ColorCodedPGN moves={moves} analysis={analysis} userColor={userColor} onMoveClick={onMoveClick} currentMoveIndex={currentMoveIndex} />
          </div>
        </>
      )}
    </div>
  );
}
