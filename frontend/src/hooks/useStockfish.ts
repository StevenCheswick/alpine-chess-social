import { useState, useEffect, useRef, useCallback } from 'react';
import { Chess } from 'chess.js';

// Simple LRU-ish cache for evaluations
const evalCache = new Map<string, { evaluation: number | null; isMate: boolean; mateIn: number | null }>();
const MAX_CACHE_SIZE = 500;

function getCachedEval(fen: string) {
  // Normalize FEN (ignore move counters for caching)
  const key = fen.split(' ').slice(0, 4).join(' ');
  return evalCache.get(key);
}

function setCachedEval(fen: string, eval_: { evaluation: number | null; isMate: boolean; mateIn: number | null }) {
  const key = fen.split(' ').slice(0, 4).join(' ');
  if (evalCache.size >= MAX_CACHE_SIZE) {
    // Delete oldest entry
    const firstKey = evalCache.keys().next().value;
    if (firstKey) evalCache.delete(firstKey);
  }
  evalCache.set(key, eval_);
}

export interface EngineLine {
  depth: number;
  multipv: number;
  score: { type: 'cp' | 'mate'; value: number };
  pv: string[];
}

export interface StockfishState {
  isReady: boolean;
  isAnalyzing: boolean;
  evaluation: number | null;
  isMate: boolean;
  mateIn: number | null;
  depth: number;
  targetDepth: number;
  lines: EngineLine[];
  error: string | null;
}

export interface StockfishOptions {
  multiPv?: number;
  depth?: number;
  debounceMs?: number;
}

const STOCKFISH_PATH = '/stockfish/stockfish.js';

const DEFAULT_OPTIONS: StockfishOptions = {
  multiPv: 3,
  depth: 18,
  debounceMs: 300,
};

// Check if position is valid and has legal moves
function isAnalyzablePosition(fen: string): { valid: boolean; gameOver: boolean; result?: string } {
  try {
    const chess = new Chess(fen);
    if (chess.isGameOver()) {
      let result = 'Game over';
      if (chess.isCheckmate()) {
        result = chess.turn() === 'w' ? 'Black wins by checkmate' : 'White wins by checkmate';
      } else if (chess.isStalemate()) {
        result = 'Draw by stalemate';
      } else if (chess.isDraw()) {
        result = 'Draw';
      }
      return { valid: true, gameOver: true, result };
    }
    return { valid: true, gameOver: false };
  } catch {
    return { valid: false, gameOver: false };
  }
}

// Get whose turn it is from FEN
function getTurnFromFen(fen: string): 'w' | 'b' {
  const parts = fen.split(' ');
  return (parts[1] === 'b' ? 'b' : 'w');
}

