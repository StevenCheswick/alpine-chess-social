/**
 * Analysis Proxy Service
 *
 * Connects to the Rust WebSocket analysis server. The client acts as a
 * Stockfish eval worker — the server sends positions to evaluate, the client
 * runs Stockfish WASM and returns results. The server handles move
 * classification, puzzle extraction, and cook() theme tagging.
 */

import { ANALYSIS_WS_URL } from '../config/api';
import type {
  FullAnalysis,
  BatchProgress,
  BatchGameResult,
  BatchGameInput,
} from '../types/analysis';

const STOCKFISH_PATH = '/stockfish/stockfish.js';

interface StockfishWorker {
  worker: Worker;
  analyze(fen: string, nodes: number): Promise<{ cp: number | null; mate: number | null; bestMove: string }>;
  analyzeMultiPv(fen: string, nodes: number, numPvs: number): Promise<{ lines: { pv: string[]; cp: number | null; mate: number | null }[] }>;
  destroy(): void;
}

function createStockfishWorker(): Promise<StockfishWorker> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(STOCKFISH_PATH);

    let resolveReady: (() => void) | null = null;
    let resolveAnalysis: ((r: { cp: number | null; mate: number | null; bestMove: string }) => void) | null = null;
    let currentResult: { cp: number | null; mate: number | null; bestMove: string } = { cp: null, mate: null, bestMove: '' };

    const handleMessage = (line: string) => {
      if (line === 'uciok') {
        worker.postMessage('setoption name Threads value 4');
        worker.postMessage('setoption name Hash value 64');
        worker.postMessage('isready');
      }
      if (line === 'readyok' && resolveReady) {
        resolveReady();
        resolveReady = null;
      }
      if (line.startsWith('info') && line.includes(' pv ')) {
        const parts = line.split(' ');
        const scoreIdx = parts.indexOf('score');
        if (scoreIdx !== -1 && scoreIdx + 2 < parts.length) {
          if (parts[scoreIdx + 1] === 'cp') {
            currentResult.cp = parseInt(parts[scoreIdx + 2]);
            currentResult.mate = null;
          }
          if (parts[scoreIdx + 1] === 'mate') {
            currentResult.mate = parseInt(parts[scoreIdx + 2]);
            currentResult.cp = null;
          }
        }
        const pvIdx = parts.indexOf('pv');
        if (pvIdx !== -1 && pvIdx + 1 < parts.length) {
          currentResult.bestMove = parts[pvIdx + 1];
        }
      }
      if (line.startsWith('bestmove')) {
        const parts = line.split(' ');
        if (!currentResult.bestMove && parts.length >= 2) {
          currentResult.bestMove = parts[1];
        }
        if (resolveAnalysis) {
          resolveAnalysis({ ...currentResult });
          resolveAnalysis = null;
        }
      }
    };

    worker.onmessage = (e: MessageEvent<string>) => handleMessage(e.data);
    worker.onerror = (e) => reject(new Error(`Stockfish worker error: ${e.message}`));

    resolveReady = () => {
      resolve({
        worker,
        analyze(fen: string, nodes: number) {
          currentResult = { cp: null, mate: null, bestMove: '' };
          return new Promise((res) => {
            resolveAnalysis = res;
            worker.postMessage(`position fen ${fen}`);
            worker.postMessage(`go nodes ${nodes}`);
          });
        },
        analyzeMultiPv(fen: string, nodes: number, numPvs: number) {
          return new Promise((res) => {
            const lines = new Map<number, { pv: string[]; cp: number | null; mate: number | null }>();

            const originalOnMessage = worker.onmessage;
            worker.onmessage = (e: MessageEvent<string>) => {
              const line = e.data;

              if (line.startsWith('info') && line.includes(' pv ')) {
                const parts = line.split(' ');
                const mpvIdx = parts.indexOf('multipv');
                const mpv = mpvIdx !== -1 ? parseInt(parts[mpvIdx + 1]) : 1;

                let cp: number | null = null;
                let mate: number | null = null;
                const scoreIdx = parts.indexOf('score');
                if (scoreIdx !== -1 && scoreIdx + 2 < parts.length) {
                  if (parts[scoreIdx + 1] === 'cp') cp = parseInt(parts[scoreIdx + 2]);
                  if (parts[scoreIdx + 1] === 'mate') mate = parseInt(parts[scoreIdx + 2]);
                }

                const pvIdx = parts.indexOf('pv');
                const pv = pvIdx !== -1 ? parts.slice(pvIdx + 1) : [];
                if (pv.length > 0) lines.set(mpv, { pv, cp, mate });
              }

              if (line.startsWith('bestmove')) {
                worker.onmessage = originalOnMessage;
                worker.postMessage('setoption name MultiPV value 1');
                const sorted = Array.from(lines.entries())
                  .sort(([a], [b]) => a - b)
                  .map(([, v]) => v);
                res({ lines: sorted });
              }
            };

            worker.postMessage(`setoption name MultiPV value ${numPvs}`);
            worker.postMessage(`position fen ${fen}`);
            worker.postMessage(`go nodes ${nodes}`);
          });
        },
        destroy() {
          worker.postMessage('quit');
          worker.terminate();
        },
      });
    };

    worker.postMessage('uci');
  });
}

/**
 * Analyze a single game via the WebSocket analysis server.
 * The client provides Stockfish compute; the server orchestrates.
 */
