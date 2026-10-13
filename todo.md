# TODO

## Analysis Bugs (from 2026-10-11 algorithm review)
- [ ] **Puzzles generated for the losing side** — `analyzer.rs` flags any 200cp+ drop as a blunder (e.g. White +900 → +700), and the `MIN_PUZZLE_CP` check uses `cp.abs()`, so Black gets a "Crushing" puzzle while still lost. Keep the sign from the solver's POV and require the solver to be clearly better after the blunder.
- [ ] **Puzzle lines can end on the opponent's move** — `extend_puzzle_line` breaks at an ambiguous solver turn after the opponent reply was already pushed. Pop the trailing opponent move so puzzles always end on a solver move (as the Lichess puzzler does).
- [ ] **Engine's top move graded below "best"** — before/after evals come from separate fixed-node searches, so a played move equal to `best_moves[i]` can show cp_loss > 0. If the played move == SF best move, force "best" with 0 loss.
- [ ] **Accuracy inflated by book/forced moves** — they count toward move_count with 0 loss (`analyzer.rs` stats loop). Exclude them from the accuracy calculation.
- [ ] **Engine line move numbers skip** — uncommitted `pvToSan` change in `frontend/js/analysis.js` increments `moveNum` after both White and Black moves (`1. e4 e5 3. Nf3`). Only increment after Black's move.

## Analysis Algorithm Improvements
- [ ] **Grade moves by win probability instead of raw centipawns** — `win% = 50 + 50 * (2 / (1 + exp(-0.00368208 * cp)) - 1)`. Inaccuracy / mistake / blunder = 10 / 20 / 30 point drop in win%. Per-move accuracy `103.17 * exp(-0.04354 * win%_drop) - 3.17`, game accuracy = Lichess-style blend of volatility-weighted mean and harmonic mean. Note: changes accuracy for existing games — re-run analysis or store a grading version.
- [ ] **Use win% for puzzle "only move" check** — replace `advantage < 100` in `extend_puzzle_line` with a win-chance gap.
- [ ] **"Great move / only move" classification** — run the main eval pass with MultiPV 2 to detect when only one move holds the position.
- [ ] **Verify book FEN agreement** — analyzer builds FENs with the `chess` crate, book was built with shakmaty legal en passant mode. Add a unit test for edge cases (e.g. pinned pawn that can't actually capture en passant).

## Trainer Generator Improvements (`generate_puzzles.py`)
Philosophy: fast and light. ~95% confidence on "most common" is enough; what must be right is that every puzzle is accurate.

Done (2026-10-12, uncommitted, not yet run end to end on a real opening):
- [x] **G6: Same puzzle repeated within a set** — 188 of 1,712 saved puzzles (11%) started from a position another puzzle already covered, reached by a different move order. New `merge_transpositions` step after Phase 1 keeps the most-played version, sums the game counts and lists the others in `also_reached_by`. `dedup_puzzles` now ignores move counters. Different blunders from the same position still give separate puzzles.
- [x] **G2: Moves with no stored eval hide later mistakes** — `find-mistakes` can't judge any move after a move with no RocksDB eval (Evans: 501 such moves, up to 48 games, including main-line positions). New `--eval-cache` arg passes a `backfill-evals` JSON through (e.g. `../lila-openingexplorer/evals_evans.json`, covers 418/501). Runs without a cache print a warning; puzzles record which cache was used.

Open:
- [ ] **Backfill eval cache for every opening run** — only the Evans (2,916 entries) and Dragon Black (100) have caches. Consider a `--backfill` switch that runs `backfill-evals` before Phase 1 (adds Stockfish time per run).
- [ ] **Cache positions by FEN** — transpositions currently get a fresh 200K-node MultiPV 3 search + Maia call.
- [ ] **Always include the opponent's best defense** — opponent moves are Maia top + one extra at 20%+, so SF's best defense (e.g. 12% on Maia) can be skipped. Include it when above ~5%.
- [ ] **Replace hand-tuned cp cutoff buckets with win%** — the `root_eval >= 625 / 400 / 300 / 150` advantage-secured buckets and dynamic CP_GAP could become a single win% threshold.

- [ ] **Explore: "tempting mistake" test** — replace the Phase 2 Maia filter's `nodes=100` search share with Maia's instant guess (`nodes=1`) on the mistake move, and require a fairly high share (e.g. 10%+). Natural-looking moves signal real opening flaws; mouse slips score near 0.

Parked / decided:
- **G1: Budget-cut lines treated as finished** — PARKED until it happens. If a tree hits the 100-node budget, cut-short lines get relabelled `advantage_secured` (`_opponent_node`) and `prune_tree`'s "prefer clean" rule can delete SF#1 for an unexplored move. Rebuilt 830 saved puzzles: largest tree used 54/100, none came close. The generator now prints `!!! G1 BUDGET BUG TRIGGERED` with the FEN if it ever happens. Agreed design if needed: split the budget at forks with leftovers rolling forward; a budget-cut line never counts as "simple". (This also covers the old "expand most likely lines first" item.)
- **Pruning intent (decided)** — teach the simplest way to secure the win; moves pruned for complexity count as wrong. Multiple correct answers are wanted when they're about as simple. Possible later tweak if puzzles show bad picks: measure complexity as lines to learn + non-forced moves (7 forcing moves isn't complex), mate in 1 counts as the simplest finish, a draw never does.
- **G5: Puzzles from already-lost positions** — CLOSED, not a bug. Follow-ups after an earlier blunder, walking into mate from a lost position, and blunders from normal −1 gambit positions are all wanted. Note: follow-up puzzles (e.g. 4…h6 5.Nxf7 Qe7 6.Nxh8) only exist because mistakes are measured against the previous move's eval; don't switch to a "best move in position" baseline without checking this.
- **G3: 1000-candidate stop doesn't guarantee the exact top 100** — DROPPED, ~95% confidence is enough.

