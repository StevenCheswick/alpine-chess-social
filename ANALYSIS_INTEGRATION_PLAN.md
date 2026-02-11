# Analysis Integration Plan: Client-Proxy Architecture

## Goal

Integrate the lichess-puzzler unified analyzer into chess-social-media using a client-as-proxy architecture. The client runs Stockfish WASM (already implemented). The server controls all analysis logic (classification, puzzle extraction, endgame tracking, theme tagging). No Stockfish runs server-side.

---

## Current State

### What exists in chess-social-media:

**Frontend (already built):**
- `stockfishEngine.ts` — full Stockfish WASM wrapper with multi-threading, SharedArrayBuffer detection
- `analysisService.ts` — complete client-side analysis loop (eval every position, classify moves, compute accuracy)
- `useStockfish` hook — React hook for single-position analysis
- `analysis.ts` types — `MoveAnalysis`, `GameAnalysis`, `MoveClassifications`, batch types
- Batch analysis with worker pool (`analyzeGamesBatch`)

**Backend (already built):**
- `POST /api/games/{game_id}/analysis` — saves analysis results
- `GET /api/games/{game_id}/analysis` — retrieves stored analysis
- `game_analysis` table — stores accuracy, classifications, moves JSON
- No WebSocket support currently

### What exists in lichess-puzzler/rust-analyzer:

- `unified.rs` — single-pass analysis + puzzle extraction + endgame tracking
- `puzzle/extraction.rs` — `extend_puzzle_line()` with multi-PV Stockfish
- `puzzle/cook.rs` — 40+ tactical theme classifiers
- `endgame.rs` — FCE endgame categorization + segment tracking
- `analysis.rs` — cp loss, move classification, accuracy calculation

---

## Architecture

```
┌─────────────────────────────┐        WebSocket         ┌─────────────────────────────┐
│         FRONTEND            │◄────────────────────────►│          BACKEND            │
│                             │                           │                             │
│  Stockfish WASM Worker      │   1. Server sends FENs    │  Analysis Orchestrator      │
│  - Receives FEN + config    │   2. Client returns evals │  - CP loss calculation      │
│  - Returns eval/bestmove    │   3. Server sends multiPV │  - Move classification      │
│  - Generic eval worker      │   4. Client returns lines │  - Blunder detection        │
│  - No classification logic  │   5. Server returns JSON  │  - Puzzle line extension    │
│                             │                           │  - cook() theme tagging     │
│  Analysis UI                │                           │  - Endgame segmentation     │
│  - Displays results         │                           │  - Accuracy calculation     │
│  - Move list + eval bar     │                           │                             │
│  - Puzzle viewer            │                           │  Database                   │
│  - Endgame report           │                           │  - game_analysis table      │
│                             │                           │  - puzzles + endgame stored │
└─────────────────────────────┘                           └─────────────────────────────┘
```

---

## Implementation Steps

### Step 1: WebSocket Infrastructure

**Backend** — Add WebSocket endpoint to FastAPI:

```
WS /ws/analyze
```

Connection flow:
1. Client connects with JWT auth token as query param
2. Server validates token, accepts connection
3. Bidirectional JSON messages for the analysis session
4. Connection stays open for the duration of one game analysis
5. Server closes connection when analysis is complete or on error

**Frontend** — New `AnalysisWebSocket` service:

Replace the current `analyzeGameCore()` loop with a WebSocket client that:
1. Connects to `ws://localhost:8000/ws/analyze`
2. Receives position eval requests from server
3. Runs Stockfish on each position
4. Sends results back
5. Receives final analysis JSON when complete

### Step 2: WebSocket Message Protocol

All messages are JSON with a `type` field.

#### Server → Client Messages:

