# chess_puzzler

A pure Rust chess puzzle classifier. Given a puzzle (a sequence of board positions and moves), it identifies which tactical and positional themes are present. No Stockfish or database dependencies — just the `chess` crate for board representation.

## Data Flow

```
Puzzle (board positions + moves)
  → cook() orchestrator
    → calls ~40 detector functions across 8 modules
  → Vec<TagKind> (list of ~70 possible themes)
```

## Dependencies

- `chess` 3.2 — board representation, move generation, bitboards
- `serde` / `serde_json` — serialization for data structs
- `anyhow` — error handling

## Module Breakdown

### `lib.rs`

Crate root. Re-exports the `chess` crate (`pub use chess;`) so downstream consumers don't need a separate `chess` dependency. Declares 5 modules.

### `analysis.rs` — Move Quality Classification

Pure math functions for evaluating individual moves:

- **`calculate_cp_loss(best, after, is_white, is_checkmate)`** — centipawn loss between the best eval and the actual move's eval, capped at 500
- **`classify_move(cp_loss, is_mate_blunder)`** — maps cp loss to tiers: best (0) / excellent (<10) / good (<50) / inaccuracy (<100) / mistake (<200) / blunder (200+)
- **`calculate_accuracy(total_cp_loss, move_count)`** — overall accuracy percentage using `100 * sqrt(1 / (1 + acpl/100))`
- **`is_mate_blunder()`** — detects when a player loses a forced mate or allows one

Structs: `MoveAnalysis`, `Classifications`

### `board_utils.rs` — Chess Utility Functions

The workhorse module, ported from Python `util.py`. Every detector depends on this:

- **Piece values**: Pawn=1, Knight/Bishop=3, Rook=5, Queen=9, King=99
- **`attacks(board, square)`** — bitboard of squares attacked by a piece
- **`attackers(board, color, square)`** — all pieces of a color attacking a square
- **`pawn_attacks()`** — diagonal pawn attack squares only
- **`pin_direction()`** — returns the pin line if a piece is pinned, otherwise all-bits-set
- **`is_defended()` / `is_hanging()`** — includes x-ray defense through enemy pieces
- **`is_trapped()`** — piece has no safe escape and is attackable
- **`material_count()` / `material_diff()`** — material arithmetic
- **`is_castling_move()`, `king_square()`, `square_distance()`** etc.

### `endgame.rs` — FCE Endgame Classification

Classifies positions into 12 Fundamental Chess Endings categories using bit-flags for piece types present:

- PawnEndings, KnightEndings, BishopEndings, BishopVsKnight, RookEndings, RookVsMinorPiece, RookMinorVsRookMinor, RookMinorVsRook, QueenEndings, QueenVsRook, QueenVsMinorPiece, QueenPieceVsQueen
- `EndgameTracker` — tracks per-segment stats (cp loss, blunders) as a game progresses through endgame phases
- `classify_endgame(board)` — returns `Option<EndgameType>` based on non-pawn piece composition

### `puzzle/mod.rs` — Data Model

- **`TagKind`** — enum with ~70 tactical themes (mate patterns, forks, pins, sacrifices, endgame types, etc.)
- **`PuzzleNode`** — one ply: `board_before`, `board_after`, `chess_move`, `ply`
- **`Puzzle`** — full puzzle: `id`, `mainline` (Vec of PuzzleNode), `pov` (solver's color), `cp` (eval)
  - `solver_moves()` — odd-indexed nodes (solver's actual decisions)
  - `opponent_moves()` — even-indexed nodes (forced responses)

### `puzzle/cook.rs` — The Orchestrator

`cook(puzzle) -> Vec<TagKind>` calls all detectors in a specific order:

1. **Mate detection** — MateIn1 through MateIn5, then an elif chain of 15 named mate patterns
2. **Eval classification** — Crushing (>600cp), Advantage (>200cp), or Equality
3. **All other detectors** — attraction, deflection, sacrifice, fork, pin, discovered attack, etc.
4. **Length tags** — OneMove / Short / Long / VeryLong based on solver move count

### `puzzle/extraction.rs` — UCI Parsing

- `parse_uci_move(board, "e2e4")` — converts UCI string to `ChessMove`
- Constants: `BLUNDER_THRESHOLD=200`, `MIN_PUZZLE_CP=100`, `MIN/MAX_PUZZLE_LENGTH=2/20`

## Tactics Modules

Eight files under `tactics/`, organized by category:

| Module | Detectors |
|--------|-----------|
| **simple.rs** | double_check, double_checkmate, en_passant, castling, promotion, under_promotion, mate_in, corner_mate, advanced_pawn, check_escape |
| **mate_patterns.rs** | 15 named mates: smothered, back_rank, anastasia, hook, arabian, boden/double_bishop, dovetail, balestra, blind_swine, kill_box, morphys, opera, pillsburys, triangle, vukovic |
| **material.rs** | sacrifice (+ piece type identification), exposed_king, piece_endgame, queen_rook_endgame |
| **attacks.rs** | fork, hanging_piece, trapped_piece, overloading, capturing_defender |
| **pins.rs** | pin_prevents_attack, pin_prevents_escape |
| **line_geometry.rs** | discovered_check, discovered_attack, windmill, x_ray, skewer |
| **positional.rs** | quiet_move, defensive_move, attraction, deflection, interference, self_interference, intermezzo, clearance, zugzwang |
| **side_attacks.rs** | greek_gift, attacking_f2_f7, kingside_attack, queenside_attack |

## Key Design Decisions

- **All detectors are pure functions** — `fn(puzzle: &Puzzle) -> bool` (or `Option<TagKind>` for detectors that return a specific tag)
- **No side effects** — `cook()` builds and returns a tag list, nothing else
- **Python-compatible** — originally validated against 2,683 puzzles with 100% match rate vs the Python implementation
- **Pseudo-legal move generation in pins.rs** — Python-chess's `pseudo_legal_moves` ignores pins, so the Rust port generates attacks manually to match that behavior. This is critical for correct pin detection.
- **Pawn attack subtlety** — diagonal captures only count when an enemy is present (or en passant); forward pushes only when unblocked
- **En passant edge case** — `board.piece_on(dest)` returns None for en passant captures, so they're detected via piece count change

## Origin

Ported from the [lichess-puzzler](https://github.com/lichess-org/lichess-puzzler) Python tagger (`tagger/cook.py`, `tagger/util.py`), then extended with game analysis and endgame classification. The tactical classification engine was validated to produce identical output on all 2,683 test puzzles.
