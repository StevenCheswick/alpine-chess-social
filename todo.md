# TODO

## Backend Cleanup
- [x] Remove social feed/posts from backend (done)

## Server-Side Analysis (Premium Feature)
- [ ] Fix WebSocket coordination for bulk analysis on App Runner
  - App Runner doesn't support WebSocket upgrades (Envoy returns 403)
  - Options: HTTP polling protocol OR API Gateway WebSocket API
  - Client-side Stockfish WASM works fine as free version

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
