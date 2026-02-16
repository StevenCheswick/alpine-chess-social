# TODO

## Backend Cleanup
- [ ] Remove social feed/posts from backend:
  - `routes/posts.rs` (if exists)
  - `db/posts.rs` (if exists)
  - Post-related database tables
  - Post-related API endpoints

## Sacrifice Tag Detection
- [ ] Detect and tag sacrifice moves during analysis:
  - Queen sacrifice
  - Rook sacrifice
  - Minor piece sacrifice (bishop/knight for pawn)
  - Exchange sacrifice (rook for minor piece)
  - Pawn sacrifice (gambits)

## Chess.com Sync
- [ ] On re-sync, fetch all games since last sync (not just current month) — currently misses games if user doesn't sync for multiple months

## Costliest Opening Habits
- [ ] Filter out known opening theory from results — currently established lines like the Evans Gambit show up as "blunders" because they lose some eval, but they're legitimate theory moves, not repeated mistakes

## Endgame Type Detection
- [ ] Detect and tag common endgame types during analysis:
  - KP vs K (King + Pawn vs King)
  - Lucena position
  - Philidor position
  - KBN vs K (Knight and Bishop mate)
  - Basic rook endgames (rook + pawn vs rook)
  - Queen vs Rook
  - Opposite-color bishop endgames
