# TODO

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
- [ ] Add "Choke Rate" metric — track how often a player loses a winning position

## Performance
- [ ] **Dashboard precompute tables** — Dashboard stats API takes ~8s (uncached). Create `game_opening_stats` and `game_opening_mistakes` tables with precomputed per-game opening data so dashboard queries are simple SELECTs instead of JSONB explosions. Populate on analysis save, one-time backfill via SQL for existing games.
- [x] **Move reclassify to local binary**
- [ ] Speed up Games page load — 5 parallel API calls on mount (~435ms each). Options: prefetch on nav hover, skeleton UI, or edge caching via CloudFront

## New Game Tag Ideas
- [ ] **King Walk** — detect checkmates where opponent's king was hunted far from home rank through a series of checks. Needs eval swing filter: no single user move during the hunt should swing eval from winning to losing.
- [ ] **Roller Coaster Game** — find games where the eval swaps between +/- the most times. Track how many times the advantage flips sides throughout the game (e.g., +3 → -2 → +4 → -1 = 3 swaps). Tag the wildest back-and-forth games.

## UI Polish
- [ ] Implement move/game sounds