export function useStockfish(
  fen: string | null,
  options: StockfishOptions = {}
): StockfishState {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  const [state, setState] = useState<StockfishState>({
    isReady: false,
    isAnalyzing: false,
    evaluation: null,
    isMate: false,
    mateIn: null,
    depth: 0,
    targetDepth: opts.depth!,
    lines: [],
    error: null,
  });

  const workerRef = useRef<Worker | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingFenRef = useRef<string | null>(null);
  const currentFenRef = useRef<string | null>(null);
  const isEngineReadyRef = useRef(false);
  const multiPvRef = useRef(opts.multiPv);
  const depthRef = useRef(opts.depth);

  // Throttling: accumulate updates and flush periodically
  const pendingStateRef = useRef<Partial<StockfishState>>({});
  const throttleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastUpdateRef = useRef<number>(0);
  const THROTTLE_MS = 150; // Only update UI every 150ms

  // Throttled state update - batches rapid updates
  const flushState = useCallback(() => {
    const pending = pendingStateRef.current;
    if (Object.keys(pending).length > 0) {
      setState(s => ({ ...s, ...pending }));
      pendingStateRef.current = {};
    }
    throttleTimerRef.current = null;
  }, []);

  const throttledSetState = useCallback((updates: Partial<StockfishState>) => {
    pendingStateRef.current = { ...pendingStateRef.current, ...updates };

    const now = Date.now();
    const timeSinceLastUpdate = now - lastUpdateRef.current;

    // If enough time has passed, update immediately
    if (timeSinceLastUpdate >= THROTTLE_MS) {
      lastUpdateRef.current = now;
      flushState();
    } else if (!throttleTimerRef.current) {
      // Schedule an update
      throttleTimerRef.current = setTimeout(() => {
        lastUpdateRef.current = Date.now();
        flushState();
      }, THROTTLE_MS - timeSinceLastUpdate);
    }
  }, [flushState]);

  // Initialize worker
  useEffect(() => {
    if (typeof window === 'undefined') return;

    try {
      const worker = new Worker(STOCKFISH_PATH);
      workerRef.current = worker;

      worker.onmessage = (event: MessageEvent<string>) => {
        const line = event.data;

        if (line === 'uciok') {
          worker.postMessage(`setoption name MultiPV value ${multiPvRef.current}`);
          worker.postMessage('isready');
        }

        if (line === 'readyok') {
          isEngineReadyRef.current = true;
          setState(s => ({ ...s, isReady: true, error: null }));

          // If there's a pending analysis, start it now
          if (pendingFenRef.current) {
            const fenToAnalyze = pendingFenRef.current;
            pendingFenRef.current = null;
            startAnalysis(fenToAnalyze);
          }
        }

        if (line.startsWith('info depth') && line.includes(' pv ')) {
          const parsed = parseInfoLine(line);
          if (parsed && currentFenRef.current) {
            // UCI scores are from side-to-move perspective
            // Convert to white's perspective for display
            const turn = getTurnFromFen(currentFenRef.current);
            const multiplier = turn === 'b' ? -1 : 1;

            // Adjust the score
            const adjustedScore = {
              type: parsed.score.type,
              value: parsed.score.value * multiplier,
            };

            const adjustedParsed = { ...parsed, score: adjustedScore };

            // Update lines in pending state (will be batched)
            setState(s => {
              const newLines = [...s.lines];
              const idx = adjustedParsed.multipv - 1;
              newLines[idx] = adjustedParsed;

              const primaryLine = newLines[0];
              const newEval = primaryLine?.score.type === 'cp' ? primaryLine.score.value : null;
              const newIsMate = primaryLine?.score.type === 'mate';
              const newMateIn = primaryLine?.score.type === 'mate' ? primaryLine.score.value : null;

              // Cache the evaluation at higher depths
              if (adjustedParsed.depth >= 12 && currentFenRef.current) {
                setCachedEval(currentFenRef.current, {
                  evaluation: newEval,
                  isMate: newIsMate,
                  mateIn: newMateIn,
                });
              }

              // Minimum depth before we trust the eval for display
              const MIN_DISPLAY_DEPTH = 8;

              // Don't update eval display until we have reliable depth
              // This prevents the "sharp drop then recovery" on position change
              if (adjustedParsed.depth < MIN_DISPLAY_DEPTH) {
                // Update lines and depth indicator, but keep previous eval
                return {
                  ...s,
                  depth: Math.max(s.depth, adjustedParsed.depth),
                  lines: newLines.slice(0, multiPvRef.current),
                  // Keep existing eval until we have reliable data
                };
              }

              // Throttle UI updates at higher depths
              const significantUpdate =
                adjustedParsed.depth === MIN_DISPLAY_DEPTH || // First reliable depth
                adjustedParsed.depth % 3 === 0 || // Update every 3 depths
                adjustedParsed.depth >= depthRef.current! - 1; // Always show near-final

              if (!significantUpdate && s.depth >= MIN_DISPLAY_DEPTH) {
                // Skip this update but keep lines current
                return { ...s, lines: newLines.slice(0, multiPvRef.current) };
              }

              return {
                ...s,
                depth: Math.max(s.depth, adjustedParsed.depth),
                lines: newLines.slice(0, multiPvRef.current),
                evaluation: newEval,
                isMate: newIsMate,
                mateIn: newMateIn,
              };
            });
          }
        }

        if (line.startsWith('bestmove')) {
          setState(s => ({ ...s, isAnalyzing: false }));
        }
      };

      worker.onerror = (e) => {
        console.error('Stockfish worker error:', e);
        setState(s => ({ ...s, error: `Stockfish error: ${e.message}`, isReady: false }));
      };

      worker.postMessage('uci');
    } catch (err) {
      console.error('Failed to create Stockfish worker:', err);
      setState(s => ({
        ...s,
        error: `Failed to load Stockfish: ${err instanceof Error ? err.message : 'Unknown error'}`
      }));
    }

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
      if (throttleTimerRef.current) {
        clearTimeout(throttleTimerRef.current);
      }
      if (workerRef.current) {
        workerRef.current.postMessage('quit');
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  const startAnalysis = (fenToAnalyze: string) => {
    // Check if position is valid and not game over
    const positionCheck = isAnalyzablePosition(fenToAnalyze);

    if (!positionCheck.valid) {
      setState(s => ({ ...s, isAnalyzing: false, error: 'Invalid position' }));
      return;
    }

    if (positionCheck.gameOver) {
      // For checkmate, show the final evaluation
      const chess = new Chess(fenToAnalyze);
      if (chess.isCheckmate()) {
        // The side to move is checkmated, so the other side won
        // Positive mateIn = white delivered mate, negative = black delivered mate
        setState(s => ({
          ...s,
          isAnalyzing: false,
          evaluation: null,
          isMate: true,
          mateIn: chess.turn() === 'w' ? -1 : 1, // -1 = black won, 1 = white won (for display)
          depth: 0,
          lines: [],
          error: null,
        }));
      } else {
        // Stalemate or draw
        setState(s => ({
          ...s,
          isAnalyzing: false,
          evaluation: 0,
          isMate: false,
          mateIn: null,
          depth: 0,
          lines: [],
          error: null,
        }));
      }
      return;
    }

    // Check cache first - show cached result instantly while re-analyzing
    const cached = getCachedEval(fenToAnalyze);
    if (cached) {
      setState(s => ({
        ...s,
        evaluation: cached.evaluation,
        isMate: cached.isMate,
        mateIn: cached.mateIn,
        depth: 12, // Indicate this is from cache
        error: null,
      }));
    }

    if (!workerRef.current || !isEngineReadyRef.current) {
      pendingFenRef.current = fenToAnalyze;
      return;
    }

    currentFenRef.current = fenToAnalyze;
    // Only clear lines if no cache, keeps UI stable
    setState(s => ({
      ...s,
      isAnalyzing: true,
      lines: cached ? s.lines : [],
      depth: cached ? s.depth : 0,
      error: null,
    }));

    // Stop any running analysis and use isready for proper sync
    workerRef.current.postMessage('stop');
    workerRef.current.postMessage('isready');
    // The readyok handler will see pendingFenRef is null, so we start here after a micro-delay
    setTimeout(() => {
      if (workerRef.current && currentFenRef.current === fenToAnalyze) {
        workerRef.current.postMessage(`position fen ${fenToAnalyze}`);
        workerRef.current.postMessage(`go depth ${depthRef.current}`);
      }
    }, 10);
  };

  // Update refs when options change
  useEffect(() => {
    multiPvRef.current = opts.multiPv!;
    depthRef.current = opts.depth!;
    setState(s => ({ ...s, targetDepth: opts.depth! }));
    if (state.isReady && workerRef.current) {
      workerRef.current.postMessage('stop');
      workerRef.current.postMessage(`setoption name MultiPV value ${opts.multiPv}`);
    }
  }, [opts.multiPv, opts.depth, state.isReady]);

  // Analyze position (debounced) with immediate stop on change
  useEffect(() => {
    // Immediately stop any running analysis when position changes
    if (workerRef.current && isEngineReadyRef.current) {
      workerRef.current.postMessage('stop');
    }

    if (!fen) {
      return;
    }

    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    debounceRef.current = setTimeout(() => {
      startAnalysis(fen);
    }, opts.debounceMs);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [fen, opts.debounceMs]);

  return state;
}

function parseInfoLine(line: string): EngineLine | null {
  const parts = line.split(' ');

  const getValue = (key: string): string | undefined => {
    const idx = parts.indexOf(key);
    return idx !== -1 && idx + 1 < parts.length ? parts[idx + 1] : undefined;
  };

  const depthStr = getValue('depth');
  if (!depthStr) return null;

  const depth = parseInt(depthStr, 10);
  if (depth === 0 || isNaN(depth)) return null;

  const multipv = parseInt(getValue('multipv') || '1', 10);

  let score: EngineLine['score'] = { type: 'cp', value: 0 };
  const scoreIdx = parts.indexOf('score');
  if (scoreIdx !== -1 && scoreIdx + 2 < parts.length) {
    if (parts[scoreIdx + 1] === 'cp') {
      score = { type: 'cp', value: parseInt(parts[scoreIdx + 2], 10) };
    } else if (parts[scoreIdx + 1] === 'mate') {
      score = { type: 'mate', value: parseInt(parts[scoreIdx + 2], 10) };
    }
  }

  const pvIdx = parts.indexOf('pv');
  const pv = pvIdx !== -1 ? parts.slice(pvIdx + 1) : [];

  return { depth, multipv, score, pv };
}
