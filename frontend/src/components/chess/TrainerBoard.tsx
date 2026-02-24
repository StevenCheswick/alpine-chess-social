import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Chessboard } from 'react-chessboard';
import { Chess, type Square } from 'chess.js';
import type { TrainerPuzzle, TrainerPuzzleTree } from '../../services/trainerService';

export type TrainerPhase = 'idle' | 'show_mistake' | 'solver_turn' | 'opponent_thinking' | 'showing_correction' | 'done';

/** Count variations (leaf paths) in the tree.
 *  Opponent nodes: sum all children (we guide through every branch).
 *  Solver nodes: max across accepted moves (user picks ONE, we follow).
 */
function countLeaves(node: TrainerPuzzleTree): number {
  if (node.type === 'cutoff' || node.type === 'terminal') return 1;
  if (!node.moves) return 1;
  const entries = Object.values(node.moves);
  if (entries.length === 0) return 1;
  if (node.type === 'opponent') {
    let total = 0;
    for (const m of entries) {
      if (m.result) total += countLeaves(m.result);
    }
    return total || 1;
  }
  // Solver node: user picks one accepted move, take the richest path
  let best = 0;
  for (const m of entries) {
    if (!m.accepted) continue;
    const n = m.result ? countLeaves(m.result) : 1;
    if (n > best) best = n;
  }
  return best || 1;
}

/** Count variations following only the main line (best opponent move at each node) */
function countMainLineLeaves(node: TrainerPuzzleTree): number {
  if (node.type === 'cutoff' || node.type === 'terminal') return 1;
  if (!node.moves) return 1;
  const entries = Object.values(node.moves);
  if (entries.length === 0) return 1;
  if (node.type === 'opponent') {
    // Only follow the best move (highest games count)
    const computed = entries.filter(m => m.result);
    if (computed.length === 0) return 1;
    const best = computed.reduce((a, b) => ((a.games ?? 0) >= (b.games ?? 0) ? a : b));
    return best.result ? countMainLineLeaves(best.result) : 1;
  }
  // Solver node: same as full count — user picks one, take the richest
  let best = 0;
  for (const m of entries) {
    if (!m.accepted) continue;
    const n = m.result ? countMainLineLeaves(m.result) : 1;
    if (n > best) best = n;
  }
  return best || 1;
}

/** Check if tree has opponent nodes with more than one computed move (deep variations exist) */
function treeHasDeepVariations(node: TrainerPuzzleTree): boolean {
  if (node.type === 'cutoff' || node.type === 'terminal' || !node.moves) return false;
  const entries = Object.entries(node.moves);
  if (node.type === 'opponent') {
    const computed = entries.filter(([, m]) => m.result);
    if (computed.length > 1) return true;
    for (const [, m] of computed) {
      if (m.result && treeHasDeepVariations(m.result)) return true;
    }
    return false;
  }
  // Solver node: check all accepted children
  for (const [, m] of entries) {
    if (!m.accepted) continue;
    if (m.result && treeHasDeepVariations(m.result)) return true;
  }
  return false;
}

/** Check if a subtree has any unvisited leaf nodes */
function hasUnvisitedLeaves(node: TrainerPuzzleTree, visited: Set<TrainerPuzzleTree>): boolean {
  if (node.type === 'cutoff' || node.type === 'terminal' || !node.moves) {
    return !visited.has(node);
  }
  const entries = Object.values(node.moves);
  if (entries.length === 0) return !visited.has(node);
  if (node.type === 'opponent') {
    for (const m of entries) {
      if (m.result && hasUnvisitedLeaves(m.result, visited)) return true;
    }
    return false;
  }
  // Solver node: true if any accepted move leads to unvisited territory
  for (const m of entries) {
    if (!m.accepted) continue;
    if (!m.result) { if (!visited.has(node)) return true; continue; }
    if (hasUnvisitedLeaves(m.result, visited)) return true;
  }
  return false;
}

/** Mark all direct cutoff/terminal results of accepted moves at a solver node as visited.
 *  Prevents getting stuck when user picks one of N accepted moves that all end immediately.
 */
function markSiblingLeaves(solverNode: TrainerPuzzleTree, visited: Set<TrainerPuzzleTree>) {
  if (!solverNode.moves) return;
  for (const m of Object.values(solverNode.moves)) {
    if (!m.accepted) continue;
    if (m.result && (m.result.type === 'cutoff' || m.result.type === 'terminal')) {
      visited.add(m.result);
    }
    if (!m.result) visited.add(solverNode);
  }
}