## New Features
- [ ] **FEN → punishment puzzle page** — paste a FEN (position right after a mistake, punisher to move), press Enter, get a playable puzzle built with `generate_puzzles.py`'s `build_tree` / `prune_tree` / `trim_opponent_leaves`. Working prototype parked at `Desktop/fen-puzzle/` (aiohttp server on :8090 + Chessground page); verified identical to the saved Traxler puzzle. Could become an "make a puzzle from this position" button on the analysis page.
- [ ] **Analysis board page** — standalone page to paste a FEN or PGN and analyze with Stockfish (WASM engine already exists), separate from game analysis.

## Backend Cleanup
- [x] Remove social feed/posts from backend (done)

## Server-Side Analysis (Premium Feature)
- [x] Fix WebSocket coordination for bulk analysis on App Runner

## Sacrifice Tag Detection
- [ ] Detect and tag sacrifice moves during analysis:
  - Queen sacrifice
  - Rook sacrifice
  - Minor piece sacrifice (bishop/knight for pawn)
  - Exchange sacrifice (rook for minor piece)
  - Pawn sacrifice (gambits)

## Chess.com Sync
- [x] On re-sync, fetch all games since last sync (not just current month)

## Costliest Opening Habits
- [x] Filter out known opening theory from results — reclassify endpoint now marks book moves, dashboard filters them out

## Deepest Opening
- [ ] Add ECO code support for deepest opening feature

## Endgame Type Detection
- [ ] Detect and tag common endgame types during analysis:
  - KP vs K (King + Pawn vs King)
  - Lucena position
  - Philidor position
  - KBN vs K (Knight and Bishop mate)
  - Basic rook endgames (rook + pawn vs rook)
  - Queen vs Rook
  - Opposite-color bishop endgames

## Explorer DB (lila-openingexplorer)
- [ ] **Write evals back to RocksDB** — `backfill-evals` generates SF evals for missing positions but dumps to JSON. No write-back mechanism exists. Need to add a merge operation or new binary that updates the eval field in-place. Requires Rust build (currently broken on Windows due to stdbool.h/clang issue — try from VS Developer Command Prompt). ~100K+ moves with 2+ games are missing evals in the Dragon alone.

## Opening Mistake Trainer
- [ ] Fix eval bar in trainer
- [ ] Deduplicate puzzles: if puzzle A's post-mistake FEN appears as a node inside puzzle B's tree, drop puzzle A (it's a subset)
- [ ] **Spaced repetition** — track which puzzles the user got wrong and resurface them more frequently
- [ ] **Speed mode** — timed drills, see how fast the user can complete the main lines
- [x] **Hard move evals to White's POV** — fixed: generator now outputs evals from White's POV

## Dashboard
- [x] Add "Choke Rate" metric — track how often a player loses a winning position
- [ ] **Dashboard metrics table** — store all computed dashboard stats in a dedicated DB table so they persist across server restarts and only recompute on trigger (new game analyzed, manual refresh). Eliminates cold-cache ~8s loads.

## Performance
- [ ] **Dashboard precompute tables** — Dashboard stats API takes ~8s (uncached). Create `game_opening_stats` and `game_opening_mistakes` tables with precomputed per-game opening data so dashboard queries are simple SELECTs instead of JSONB explosions. Populate on analysis save, one-time backfill via SQL for existing games.
- [x] **Move reclassify to local binary**
- [ ] Speed up Games page load — 5 parallel API calls on mount (~435ms each). Options: prefetch on nav hover, skeleton UI, or edge caching via CloudFront

## New Game Tag Ideas
- [ ] **King Walk** — detect checkmates where opponent's king was hunted far from home rank through a series of checks. Needs eval swing filter: no single user move during the hunt should swing eval from winning to losing.
- [ ] **Roller Coaster Game** — find games where the eval swaps between +/- the most times. Track how many times the advantage flips sides throughout the game (e.g., +3 → -2 → +4 → -1 = 3 swaps). Tag the wildest back-and-forth games.

## UI Polish
- [ ] Implement move/game sounds