```typescript
// Request batch evaluation of positions
{
  type: "eval_batch",
  positions: [
    { id: number, fen: string, nodes: number }
  ]
}

// Request multi-PV evaluation (for puzzle extension)
{
  type: "eval_multipv",
  request_id: number,
  fen: string,
  nodes: number,
  multipv: number  // typically 2
}

// Request single evaluation
{
  type: "eval_single",
  request_id: number,
  fen: string,
  nodes: number
}

// Analysis complete — final results
{
  type: "analysis_complete",
  result: {
    moves: MoveAnalysis[],
    white_accuracy: number,
    black_accuracy: number,
    white_avg_cp_loss: number,
    black_avg_cp_loss: number,
    white_classifications: MoveClassifications,
    black_classifications: MoveClassifications,
    puzzles: PuzzleOutput[],
    endgame_segments: EndgameSegment[]
  }
}

// Progress update
{
  type: "progress",
  phase: "eval" | "puzzles",
  current: number,
  total: number
}

// Error
{
  type: "error",
  message: string
}
```

#### Client → Server Messages:

```typescript
// Start analysis for a game
{
  type: "analyze_game",
  game_id: number,
  moves: string[],       // SAN moves from the game
  nodes: number          // nodes per position (client's choice)
}

// Batch eval results
{
  type: "eval_results",
  results: [
    { id: number, cp: number, mate: number | null, best_move: string }
  ]
}

// Multi-PV result
{
  type: "multipv_result",
  request_id: number,
  lines: [
    { pv: string[], cp: number, mate: number | null }
  ]
}

// Single eval result
{
  type: "eval_result",
  request_id: number,
  cp: number,
  mate: number | null,
  best_move: string
}
```

### Step 3: Server-Side Analysis Orchestrator

New module in the backend that implements the unified analysis loop but delegates all Stockfish calls to the WebSocket client.

**Python backend** (`backend/src/analysis_orchestrator.py`):

The orchestrator mirrors the logic from `rust-analyzer/src/unified.rs`:

```
async def orchestrate_analysis(websocket, moves, nodes):
    1. Convert SAN moves to FENs (using python-chess)
    2. Send eval_batch for all positions
    3. Receive all evals from client
    4. For each move:
       a. Calculate CP loss from adjacent evals
       b. Classify move (best/excellent/good/inaccuracy/mistake/blunder)
       c. Track endgame segments (classify_endgame on each position)
       d. If blunder detected → start puzzle extraction
    5. For each blunder (puzzle extraction):
       a. Send eval_multipv to client for the post-blunder position
       b. Receive multi-PV result
       c. Check if best move is clearly best (gap ≥ 100cp)
       d. If yes, send next position for opponent's response
       e. Continue extending until puzzle is complete or ambiguous
       f. Run cook() theme classification on the puzzle (server-side, no Stockfish needed)
    6. Compute accuracy, aggregate classifications
    7. Send analysis_complete with full results
    8. Save to database
```

Key logic ported from rust-analyzer:
- `calculate_cp_loss()` — from `analysis.rs`
- `classify_move()` — from `analysis.rs`
- `calculate_accuracy()` — from `analysis.rs`
- `classify_endgame()` — from `endgame.rs`
- `EndgameTracker` — from `endgame.rs`
- `extend_puzzle_line()` logic — from `extraction.rs` (but Stockfish calls go over WebSocket)
- `cook()` — from `puzzle/cook.rs` (runs entirely server-side, no Stockfish)

### Step 4: Database Schema Updates

Extend `game_analysis` table to store puzzles and endgame data:

```sql
ALTER TABLE game_analysis ADD COLUMN puzzles TEXT;           -- JSON array of puzzles
ALTER TABLE game_analysis ADD COLUMN endgame_segments TEXT;  -- JSON array of segments
```

Each puzzle stored as:
```json
{
  "fen": "...",
  "moves": ["e2e4", "d7d5", ...],
  "cp": 350,
  "themes": ["Fork", "Advantage"]
}
```

Each endgame segment stored as:
```json
{
  "endgame_type": "Rook Endings",
  "entry_move": 26,
  "entry_eval": 66,
  "white_moves": 30,
  "white_cp_loss": 889,
  "white_blunders": 1,
  "black_moves": 31,
  "black_cp_loss": 1382,
  "black_blunders": 3,
  "mistakes": [...]
}
```

