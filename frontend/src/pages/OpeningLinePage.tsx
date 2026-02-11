import { useSearchParams, useNavigate } from 'react-router-dom';
import { ChessBoard } from '../components/chess';

export default function OpeningLinePage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();

  const movesParam = searchParams.get('moves') || '';
  const color = (searchParams.get('color') || 'white') as 'white' | 'black';
  const line = searchParams.get('line') || '';
  const count = searchParams.get('count') || '';
  const cpLoss = searchParams.get('cp') || '';
  const bestMove = searchParams.get('best') || '';

  const moves = movesParam ? movesParam.split(',') : [];
  const blunderMove = moves.length > 0 ? moves[moves.length - 1] : '';

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
      </div>

      {/* Main content: Board + Info side by side */}
      <div className="flex flex-col xl:flex-row gap-6 flex-1 min-h-0">
        {/* Board */}
        <div className="xl:max-w-[min(520px,calc(100vh-12rem))] flex-shrink-0">
          <ChessBoard
            moves={moves}
            orientation={color}
            startIndex={moves.length}
            showControls={true}
          />
        </div>

        {/* Info panel */}
        <div className="flex-1 min-w-0">
          <div className="bg-slate-800/50 border border-slate-700 rounded-xl p-5 space-y-4">
            <div>
              <h1 className="text-lg font-semibold text-white">{line}</h1>
              {count && cpLoss && (
                <p className="text-sm text-slate-400 mt-1">
                  Repeated {count} times as {color} &middot; avg -{cpLoss} cp
                </p>
              )}
            </div>

            <div className="border-t border-slate-700 pt-4 space-y-3">
              <div className="flex items-start gap-3">
                <span className="text-sm text-slate-500 w-20 shrink-0">You played</span>
                <span className="text-sm font-semibold text-orange-400">{blunderMove}</span>
              </div>
              {bestMove && (
                <div className="flex items-start gap-3">
                  <span className="text-sm text-slate-500 w-20 shrink-0">Best was</span>
                  <span className="text-sm font-semibold text-emerald-400">{bestMove}</span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
