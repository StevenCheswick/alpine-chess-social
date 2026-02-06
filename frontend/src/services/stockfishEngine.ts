/**
 * Stockfish Engine Manager
 *
 * Handles Stockfish WASM initialization with multi-threading support.
 * Uses SharedArrayBuffer when available for optimal performance.
 *
 * Based on Lichess's approach:
 * - Feature detection for SharedArrayBuffer, Atomics, and growable memory
 * - Configurable thread count based on navigator.hardwareConcurrency
 * - Fallback to single-threaded mode when threading unavailable
 */

export const STOCKFISH_PATH = '/stockfish/stockfish.js';

// Default configuration
const DEFAULT_HASH_MB = 64;
const DEFAULT_MULTI_PV = 1;

export interface StockfishConfig {
  threads?: number;
  hashMb?: number;
  multiPv?: number;
}

export interface EngineCapabilities {
  supportsThreads: boolean;
  maxThreads: number;
  deviceMemoryMb: number | null;
}

/**
 * Detect if the browser supports multi-threaded WebAssembly
 * Based on Lichess's wasmThreadsSupported() implementation
 */
export function detectThreadingSupport(): EngineCapabilities {
  let supportsThreads = false;
  let maxThreads = 1;
  let deviceMemoryMb: number | null = null;

  try {
    // 1. Check WebAssembly 1.0 support
    const wasmHeader = Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00);
    if (typeof WebAssembly !== 'object' || typeof WebAssembly.validate !== 'function') {
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }
    if (!WebAssembly.validate(wasmHeader)) {
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // 2. Check SharedArrayBuffer
    if (typeof SharedArrayBuffer !== 'function') {
      console.log('[Stockfish] SharedArrayBuffer not available - using single thread');
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // 3. Check Atomics
    if (typeof Atomics !== 'object') {
      console.log('[Stockfish] Atomics not available - using single thread');
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // 4. Test shared WebAssembly.Memory
    const testMem = new WebAssembly.Memory({ shared: true, initial: 8, maximum: 16 });
    if (!(testMem.buffer instanceof SharedArrayBuffer)) {
      console.log('[Stockfish] Shared memory not SharedArrayBuffer - using single thread');
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // 5. Test structured cloning (required for postMessage)
    try {
      window.postMessage(testMem, '*');
    } catch {
      console.log('[Stockfish] Structured cloning failed - using single thread');
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // 6. Test growable shared memory
    try {
      testMem.grow(8);
    } catch {
      console.log('[Stockfish] Memory growth failed - using single thread');
      return { supportsThreads: false, maxThreads: 1, deviceMemoryMb };
    }

    // All checks passed - threading is supported
    supportsThreads = true;

    // Get optimal thread count
    maxThreads = navigator.hardwareConcurrency || 1;
    // Cap at reasonable maximum (Stockfish supports up to 32)
    maxThreads = Math.min(maxThreads, 32);

    // Get device memory if available
    if ('deviceMemory' in navigator) {
      deviceMemoryMb = (navigator as Navigator & { deviceMemory?: number }).deviceMemory! * 1024;
    }

    console.log(`[Stockfish] Threading supported - ${maxThreads} threads available`);
  } catch (e) {
    console.log('[Stockfish] Threading detection failed:', e);
  }

  return { supportsThreads, maxThreads, deviceMemoryMb };
}

// Cached capabilities
let cachedCapabilities: EngineCapabilities | null = null;

export function getEngineCapabilities(): EngineCapabilities {
  if (!cachedCapabilities) {
    cachedCapabilities = detectThreadingSupport();
  }
  return cachedCapabilities;
}

/**
 * Get recommended thread count for analysis
 * Uses half of available cores to keep UI responsive
 */
export function getRecommendedThreads(): number {
  const caps = getEngineCapabilities();
  if (!caps.supportsThreads) return 1;

  // Use half the cores (min 1, max 8 for balance)
  const recommended = Math.max(1, Math.floor(caps.maxThreads / 2));
  return Math.min(recommended, 8);
}

/**
 * Get recommended hash size in MB
 */
export function getRecommendedHashMb(): number {
  const caps = getEngineCapabilities();

  if (caps.deviceMemoryMb) {
    // Use ~10% of device memory, capped at 256MB
    return Math.min(256, Math.floor(caps.deviceMemoryMb * 0.1));
  }

  return DEFAULT_HASH_MB;
}

export type EngineMessageHandler = (line: string) => void;

/**
 * Stockfish Engine wrapper with threading support
 */
export class StockfishEngine {
  private worker: Worker | null = null;
  private messageHandler: EngineMessageHandler | null = null;
  private isInitialized = false;
  private config: Required<StockfishConfig>;
  private capabilities: EngineCapabilities;

  constructor(config: StockfishConfig = {}) {
    this.capabilities = getEngineCapabilities();

    // Apply defaults with threading awareness
    const defaultThreads = this.capabilities.supportsThreads
      ? getRecommendedThreads()
      : 1;

    this.config = {
      threads: config.threads ?? defaultThreads,
      hashMb: config.hashMb ?? getRecommendedHashMb(),
      multiPv: config.multiPv ?? DEFAULT_MULTI_PV,
    };
  }

  /**
   * Initialize the Stockfish engine
   */
  async init(): Promise<void> {
    if (this.isInitialized) return;

    return new Promise((resolve, reject) => {
      try {
        this.worker = new Worker(STOCKFISH_PATH);

        const handleReady = (e: MessageEvent<string>) => {
          const line = e.data;

          if (line === 'uciok') {
            // Configure engine options
            this.worker!.postMessage(`setoption name Threads value ${this.config.threads}`);
            this.worker!.postMessage(`setoption name Hash value ${this.config.hashMb}`);
            this.worker!.postMessage(`setoption name MultiPV value ${this.config.multiPv}`);
            this.worker!.postMessage('setoption name UCI_ShowWDL value true');
            this.worker!.postMessage('isready');
          }

          if (line === 'readyok') {
            this.isInitialized = true;
            console.log(`[Stockfish] Engine ready (${this.config.threads} threads, ${this.config.hashMb}MB hash)`);

            // Switch to user's message handler
            this.worker!.onmessage = (e) => {
              this.messageHandler?.(e.data);
            };

            resolve();
          }
        };

        this.worker.onmessage = handleReady;

        this.worker.onerror = (e) => {
          console.error('[Stockfish] Worker error:', e);
          reject(new Error(`Stockfish worker error: ${e.message}`));
        };

        // Start UCI initialization
        this.worker.postMessage('uci');
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Set the message handler for engine output
   */
  setMessageHandler(handler: EngineMessageHandler): void {
    this.messageHandler = handler;
  }

  /**
   * Send a UCI command to the engine
   */
  postMessage(command: string): void {
    if (!this.worker) {
      console.error('[Stockfish] Engine not initialized');
      return;
    }
    this.worker.postMessage(command);
  }

  /**
   * Update engine configuration (threads, hash, multiPV)
   */
  setOption(name: string, value: number | string | boolean): void {
    if (!this.worker) return;
    this.worker.postMessage(`setoption name ${name} value ${value}`);
  }

  /**
   * Update threads (respects threading capabilities)
   */
  setThreads(threads: number): void {
    if (!this.capabilities.supportsThreads) {
      console.log('[Stockfish] Threading not supported, ignoring setThreads');
      return;
    }
    const capped = Math.min(threads, this.capabilities.maxThreads);
    this.config.threads = capped;
    this.setOption('Threads', capped);
  }

  /**
   * Update hash size
   */
  setHash(hashMb: number): void {
    this.config.hashMb = hashMb;
    this.setOption('Hash', hashMb);
  }

  /**
   * Update MultiPV
   */
  setMultiPv(multiPv: number): void {
    this.config.multiPv = multiPv;
    this.setOption('MultiPV', multiPv);
  }

  /**
   * Stop current search
   */
  stop(): void {
    this.postMessage('stop');
  }

  /**
   * Start a new game (clear hash, etc.)
   */
  newGame(): void {
    this.postMessage('ucinewgame');
    this.postMessage('isready');
  }

  /**
   * Get current configuration
   */
  getConfig(): Required<StockfishConfig> {
    return { ...this.config };
  }

  /**
   * Get engine capabilities
   */
  getCapabilities(): EngineCapabilities {
    return { ...this.capabilities };
  }

  /**
   * Check if engine is ready
   */
  get ready(): boolean {
    return this.isInitialized;
  }

  /**
   * Destroy the engine and release resources
   */
  destroy(): void {
    if (this.worker) {
      this.worker.postMessage('quit');
      this.worker.terminate();
      this.worker = null;
    }
    this.isInitialized = false;
    this.messageHandler = null;
  }
}

/**
 * Create and initialize a Stockfish engine
 */
export async function createEngine(config?: StockfishConfig): Promise<StockfishEngine> {
  const engine = new StockfishEngine(config);
  await engine.init();
  return engine;
}
