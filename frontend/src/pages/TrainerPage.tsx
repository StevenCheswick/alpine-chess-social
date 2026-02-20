import { useState, useEffect, useCallback, useRef } from 'react';
import { Chessboard } from 'react-chessboard';
import { EvalBar } from '../components/chess';
import { TrainerBoard } from '../components/chess/TrainerBoard';
import { trainerService, type TrainerOpening, type TrainerPuzzle } from '../services/trainerService';

export default function TrainerPage() {
  // Phase 1: Opening selection
  const [openings, setOpenings] = useState<TrainerOpening[]>([]);
  const [loadingOpenings, setLoadingOpenings] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Phase 2: Puzzle drilling
  const [selectedOpening, setSelectedOpening] = useState<string | null>(null);
  const [puzzles, setPuzzles] = useState<TrainerPuzzle[]>([]);
  const [puzzleIndex, setPuzzleIndex] = useState(0);
  const [loadingPuzzles, setLoadingPuzzles] = useState(false);
  const [retryKey, setRetryKey] = useState(0);
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());

  // Trainer board state (lifted from hook)
  const [evalCp, setEvalCp] = useState(0);

  const fetchOpenings = useCallback(async () => {
    setLoadingOpenings(true);
    try {
      const data = await trainerService.listOpenings();
      setOpenings(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load openings');
    } finally {
      setLoadingOpenings(false);
    }
  }, []);

  // Load openings on mount
  useEffect(() => {
    fetchOpenings();
  }, [fetchOpenings]);

  const selectOpening = useCallback(async (name: string) => {
    setLoadingPuzzles(true);
    setError(null);
    try {
      const { puzzles: data, completed_ids } = await trainerService.getPuzzles(name);
      setPuzzles(data);
      const completedSet = new Set(completed_ids);
      setCompletedIds(completedSet);
      // Find first unsolved puzzle
      const firstUnsolved = data.findIndex(p => !completedSet.has(p.id));
      setPuzzleIndex(firstUnsolved === -1 ? 0 : firstUnsolved);
      setRetryKey(0);
      setSelectedOpening(name);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load puzzles');
    } finally {
      setLoadingPuzzles(false);
    }
  }, []);

  const goBack = useCallback(() => {
    setSelectedOpening(null);
    setPuzzles([]);
    setPuzzleIndex(0);
    setCompletedIds(new Set());
    // Re-fetch openings to refresh progress counts
    fetchOpenings();
  }, [fetchOpenings]);

  const handleComplete = useCallback((puzzleId: string) => {
    setCompletedIds(prev => {
      if (prev.has(puzzleId)) return prev;
      const next = new Set(prev);
      next.add(puzzleId);
      return next;
    });
    // Fire-and-forget POST to backend
    trainerService.markComplete(puzzleId).catch(() => {});
  }, []);

  const handleNext = useCallback(() => {
    // Find next unsolved puzzle after current index
    const nextUnsolved = puzzles.findIndex((p, i) => i > puzzleIndex && !completedIds.has(p.id));
    if (nextUnsolved !== -1) {
      setPuzzleIndex(nextUnsolved);
    } else {
      // All remaining are completed, just go to next sequentially
      setPuzzleIndex(i => i + 1);
    }
    setRetryKey(0);
  }, [puzzles, puzzleIndex, completedIds]);

  // ---- Puzzle drilling mode ----
  if (selectedOpening && puzzles.length > 0) {
    const puzzle = puzzles[puzzleIndex];
    const hasNext = puzzleIndex < puzzles.length - 1;
    const completedCount = puzzles.filter(p => completedIds.has(p.id)).length;

    return (
      <div className="space-y-6">
        {/* Back button */}
        <button
          onClick={goBack}
          className="flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Openings
        </button>

        <TrainerDrill
          key={`${puzzle.id}-${retryKey}`}
          puzzle={puzzle}
          puzzleIndex={puzzleIndex}
          totalPuzzles={puzzles.length}
          completedCount={completedCount}
          hasNext={hasNext}
          onNext={handleNext}
          onRetry={() => setRetryKey(k => k + 1)}
          onComplete={handleComplete}
          onEvalUpdate={setEvalCp}
          evalCp={evalCp}
        />
      </div>
    );
  }

  // ---- Opening selection mode ----
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Opening Trainer</h1>
        <p className="text-slate-400 text-sm mt-1">
          Practice punishing common opening mistakes
        </p>
      </div>

      {loadingOpenings && (
        <div className="flex items-center justify-center py-12">
          <span className="w-6 h-6 border-2 border-emerald-500/30 border-t-emerald-500 rounded-full animate-spin" />
        </div>
      )}

      {error && (
        <div className="card p-4 text-red-400 text-sm">{error}</div>
      )}

      {!loadingOpenings && !error && openings.length === 0 && (
        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg className="w-8 h-8 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="10" strokeWidth="1.5" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M12 8v4m0 4h.01" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">No openings available</h2>
          <p className="text-slate-400">
            Opening trainer puzzles haven't been uploaded yet.
          </p>
        </div>
      )}

      {loadingPuzzles && (
        <div className="flex items-center justify-center py-12">
          <span className="w-6 h-6 border-2 border-emerald-500/30 border-t-emerald-500 rounded-full animate-spin" />
        </div>
      )}

      {/* Opening cards */}
      {!loadingOpenings && openings.length > 0 && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {openings.map(opening => {
            const isComplete = opening.completed_count >= opening.puzzle_count && opening.puzzle_count > 0;
            const progressPct = opening.puzzle_count > 0
              ? Math.round((opening.completed_count / opening.puzzle_count) * 100)
              : 0;

            return (
              <button
                key={opening.opening_name}
                onClick={() => selectOpening(opening.opening_name)}
                disabled={loadingPuzzles}
                className={`card p-4 text-left hover:border-emerald-500/60 transition-all duration-200 group disabled:opacity-50 ${
                  isComplete ? 'border-emerald-500/40' : ''
                }`}
              >
                {/* Mini board */}
                <div className="mb-3 aspect-square w-full">
                  <Chessboard
                    options={{
                      position: opening.sample_fen,
                      boardOrientation: 'white',
                      allowDragging: false,
                      darkSquareStyle: { backgroundColor: '#779952' },
                      lightSquareStyle: { backgroundColor: '#edeed1' },
                    }}
                  />
                </div>

                <h3 className="text-white font-semibold group-hover:text-emerald-400 transition-colors flex items-center gap-2">
                  {opening.opening_name}
                  {isComplete && (
                    <svg className="w-4 h-4 text-emerald-400 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                    </svg>
                  )}
                </h3>
                <div className="flex items-center gap-2 mt-1">
                  <span className="text-slate-500 text-xs">
                    {opening.eco_codes.join(', ')}
                  </span>
                </div>

                {/* Progress bar + count */}
                <div className="mt-2">
                  <div className="flex items-center justify-between text-sm mb-1">
                    <span className={`text-xs ${isComplete ? 'text-emerald-400' : 'text-slate-400'}`}>
                      {opening.completed_count}/{opening.puzzle_count} completed
                    </span>
                    {progressPct > 0 && (
                      <span className="text-xs text-slate-500">{progressPct}%</span>
                    )}
                  </div>
                  <div className="w-full bg-slate-700/50 rounded-full h-1.5">
                    <div
                      className={`h-1.5 rounded-full transition-all duration-300 ${
                        isComplete ? 'bg-emerald-400' : 'bg-emerald-500/70'
                      }`}
                      style={{ width: `${progressPct}%` }}
                    />
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** Inner component for the active puzzle drill session */
function TrainerDrill({
  puzzle,
  puzzleIndex,
  totalPuzzles,
  completedCount,
  hasNext,
  onNext,
  onRetry,
  onComplete,
  onEvalUpdate,
  evalCp,
}: {
  puzzle: TrainerPuzzle;
  puzzleIndex: number;
  totalPuzzles: number;
  completedCount: number;
  hasNext: boolean;
  onNext: () => void;
  onRetry: () => void;
  onComplete: (puzzleId: string) => void;
  onEvalUpdate: (cp: number) => void;
  evalCp: number;
}) {
  const [moveHistory, setMoveHistory] = useState<{ san: string; type: 'mistake' | 'solver' | 'opponent' }[]>([]);
  const completedRef = useRef(false);

  const trainer = TrainerBoard({
    puzzle,
    onMoveHistory: setMoveHistory,
    onEvalUpdate,
  });

  const { board, phase, statusMessage, start, showHint, solverColor, fen, variationsCompleted, totalLeaves } = trainer;
  const allVariationsDone = variationsCompleted >= totalLeaves;

  // Auto-start on mount
  useEffect(() => {
    const timer = setTimeout(() => start(), 300);
    return () => clearTimeout(timer);
  }, [start]);

  // Fire completion callback when all variations done
  useEffect(() => {
    if (phase === 'done' && allVariationsDone && !completedRef.current) {
      completedRef.current = true;
      onComplete(puzzle.id);
    }
  }, [phase, allVariationsDone, onComplete, puzzle.id]);

  const rootEvalDisplay = puzzle.root_eval >= 10000
    ? 'Mate'
    : `+${(puzzle.root_eval / 100).toFixed(1)}`;

  // Eval bar values
  const isMate = Math.abs(evalCp) >= 10000;
  const mateIn = isMate ? (evalCp > 0 ? 1 : -1) : null;
  const evalForBar = isMate ? null : evalCp;

  return (
    <div className="flex flex-col lg:flex-row gap-6">
      {/* Left: EvalBar + Board */}
      <div className="flex gap-2 flex-1 max-w-[600px] items-start">
        <div className="h-[560px] hidden sm:block">
          <EvalBar
            evaluation={evalForBar}
            isMate={isMate}
            mateIn={mateIn}
            orientation={solverColor}
          />
        </div>
        {board}
      </div>

      {/* Right: Info panel */}
      <div className="lg:w-80 space-y-4">
        {/* Puzzle counter + variation progress */}
        <div className="flex items-center gap-4 text-sm text-slate-400">
          <span>Puzzle <span className="text-emerald-400 font-semibold">{puzzleIndex + 1}</span> / {totalPuzzles}</span>
          {totalLeaves > 1 && (
            <span>Variation <span className="text-emerald-400 font-semibold">{Math.min(variationsCompleted + (phase !== 'done' && phase !== 'idle' ? 1 : 0), totalLeaves)}</span> / {totalLeaves}</span>
          )}
        </div>

        {/* Progress summary */}
        <div className="text-xs text-slate-500">
          {completedCount}/{totalPuzzles} completed
        </div>

        {/* Mistake info */}
        <div className="card p-4">
          <h3 className="text-sm font-medium text-slate-400 mb-2">Mistake</h3>
          <div className="flex items-baseline gap-2">
            <span className="text-red-400 font-bold text-lg">{puzzle.mistake_san}</span>
            <span className="text-slate-500 text-xs">({puzzle.eco})</span>
          </div>
          <div className="flex gap-4 mt-1 text-xs text-slate-500">
            <span>{puzzle.games} games</span>
            <span>{rootEvalDisplay}</span>
            <span>-{(puzzle.cp_loss / 100).toFixed(1)} cp loss</span>
          </div>
          <div className="mt-1 text-xs text-slate-600 font-mono select-all cursor-text">{puzzle.id}</div>
        </div>

        {/* Status */}
        <div className="card p-4">
          <h3 className={`text-sm font-semibold mb-1 ${
            statusMessage.type === 'success' ? 'text-emerald-400' :
            statusMessage.type === 'error' ? 'text-red-400' :
            'text-slate-300'
          }`}>
            {statusMessage.title}
          </h3>
          <p className="text-slate-400 text-sm">{statusMessage.msg}</p>
        </div>

        {/* Move history */}
        <div className="card p-4 max-h-[200px] overflow-y-auto">
          <h3 className="text-sm font-medium text-slate-400 mb-2">Move History</h3>
          <div className="space-y-0.5">
            {moveHistory.length === 0 && (
              <p className="text-slate-600 text-xs">No moves yet</p>
            )}
            {Array.from({ length: Math.ceil(moveHistory.length / 2) }).map((_, i) => {
              const white = moveHistory[i * 2];
              const black = moveHistory[i * 2 + 1];
              return (
                <div key={i} className="flex gap-1 text-sm">
                  <span className="text-slate-600 w-7 text-right">{i + 1}.</span>
                  {white && (
                    <span className={
                      white.type === 'mistake' ? 'text-red-400 font-bold' :
                      white.type === 'solver' ? 'text-emerald-400 font-bold' :
                      'text-orange-400'
                    }>
                      {white.san}
                    </span>
                  )}
                  {black && (
                    <span className={
                      black.type === 'mistake' ? 'text-red-400 font-bold' :
                      black.type === 'solver' ? 'text-emerald-400 font-bold' :
                      'text-orange-400'
                    }>
                      {black.san}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* FEN */}
        <div className="card p-4">
          <h3 className="text-sm font-medium text-slate-400 mb-2">FEN</h3>
          <p className="text-xs text-slate-300 font-mono break-all select-all cursor-text">{fen}</p>
        </div>

        {/* Controls */}
        <div className="flex flex-col gap-2">
          {phase === 'solver_turn' && (
            <button
              onClick={showHint}
              className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors text-sm"
            >
              Hint
            </button>
          )}
          {phase === 'done' && allVariationsDone && (
            <button
              onClick={onRetry}
              className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 transition-colors text-sm"
            >
              Retry Puzzle
            </button>
          )}
          {phase === 'done' && allVariationsDone && hasNext && (
            <button
              onClick={onNext}
              className="px-4 py-2 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 text-sm shadow-[0_0_12px_rgba(16,185,129,0.3)]"
            >
              Next Puzzle
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