interface TrainerBoardProps {
  puzzle: TrainerPuzzle;
  onPhaseChange?: (phase: TrainerPhase) => void;
  onMoveHistory?: (moves: { san: string; type: 'mistake' | 'solver' | 'opponent' }[]) => void;
  onEvalUpdate?: (cp: number) => void;
  retryKey?: number;
}

function uciToMove(uci: string): { from: string; to: string; promotion?: string } {
  return {
    from: uci.slice(0, 2),
    to: uci.slice(2, 4),
    promotion: uci.length === 5 ? uci[4] : undefined,
  };
}

export function TrainerBoard({ puzzle, onPhaseChange, onMoveHistory, onEvalUpdate, retryKey }: TrainerBoardProps) {
  const [game, setGame] = useState(() => new Chess(puzzle.pre_mistake_fen));
  const [phase, setPhase] = useState<TrainerPhase>('idle');
  const [currentNode, setCurrentNode] = useState<TrainerPuzzleTree | null>(null);
  const currentNodeRef = useRef<TrainerPuzzleTree | null>(null);
  const phaseRef = useRef<TrainerPhase>('idle');
  const gameRef = useRef(game);
  const [lastMove, setLastMove] = useState<{ from: string; to: string } | null>(null);
  const [feedbackSquare, setFeedbackSquare] = useState<{ square: string; color: string } | null>(null);
  const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
  const [moveHistory, setMoveHistory] = useState<{ san: string; type: 'mistake' | 'solver' | 'opponent' }[]>([]);
  const [statusMessage, setStatusMessage] = useState<{ title: string; msg: string; type: 'info' | 'success' | 'error' }>({
    title: 'Ready', msg: '', type: 'info',
  });
  const feedbackTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Subvariation drilling state
  const [visitedLeaves] = useState(() => new Set<TrainerPuzzleTree>());
  const visitedLeavesRef = useRef(visitedLeaves);
  const [drillMode, setDrillMode] = useState<'main' | 'deep'>('main');
  const drillModeRef = useRef<'main' | 'deep'>('main');
  const [totalLeaves, setTotalLeaves] = useState(() => countMainLineLeaves(puzzle.tree));
  const totalLeavesRef = useRef(totalLeaves);
  const [variationsCompleted, setVariationsCompleted] = useState(0);
  const isFirstAttemptRef = useRef(true);
  const hadMistakeRef = useRef(false);
  const restartTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const movePendingRef = useRef(false);
  // Per-node shuffled move order so opponent never repeats the same branch
  const opponentOrderRef = useRef(new Map<TrainerPuzzleTree, string[]>());

  const solverColor = useMemo((): 'white' | 'black' => {
    return puzzle.solver_color === 'w' ? 'white' : 'black';
  }, [puzzle.solver_color]);

  // Keep refs in sync with state so callbacks always read latest values
  useEffect(() => { currentNodeRef.current = currentNode; }, [currentNode]);
  useEffect(() => { gameRef.current = game; }, [game]);
  useEffect(() => { drillModeRef.current = drillMode; }, [drillMode]);
  useEffect(() => { totalLeavesRef.current = totalLeaves; }, [totalLeaves]);

  const updatePhase = useCallback((p: TrainerPhase) => {
    const prev = phaseRef.current;
    console.log(`[Trainer] phase: ${prev} → ${p}`);
    phaseRef.current = p;
    setPhase(p);
    onPhaseChange?.(p);
  }, [onPhaseChange]);

  const addMove = useCallback((san: string, type: 'mistake' | 'solver' | 'opponent') => {
    setMoveHistory(prev => {
      const next = [...prev, { san, type }];
      onMoveHistory?.(next);
      return next;
    });
  }, [onMoveHistory]);

  // Reset when puzzle changes or retry
  useEffect(() => {
    console.log(`[Trainer] RESET — puzzle=${puzzle.id}, retryKey=${retryKey}`);
    const newGame = new Chess(puzzle.pre_mistake_fen);
    setGame(newGame);
    setCurrentNode(null);
    setLastMove(null);
    setFeedbackSquare(null);
    setSelectedSquare(null);
    setMoveHistory([]);
    onMoveHistory?.([]);
    setStatusMessage({ title: 'Ready', msg: `Opponent will play ${puzzle.mistake_san}`, type: 'info' });
    updatePhase('idle');
    onEvalUpdate?.(0);
    // Reset subvariation state
    clearTimeout(restartTimer.current);
    movePendingRef.current = false;
    hadMistakeRef.current = false;
    opponentOrderRef.current.clear();
    visitedLeaves.clear();
    setDrillMode('main');
    drillModeRef.current = 'main';
    setTotalLeaves(countMainLineLeaves(puzzle.tree));
    setVariationsCompleted(0);
    isFirstAttemptRef.current = true;
  }, [puzzle, retryKey]);

  // Start the puzzle: show mistake then enter solver_turn
  // fast=true skips the "Watch..." intro and uses shorter delays (for subsequent variations)
  const start = useCallback((fast?: boolean) => {
    const isFast = fast || !isFirstAttemptRef.current;
    console.log(`[Trainer] start(fast=${isFast}) called — puzzle=${puzzle.id}, mistake=${puzzle.mistake_san}`);
    const g = new Chess(puzzle.pre_mistake_fen);
    setGame(g);
    setLastMove(null);
    setFeedbackSquare(null);
    setSelectedSquare(null);
    setMoveHistory([]);
    onMoveHistory?.([]);
    onEvalUpdate?.(0);

    if (!isFast) {
      updatePhase('show_mistake');
      setStatusMessage({ title: 'Watch...', msg: 'Your opponent is about to blunder.', type: 'info' });
    } else {
      updatePhase('show_mistake');
      setStatusMessage({ title: 'Next variation', msg: `Opponent plays ${puzzle.mistake_san}...`, type: 'info' });
    }

    const delay = isFast ? 300 : 1000;
    setTimeout(() => {
      const move = uciToMove(puzzle.mistake_uci);
      try {
        g.move({ from: move.from as Square, to: move.to as Square, promotion: move.promotion });
      } catch (err) {
        console.error(`[Trainer] FAILED to apply mistake move:`, err, `fen=${g.fen()}`);
      }
      setGame(new Chess(g.fen()));
      setLastMove({ from: move.from, to: move.to });
      setCurrentNode(puzzle.tree);
      updatePhase('solver_turn');
      setStatusMessage({
        title: 'Punish the mistake!',
        msg: `They played ${puzzle.mistake_san}. Find the best response!`,
        type: 'info',
      });
      onEvalUpdate?.(puzzle.root_eval);
      addMove(puzzle.mistake_san, 'mistake');
      isFirstAttemptRef.current = false;
    }, delay);
  }, [puzzle, updatePhase, addMove, onEvalUpdate, onMoveHistory]);

  // Expose start method via ref-like pattern through phase
  // The parent calls start() indirectly; we track it in the phase
  useEffect(() => {
    // Auto-start on mount is NOT desired; parent controls via calling start
  }, []);

  const puzzleComplete = useCallback((leaf: TrainerPuzzleTree | null, message: string) => {
    // Mark this leaf as visited
    if (leaf) visitedLeavesRef.current.add(leaf);
    const completed = visitedLeavesRef.current.size;
    const total = totalLeavesRef.current;
    // Clamp in case sibling marking pushed visited count past max-based total
    const clamped = Math.min(completed, total);
    setVariationsCompleted(clamped);
    console.log(`[Trainer] puzzleComplete: ${message} — variation ${clamped}/${total} (mode=${drillModeRef.current}, raw visited=${completed})`);

    if (clamped >= total) {
      // All variations done — but if user made mistakes, replay the whole thing
      if (hadMistakeRef.current) {
        console.log(`[Trainer] puzzleComplete: had mistakes — restarting for clean run`);
        setStatusMessage({ title: 'Good, but not perfect', msg: 'You made mistakes along the way. Let\'s try it again from the top.', type: 'info' });
        hadMistakeRef.current = false;
        visitedLeavesRef.current.clear();
        opponentOrderRef.current.clear();
        setVariationsCompleted(0);
        clearTimeout(restartTimer.current);
        // Quick restart: skip the mistake animation, jump straight to solver's turn
        restartTimer.current = setTimeout(() => {
          const g = new Chess(puzzle.tree.fen);
          setGame(g);
          setLastMove(uciToMove(puzzle.mistake_uci));
          setMoveHistory([{ san: puzzle.mistake_san, type: 'mistake' }]);
          onMoveHistory?.([{ san: puzzle.mistake_san, type: 'mistake' }]);
          setCurrentNode(puzzle.tree);
          onEvalUpdate?.(puzzle.root_eval);
          updatePhase('solver_turn');
          setStatusMessage({
            title: 'Try again from the top',
            msg: `They played ${puzzle.mistake_san}. Find the best response!`,
            type: 'info',
          });
        }, 1000);
        return;
      }
      updatePhase('done');
      const doneMsg = drillModeRef.current === 'main' ? 'Main line complete!' : `Completed all ${total} variation${total !== 1 ? 's' : ''}.`;
      setStatusMessage({ title: doneMsg, msg: message, type: 'success' });
    } else {
      // More variations remain — show status, then auto-restart
      setStatusMessage({ title: `Variation ${clamped}/${total} complete!`, msg: message, type: 'success' });
      // Guard against React Strict Mode double-fire
      clearTimeout(restartTimer.current);
      restartTimer.current = setTimeout(() => start(true), 1500);
    }
  }, [updatePhase, start]);

  const playOpponentMove = useCallback((oppNode: TrainerPuzzleTree) => {
    console.log(`[Trainer] playOpponentMove() — node type=${oppNode.type}, fen=${oppNode.fen}, moves=${Object.keys(oppNode.moves || {}).length}`);

    if (!oppNode.moves) {
      console.log(`[Trainer] no opponent moves → puzzle complete`);
      puzzleComplete(oppNode, 'Position won!');
      return;
    }

    // Only pick from moves that have a result tree
    const computed = Object.entries(oppNode.moves).filter(([, m]) => m.result);
    console.log(`[Trainer] opponent moves with result: ${computed.length}/${Object.keys(oppNode.moves).length}`, computed.map(([u, m]) => `${u}(${m.san})`));
    if (computed.length === 0) {
      console.log(`[Trainer] no computed opponent moves → puzzle complete`);
      puzzleComplete(oppNode, 'Position won!');
      return;
    }

    let pick: [string, (typeof oppNode.moves)[string]];

    if (drillModeRef.current === 'main') {
      // Main line: always pick the most popular move (highest games count)
      pick = computed.reduce((a, b) => ((a[1].games ?? 0) >= (b[1].games ?? 0) ? a : b));
    } else {
      // Deep mode: shuffled order, never repeat
      let order = opponentOrderRef.current.get(oppNode);
      if (!order) {
        order = Object.keys(oppNode.moves).slice();
        for (let i = order.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [order[i], order[j]] = [order[j], order[i]];
        }
        opponentOrderRef.current.set(oppNode, order);
      }

      const visited = visitedLeavesRef.current;
      const computedSet = new Set(computed.map(([u]) => u));
      pick = order
        .filter(u => computedSet.has(u))
        .map(u => [u, oppNode.moves![u]] as [string, (typeof oppNode.moves)[string]])
        .find(([, m]) => hasUnvisitedLeaves(m.result!, visited))
        ?? computed[0];
    }

    const [uci, moveData] = pick;
    console.log(`[Trainer] opponent picks: ${uci} (${moveData.san}), mode=${drillModeRef.current}, computed=${computed.length}, result.type=${moveData.result!.type}`);

    setTimeout(() => {
      const curGame = gameRef.current;
      const copy = new Chess(curGame.fen());
      const move = uciToMove(uci);
      console.log(`[Trainer] applying opponent move: ${uci} on fen=${copy.fen()}`);
      try {
        const applied = copy.move({ from: move.from as Square, to: move.to as Square, promotion: move.promotion });
        console.log(`[Trainer] opponent move applied: ${applied.san}, new fen=${copy.fen()}`);
      } catch (err) {
        console.error(`[Trainer] FAILED to apply opponent move ${uci}:`, err, `fen=${copy.fen()}`);
      }
      setGame(copy);

      setLastMove(uciToMove(uci));
      addMove(moveData.san, 'opponent');
      onEvalUpdate?.(moveData.cp ?? 200);

      const result = moveData.result!;
      if (result.type === 'cutoff') {
        console.log(`[Trainer] opponent result=cutoff → puzzle complete`);
        puzzleComplete(result, `Opponent played ${moveData.san}. Advantage secured!`);
        return;
      }
      if (result.type === 'terminal') {
        console.log(`[Trainer] opponent result=terminal → puzzle complete`);
        puzzleComplete(result, `Opponent played ${moveData.san}. Game over!`);
        return;
      }

      // Solver's turn again
      console.log(`[Trainer] opponent result=solver node — solver moves: ${Object.keys(result.moves || {}).length}`);
      setCurrentNode(result);
      updatePhase('solver_turn');
      const gamesNote = moveData.games && moveData.games > 0 ? ` (${moveData.games} games)` : ' (engine)';
      setStatusMessage({
        title: 'Your turn',
        msg: `Opponent played ${moveData.san}${gamesNote}. Find the best response.`,
        type: 'info',
      });
    }, 700);
  }, [puzzleComplete, updatePhase, addMove, onEvalUpdate]);

  const tryMove = useCallback(
    (sourceSquare: string, targetSquare: string, pieceType: string): boolean => {
      // Read latest values from refs to avoid stale closures
      const curPhase = phaseRef.current;
      const curNode = currentNodeRef.current;
      const curGame = gameRef.current;

      console.log(`[Trainer] tryMove(${sourceSquare}→${targetSquare}, piece=${pieceType}) phase=${curPhase}, hasNode=${!!curNode}, hasMoves=${!!curNode?.moves}`);

      // Prevent double-fire from click+drop both triggering
      if (movePendingRef.current) {
        console.warn(`[Trainer] tryMove BLOCKED — move already pending`);
        return false;
      }

      if (curPhase !== 'solver_turn' || !curNode?.moves) {
        console.warn(`[Trainer] tryMove BLOCKED — phase=${curPhase}, currentNode=${!!curNode}, moves=${!!curNode?.moves}`);
        return false;
      }

      movePendingRef.current = true;

      // Build UCI string
      let uci = sourceSquare + targetSquare;
      if (
        pieceType.toLowerCase().includes('p') &&
        (targetSquare[1] === '8' || targetSquare[1] === '1')
      ) {
        // Check if any accepted move matches with a promotion
        const promoMatch = Object.keys(curNode.moves).find(
          k => k.startsWith(uci) && k.length === 5 && curNode.moves![k].accepted
        );
        uci = promoMatch || uci + 'q';
        console.log(`[Trainer] promotion → uci=${uci}`);
      }

      const moveData = curNode.moves[uci];
      console.log(`[Trainer] lookup uci=${uci} — found=${!!moveData}, accepted=${moveData?.accepted}, available=[${Object.keys(curNode.moves).join(', ')}]`);

      if (!moveData || !moveData.accepted) {
        // Wrong move — red flash, show correct move, then reset to same position
        console.log(`[Trainer] wrong move ${uci} — ${!moveData ? 'not in book' : 'not accepted'}`);
        hadMistakeRef.current = true;
        movePendingRef.current = false;
        setFeedbackSquare({ square: targetSquare, color: 'rgba(220, 38, 38, 0.5)' });
        setSelectedSquare(null);
        clearTimeout(feedbackTimeout.current);

        // After 800ms red flash, show the best accepted move on the board
        feedbackTimeout.current = setTimeout(() => {
          setFeedbackSquare(null);
          // Find best accepted move (highest cp)
          const accepted = Object.entries(curNode.moves!).filter(([, m]) => m.accepted);
          if (accepted.length === 0) return;
          const best = accepted.reduce((a, b) => ((a[1].cp ?? 0) >= (b[1].cp ?? 0) ? a : b));
          const [bestUci, bestData] = best;
          const bestMove = uciToMove(bestUci);

          // Apply the best move on the board with green highlight
          const correctionGame = new Chess(curGame.fen());
          try {
            correctionGame.move({ from: bestMove.from as Square, to: bestMove.to as Square, promotion: bestMove.promotion });
          } catch (err) {
            console.error(`[Trainer] FAILED to apply correction move:`, err);
          }
          setGame(correctionGame);
          setLastMove({ from: bestMove.from, to: bestMove.to });
          setFeedbackSquare({ square: bestMove.to, color: 'rgba(0, 200, 83, 0.5)' });
          updatePhase('showing_correction');
          setStatusMessage({
            title: 'Wrong move',
            msg: `The best move was ${bestData.san}. Now play it.`,
            type: 'error',
          });

          // After 1.5s, reset back to the same position so user can play the correct move
          setTimeout(() => {
            setFeedbackSquare(null);
            setGame(new Chess(curGame.fen()));
            setLastMove(null);
            updatePhase('solver_turn');
            setStatusMessage({
              title: 'Try again',
              msg: `Play ${bestData.san}.`,
              type: 'info',
            });
          }, 1500);
        }, 800);

        return false;
      }

      // Correct move!
      console.log(`[Trainer] correct move: ${uci} (${moveData.san}), result.type=${moveData.result?.type ?? 'none'}`);
      const copy = new Chess(curGame.fen());
      const move = uciToMove(uci);
      try {
        const applied = copy.move({
          from: move.from as Square,
          to: move.to as Square,
          promotion: move.promotion || (
            pieceType.toLowerCase().includes('p') && (targetSquare[1] === '8' || targetSquare[1] === '1')
              ? 'q' : undefined
          ),
        });
        console.log(`[Trainer] solver move applied: ${applied.san}, new fen=${copy.fen()}`);
      } catch (err) {
        console.error(`[Trainer] FAILED to apply solver move ${uci}:`, err, `fen=${curGame.fen()}`);
        movePendingRef.current = false;
        return false;
      }

      setGame(copy);
      setLastMove({ from: sourceSquare, to: targetSquare });
      setFeedbackSquare({ square: targetSquare, color: 'rgba(0, 200, 83, 0.5)' });
      setSelectedSquare(null);

      clearTimeout(feedbackTimeout.current);
      feedbackTimeout.current = setTimeout(() => setFeedbackSquare(null), 600);

      // Status message for correct
      const accepted = Object.values(curNode.moves).filter(m => m.accepted);
      if (accepted.length === 1) {
        setStatusMessage({ title: 'Correct!', msg: `${moveData.san} — the only winning move.`, type: 'success' });
      } else {
        const others = accepted.filter(m => m.san !== moveData.san).map(m => m.san).join(', ');
        setStatusMessage({ title: 'Correct!', msg: `${moveData.san} — correct! Also good: ${others}`, type: 'success' });
      }

      addMove(moveData.san, 'solver');
      onEvalUpdate?.(moveData.cp ?? puzzle.root_eval);

      const result = moveData.result;
      if (!result || result.type === 'cutoff') {
        console.log(`[Trainer] solver result=${result?.type ?? 'none'} → puzzle complete`);
        movePendingRef.current = false;
        // Mark all sibling cutoff/terminal leaves so we don't revisit this solver node
        markSiblingLeaves(curNode, visitedLeavesRef.current);
        setTimeout(() => puzzleComplete(result ?? curNode, 'Position won! Advantage secured.'), 500);
        return true;
      }
      if (result.type === 'terminal') {
        console.log(`[Trainer] solver result=terminal → puzzle complete`);
        movePendingRef.current = false;
        markSiblingLeaves(curNode, visitedLeavesRef.current);
        setTimeout(() => puzzleComplete(result, 'Checkmate! Brilliant!'), 500);
        return true;
      }

      // Opponent's turn
      console.log(`[Trainer] → opponent_thinking`);
      movePendingRef.current = false;
      updatePhase('opponent_thinking');
      playOpponentMove(result);

      return true;
    },
    [puzzle, addMove, onEvalUpdate, puzzleComplete, updatePhase, playOpponentMove],
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
      const curPhase = phaseRef.current;
      if (curPhase !== 'solver_turn') return false;
      const isWhitePiece = piece.pieceType[0] === 'w';
      return solverColor === 'white' ? isWhitePiece : !isWhitePiece;
    },
    [solverColor],
  );

  const isSolverPiece = useCallback(
    (pieceType: string) => {
      const isWhitePiece = pieceType[0] === 'w';
      return solverColor === 'white' ? isWhitePiece : !isWhitePiece;
    },
    [solverColor],
  );

  const legalMoveSquares = useMemo(() => {
    if (!selectedSquare) return new Set<string>();
    const moves = game.moves({ square: selectedSquare as Square, verbose: true });
    return new Set(moves.map(m => m.to));
  }, [selectedSquare, game]);

  const handlePieceClick = useCallback(
    ({ piece, square }: { isSparePiece: boolean; piece: { pieceType: string }; square: string | null }) => {
      if (phaseRef.current !== 'solver_turn' || !square) return;

      if (isSolverPiece(piece.pieceType)) {
        setSelectedSquare(prev => prev === square ? null : square);
        return;
      }

      if (selectedSquare && selectedSquare !== square && legalMoveSquares.has(square)) {
        const selectedPiece = gameRef.current.get(selectedSquare as Square);
        if (selectedPiece) {
          tryMove(selectedSquare, square, selectedPiece.type);
        }
      } else {
        setSelectedSquare(null);
      }
    },
    [selectedSquare, isSolverPiece, tryMove, legalMoveSquares],
  );

  const handleSquareClick = useCallback(
    ({ square }: { piece: { pieceType: string } | null; square: string }) => {
      if (phaseRef.current !== 'solver_turn' || !selectedSquare) return;
      if (square === selectedSquare) {
        setSelectedSquare(null);
        return;
      }

      const curGame = gameRef.current;
      const pieceOnSquare = curGame.get(square as Square);
      if (pieceOnSquare) {
        const pt = (pieceOnSquare.color === 'w' ? 'w' : 'b') + pieceOnSquare.type.toUpperCase();
        if (isSolverPiece(pt)) {
          setSelectedSquare(square);
          return;
        }
      }

      if (!legalMoveSquares.has(square)) {
        setSelectedSquare(null);
        return;
      }

      const selectedPiece = curGame.get(selectedSquare as Square);
      if (selectedPiece) {
        tryMove(selectedSquare, square, selectedPiece.type);
      }
    },
    [selectedSquare, isSolverPiece, tryMove, legalMoveSquares],
  );

  // Hint: highlight from-square of first accepted move
  const showHint = useCallback(() => {
    if (!currentNode?.moves) return;
    const accepted = Object.entries(currentNode.moves).filter(([, m]) => m.accepted);
    if (accepted.length > 0) {
      const [uci] = accepted[0];
      const fromSq = uci.slice(0, 2);
      setSelectedSquare(fromSq);
      setStatusMessage({ title: 'Hint', msg: `Try moving the piece on ${fromSq}.`, type: 'info' });
    }
  }, [currentNode]);

  const hasDeepVariations = useMemo(() => treeHasDeepVariations(puzzle.tree), [puzzle]);
  const deepVariationCount = useMemo(() => countLeaves(puzzle.tree), [puzzle]);

  const startDeepDrill = useCallback(() => {
    visitedLeaves.clear();
    opponentOrderRef.current.clear();
    movePendingRef.current = false;
    setDrillMode('deep');
    drillModeRef.current = 'deep';
    setTotalLeaves(deepVariationCount);
    setVariationsCompleted(0);
    isFirstAttemptRef.current = true;
    // Will be started by the parent via start()
  }, [visitedLeaves, deepVariationCount]);

  // Build square styles
  const squareStyles: Record<string, React.CSSProperties> = {};

  if (lastMove) {
    squareStyles[lastMove.from] = { backgroundColor: 'rgba(255, 255, 0, 0.3)' };
    squareStyles[lastMove.to] = { backgroundColor: 'rgba(255, 255, 0, 0.3)' };
  }

  if (selectedSquare) {
    squareStyles[selectedSquare] = { backgroundColor: 'rgba(20, 85, 200, 0.5)' };
    for (const sq of legalMoveSquares) {
      const hasPiece = game.get(sq as Square);
      if (hasPiece) {
        squareStyles[sq] = { boxShadow: 'inset 0 0 0 4px rgba(20, 85, 200, 0.6)' };
      } else {
        squareStyles[sq] = { background: 'radial-gradient(circle, rgba(20, 85, 200, 0.4) 24%, transparent 24%)' };
      }
    }
  }

  if (feedbackSquare) {
    squareStyles[feedbackSquare.square] = { backgroundColor: feedbackSquare.color };
  }

  return {
    board: (
      <div className="w-full max-w-[560px] aspect-square overflow-hidden">
        <Chessboard
          options={{
            position: game.fen(),
            boardOrientation: solverColor,
            onPieceDrop: handlePieceDrop,
            onPieceClick: handlePieceClick,
            onSquareClick: handleSquareClick,
            canDragPiece,
            squareStyles,
            animationDurationInMs: 200,
          }}
        />
      </div>
    ),
    phase,
    statusMessage,
    moveHistory,
    start,
    showHint,
    solverColor,
    fen: game.fen(),
    variationsCompleted,
    totalLeaves,
    drillMode,
    hasDeepVariations,
    deepVariationCount,
    startDeepDrill,
  };
}
