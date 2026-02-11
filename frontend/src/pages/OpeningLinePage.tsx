import { useState, useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { AnalyzableChessBoard } from '../components/chess';
import GameAnalysisPanel from '../components/chess/GameAnalysisPanel';
import type { GameAnalysis, MoveAnalysis, MoveClassification, MoveClassifications } from '../types/analysis';

interface BlunderState {
  type: 'blunder';
  moves: string[];
  color: string;
  line: string;
  ply: number;
  avgCpLoss: number;
  bestMove?: string;
  mistakeCount: number;
}

interface CleanState {
  type: 'clean';
  moves: string[];
  color: string;
  line: string;
  avgCpLoss: number;
  cleanDepth: number;
  gameCount: number;
}

type LineState = BlunderState | CleanState;

function classifyByCpLoss(cpLoss: number): MoveClassification {
  if (cpLoss >= 200) return 'blunder';
  if (cpLoss >= 100) return 'mistake';
  if (cpLoss >= 50) return 'inaccuracy';
  return 'good';
}

function emptyClassifications(): MoveClassifications {
  return { best: 0, excellent: 0, good: 0, inaccuracy: 0, mistake: 0, blunder: 0, book: 0, forced: 0 };
}

function buildSyntheticAnalysis(state: LineState): GameAnalysis {
  const moves: MoveAnalysis[] = [];
  const userIsWhite = state.color === 'white';

  for (let i = 0; i < state.moves.length; i++) {
    const isUserMove = userIsWhite ? (i % 2 === 0) : (i % 2 === 1);

    if (state.type === 'blunder') {
      const isBlunderPly = i === state.ply;
      let classification: MoveClassification;
      let cpLoss = 0;
      let bestMove = '';

      if (isBlunderPly && isUserMove) {
        cpLoss = state.avgCpLoss;
        classification = classifyByCpLoss(cpLoss);
        bestMove = state.bestMove || '';
      } else if (isUserMove) {
        classification = 'book';
      } else {
        classification = 'book';
      }

      moves.push({
        move: state.moves[i],
        move_eval: 0,
        best_move: bestMove,
        best_eval: 0,
        cp_loss: cpLoss,
        classification,
      });
    } else {
      // Clean line: all moves are book (opening prep)
      moves.push({
        move: state.moves[i],
        move_eval: 0,
        best_move: '',
        best_eval: 0,
        cp_loss: 0,
        classification: 'book',
      });
    }
  }

  // Build classification counts
  const whiteCls = emptyClassifications();
  const blackCls = emptyClassifications();
  for (let i = 0; i < moves.length; i++) {
    const cls = i % 2 === 0 ? whiteCls : blackCls;
    cls[moves[i].classification]++;
  }

  // Compute simple accuracy (100 - avg cp loss scaled)
  const userMoves = moves.filter((_, i) => userIsWhite ? i % 2 === 0 : i % 2 === 1);
  const totalCpLoss = userMoves.reduce((sum, m) => sum + m.cp_loss, 0);
  const userAvgCpLoss = userMoves.length > 0 ? totalCpLoss / userMoves.length : 0;
  const userAccuracy = Math.max(0, Math.min(100, 100 - userAvgCpLoss / 3));

  return {
    white_accuracy: userIsWhite ? userAccuracy : 100,
    black_accuracy: userIsWhite ? 100 : userAccuracy,
    white_avg_cp_loss: userIsWhite ? userAvgCpLoss : 0,
    black_avg_cp_loss: userIsWhite ? 0 : userAvgCpLoss,
    white_classifications: whiteCls,
    black_classifications: blackCls,
    moves,
    isComplete: true,
  };
}

export default function OpeningLinePage() {
  const location = useLocation();
  const navigate = useNavigate();
  const state = location.state as LineState | null;

  const analysis = useMemo(() => state ? buildSyntheticAnalysis(state) : null, [state]);

  // Start at blunder ply for blunders, move 0 for clean lines
  const initialMoveIndex = state?.type === 'blunder' ? state.ply + 1 : 0;
  const [currentMoveIndex, setCurrentMoveIndex] = useState(initialMoveIndex);

  const handlePositionChange = (_fen: string, moveIndex: number) => {
    setCurrentMoveIndex(moveIndex);
  };

  const handleMoveClick = (moveIndex: number) => {
    setCurrentMoveIndex(moveIndex);
  };

  if (!state) {
    return (
      <div className="space-y-6">
        <button
          onClick={() => navigate(-1)}
          className="inline-flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Dashboard
        </button>
        <div className="card p-8 text-center">
          <h2 className="text-xl font-semibold text-white mb-2">No line data</h2>
          <p className="text-slate-400">Navigate here from your dashboard to review an opening line.</p>
        </div>
      </div>
    );
  }

  const color = state.color as 'white' | 'black';

  return (
    <div className="h-[calc(100vh-5.5rem)] flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-3 flex-shrink-0">
        <button
          onClick={() => navigate(-1)}
          className="inline-flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Dashboard
        </button>

        <div className="text-right">
          <h1 className="text-sm font-semibold text-white">{state.line}</h1>
          <p className="text-xs text-slate-400">
            {state.type === 'blunder'
              ? `Repeated ${state.mistakeCount} times as ${state.color} \u00b7 avg -${state.avgCpLoss} cp`
              : `${state.cleanDepth} moves deep as ${state.color} \u00b7 ${state.gameCount} ${state.gameCount === 1 ? 'game' : 'games'} \u00b7 ~${state.avgCpLoss} cp`
            }
          </p>
        </div>
      </div>

      {/* Main content: Board and Analysis side by side */}
      <div className="flex flex-col xl:flex-row gap-4 flex-1 min-h-0">
        {/* Chess Board */}
        <div className="xl:max-w-[min(520px,calc(100vh-12rem))] flex-shrink-0">
          <AnalyzableChessBoard
            moves={state.moves}
            orientation={color}
            startIndex={initialMoveIndex}
            showAnalysis={false}
            analysis={analysis || undefined}
            externalMoveIndex={currentMoveIndex}
            onPositionChange={handlePositionChange}
          />
        </div>

        {/* Analysis Panel */}
        {analysis && (
          <div className="flex-1 min-w-0">
            <GameAnalysisPanel
              analysis={analysis}
              userColor={color}
              moves={state.moves}
              onMoveClick={handleMoveClick}
              currentMoveIndex={currentMoveIndex}
            />
          </div>
        )}
      </div>
    </div>
  );
}