### Step 5: Frontend Changes

**New: `services/analysisProxy.ts`**

Replaces `analysisService.ts` for server-orchestrated analysis. The client becomes a thin Stockfish worker:

```typescript
class AnalysisProxy {
  private ws: WebSocket;
  private engine: StockfishAnalyzer;

  async analyzeGame(gameId: number, moves: string[], nodes: number,
                     onProgress: (p) => void): Promise<FullAnalysis> {
    // 1. Connect WebSocket
    // 2. Send analyze_game message
    // 3. Listen for eval requests, run Stockfish, send results
    // 4. Listen for progress updates, call onProgress
    // 5. Receive analysis_complete, return results
  }
}
```

**Keep: `analysisService.ts`**

The existing client-side analysis stays as a fallback for:
- Offline mode
- When server is unavailable
- Quick single-position analysis (useStockfish hook)

**New types** (`types/analysis.ts` additions):

```typescript
interface PuzzleOutput {
  fen: string;
  moves: string[];
  cp: number;
  themes: string[];
}

interface EndgameMistake {
  fen: string;
  move_uci: string;
  best_move: string;
  cp_loss: number;
  classification: MoveClassification;
  move_number: number;
  is_white: boolean;
}

interface EndgameSegment {
  endgame_type: string;
  entry_move: number;
  entry_eval: number;
  white_moves: number;
  white_cp_loss: number;
  white_blunders: number;
  black_moves: number;
  black_cp_loss: number;
  black_blunders: number;
  mistakes: EndgameMistake[];
}

interface FullAnalysis extends GameAnalysis {
  puzzles: PuzzleOutput[];
  endgame_segments: EndgameSegment[];
}
```

### Step 6: Batch Analysis Adaptation

The current `analyzeGamesBatch()` spawns multiple Stockfish workers for parallel game analysis. With the server-proxy model:

**Option A: One WebSocket per game** — Open N concurrent WebSocket connections, each analyzing one game. Server handles N concurrent orchestrations. Simple but N connections.

**Option B: Multiplexed WebSocket** — Single WebSocket with game IDs on each message. Server runs N game analyses concurrently on one connection. More complex but cleaner.

**Recommendation:** Option A for simplicity. Each connection is independent and maps cleanly to the existing batch worker loop.

---

## What Stays Server-Side (Proprietary)

- CP loss thresholds (what makes a "blunder" vs "mistake")
- Accuracy formula
- Puzzle extraction criteria (blunder threshold, min length, min CP)
- Puzzle line extension logic (when to stop, clarity threshold)
- cook() theme classification (40+ tactical patterns)
- Endgame categorization (FCE categories, segment tracking)
- Endgame mistake thresholds

## What the Client Sees

- A generic Stockfish eval worker
- "Evaluate this FEN" / "Multi-PV this FEN" requests
- The final analysis results (but not how they were computed)

---

## Migration Path

1. **Phase 1:** Add WebSocket endpoint + orchestrator. Keep existing client-side analysis as default. Add a "Server Analysis" toggle/option.
2. **Phase 2:** Port classification logic to Python backend (or call rust-analyzer as subprocess/FFI).
3. **Phase 3:** Make server analysis the default. Client-side becomes fallback.
4. **Phase 4:** Add puzzle viewer and endgame report UI components.

---

## Open Questions

- **Python or Rust backend?** The Python backend is active. The Rust backend exists but mirrors it. Classification logic is already in Rust (lichess-puzzler). Options:
  - Port cook() + endgame to Python (quick, matches current backend)
  - Use Rust backend with the existing code (already done, but backend-rust is WIP)
  - Call rust-analyzer as a subprocess from Python (ugly but works)
  - Compile rust-analyzer to a Python extension via PyO3 (clean but setup cost)

- **Node count:** Should the server specify nodes, or let the client choose based on hardware? Currently the client chooses (100k default in analysisService.ts).

- **Caching:** Should we cache eval results in the database for positions seen across multiple users' games? Could save significant client compute for common openings.