export async function analyzeGameProxy(
  gameId: string,
  moves: string[],
  nodes: number = 100000,
  onProgress?: (progress: number) => void,
): Promise<FullAnalysis> {
  const sf = await createStockfishWorker();

  try {
    return await new Promise<FullAnalysis>((resolve, reject) => {
      const ws = new WebSocket(`${ANALYSIS_WS_URL}/api/ws/analyze`);

      const cleanup = () => {
        sf.destroy();
        if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
          ws.close();
        }
      };

      ws.onopen = () => {
        ws.send(JSON.stringify({
          type: 'analyze_game',
          game_id: String(gameId),
          moves,
          nodes,
        }));
      };

      ws.onerror = () => {
        cleanup();
        reject(new Error('WebSocket connection failed'));
      };

      ws.onclose = (e) => {
        if (!e.wasClean) {
          reject(new Error('WebSocket closed unexpectedly'));
        }
      };

      ws.onmessage = async (event) => {
        try {
          const msg = JSON.parse(event.data);

          switch (msg.type) {
            case 'eval_batch': {
              // Evaluate all positions sequentially
              const results: { id: number; cp: number | null; mate: number | null; best_move: string }[] = [];
              for (const pos of msg.positions) {
                const r = await sf.analyze(pos.fen, pos.nodes);
                results.push({
                  id: pos.id,
                  cp: r.cp,
                  mate: r.mate,
                  best_move: r.bestMove,
                });
                if (onProgress && msg.positions.length > 0) {
                  onProgress(Math.round((results.length / msg.positions.length) * 70));
                }
              }
              ws.send(JSON.stringify({ type: 'eval_results', results }));
              break;
            }

            case 'eval_multi_pv': {
              const r = await sf.analyzeMultiPv(msg.fen, msg.nodes, msg.multipv);
              ws.send(JSON.stringify({
                type: 'multi_pv_result',
                request_id: msg.request_id,
                lines: r.lines,
              }));
              break;
            }

            case 'zugzwang_test': {
              // Evaluate each position normally and with null-move FEN
              const zugResults: {
                puzzle_idx: number;
                solver_idx: number;
                cp: number | null;
                null_cp: number | null;
                mate: number | null;
                null_mate: number | null;
              }[] = [];

              for (const pos of msg.positions) {
                const normal = await sf.analyze(pos.fen, pos.nodes);
                const nullEval = await sf.analyze(pos.null_fen, pos.nodes);
                zugResults.push({
                  puzzle_idx: pos.puzzle_idx,
                  solver_idx: pos.solver_idx,
                  cp: normal.cp,
                  null_cp: nullEval.cp,
                  mate: normal.mate,
                  null_mate: nullEval.mate,
                });
              }

              ws.send(JSON.stringify({
                type: 'zugzwang_results',
                results: zugResults,
              }));
              break;
            }

            case 'progress': {
              if (onProgress) {
                if (msg.phase === 'eval') {
                  // 0-70% for eval phase (already handled above)
                } else if (msg.phase === 'puzzles' && msg.total > 0) {
                  onProgress(70 + Math.round((msg.current / msg.total) * 25));
                }
              }
              break;
            }

            case 'analysis_complete': {
              if (onProgress) onProgress(100);
              cleanup();
              resolve(msg.result as FullAnalysis);
              break;
            }

            case 'error': {
              cleanup();
              reject(new Error(msg.message));
              break;
            }
          }
        } catch (err) {
          cleanup();
          reject(err);
        }
      };
    });
  } catch (err) {
    sf.destroy();
    throw err;
  }
}

/**
 * Analyze multiple games via parallel WebSocket connections.
 * Each game gets its own WS connection + Stockfish worker.
 */
export async function analyzeGamesBatchProxy(
  games: BatchGameInput[],
  options: {
    nodes?: number;
    onProgress?: (progress: BatchProgress) => void;
    onGameComplete?: (result: BatchGameResult) => void;
    signal?: AbortSignal;
  } = {},
): Promise<BatchGameResult[]> {
  const { nodes = 100000, onProgress, onGameComplete, signal } = options;
  if (games.length === 0) return [];

  const hwConcurrency = navigator.hardwareConcurrency || 4;
  const workerCount = Math.max(2, Math.min(8, Math.floor(hwConcurrency * 0.75), games.length));

  let gamesCompleted = 0;
  let gamesSucceeded = 0;
  let gamesFailed = 0;
  let activeWorkers = 0;
  const results: BatchGameResult[] = [];
  let nextGameIndex = 0;

  function claimNext(): BatchGameInput | null {
    if (nextGameIndex >= games.length) return null;
    return games[nextGameIndex++];
  }

  function reportProgress() {
    onProgress?.({
      gamesCompleted,
      gamesTotal: games.length,
      gamesSucceeded,
      gamesFailed,
      activeWorkers,
    });
  }

  async function workerLoop(): Promise<void> {
    activeWorkers++;
    reportProgress();

    while (true) {
      if (signal?.aborted) break;
      const game = claimNext();
      if (!game) break;

      let result: BatchGameResult;
      try {
        const analysis = await analyzeGameProxy(game.id, game.moves, nodes);
        result = { gameId: game.id, analysis, error: null };
        gamesSucceeded++;
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') break;
        result = {
          gameId: game.id,
          analysis: null,
          error: err instanceof Error ? err.message : 'Unknown error',
        };
        gamesFailed++;
      }

      gamesCompleted++;
      results.push(result);
      onGameComplete?.(result);
      reportProgress();
    }

    activeWorkers--;
    reportProgress();
  }

  const workers: Promise<void>[] = [];
  for (let i = 0; i < workerCount; i++) {
    workers.push(workerLoop());
  }
  await Promise.all(workers);
  reportProgress();

  return results;
}
