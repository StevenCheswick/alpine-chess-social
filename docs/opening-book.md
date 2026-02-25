# Opening Book

The opening book classifies known opening moves as "book" during game analysis so they don't count toward accuracy or show up as mistakes. Two systems consume it:

- **Server** (App Runner): Reads from **prod Postgres** at query time (DB fallback — no bin file in Docker image)
- **Analysis Worker** (Fargate): Reads from **`data/opening_book.bin`** baked into Docker image at build time

## Table Schema

```sql
CREATE TABLE opening_book (
    parent_fen TEXT NOT NULL,     -- Normalized FEN: position + side + castling + ep (no move counters)
    move_san   TEXT NOT NULL,     -- Move in SAN notation (e.g. "h4", "Nf3")
    result_fen TEXT NOT NULL,     -- Normalized FEN after the move is played
    games      INTEGER NOT NULL,  -- Number of master games with this move
    white_wins INTEGER NOT NULL,
    draws      INTEGER NOT NULL,
    black_wins INTEGER NOT NULL,
    avg_rating SMALLINT,          -- Optional
    PRIMARY KEY (parent_fen, move_san)
);
```

**FEN normalization** strips halfmove and fullmove counters — only the first 4 space-separated parts are kept: `position side castling en_passant`. Example: `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3`

## Adding a Move

### 1. Figure out the FENs

You need the **parent FEN** (before the move) and **result FEN** (after the move), both normalized (4 parts, no move counters). Use `-` for en passant if none.

Example for 1.e4 g6 2.d4 Bg7 3.h4:
- Parent: `rnbqk1nr/ppppppbp/6p1/8/3PP3/8/PPP2PPP/RNBQKBNR w KQkq -`
- Move: `h4`
- Result: `rnbqk1nr/ppppppbp/6p1/8/3PP2P/8/PPP2PP1/RNBQKBNR b KQkq -`

### 2. Check if it already exists

```bash
curl -s "https://3vzqmqtxf8.us-east-1.awsapprunner.com/api/opening-book/check?fen=<PARENT_FEN_URL_ENCODED>&san=<MOVE>"
```

Spaces in the FEN become `+` in the URL.

### 3. Insert into prod Postgres

```bash
docker exec chess-postgres psql "postgresql://chess_admin:AlpineChess2026@chess-analysis-db.cohk6egmirrt.us-east-1.rds.amazonaws.com:5432/alpine_chess?sslmode=require" -c "
INSERT INTO opening_book (parent_fen, move_san, result_fen, games, white_wins, draws, black_wins)
VALUES (
  '<parent_fen>',
  '<move_san>',
  '<result_fen>',
  <games>, <white_wins>, <draws>, <black_wins>
)
ON CONFLICT (parent_fen, move_san) DO UPDATE SET
  result_fen = EXCLUDED.result_fen,
  games = EXCLUDED.games,
  white_wins = EXCLUDED.white_wins,
  draws = EXCLUDED.draws,
  black_wins = EXCLUDED.black_wins;
"
```

This takes effect **immediately** for the server's `/api/opening-book/check` endpoint (DB fallback).

### 4. Insert into local Postgres (keep in sync)

```bash
docker exec chess-postgres psql -U chess -d chess_social -c "
INSERT INTO opening_book (parent_fen, move_san, result_fen, games, white_wins, draws, black_wins)
VALUES ('<parent_fen>', '<move_san>', '<result_fen>', <games>, <white_wins>, <draws>, <black_wins>)
ON CONFLICT (parent_fen, move_san) DO UPDATE SET
  result_fen = EXCLUDED.result_fen, games = EXCLUDED.games,
  white_wins = EXCLUDED.white_wins, draws = EXCLUDED.draws, black_wins = EXCLUDED.black_wins;
"
```

### 5. Export the bin file (for the analysis worker)

```bash
cd backend-rust
cargo run -p server --bin export-book
```

This reads from local Postgres (`DATABASE_URL` in `.env`) and writes `data/opening_book.bin`.

### 6. Commit, push, rebuild worker

```bash
git add backend-rust/data/opening_book.bin
git commit -m "Add <move> to opening book for <opening name>"
git push
cd backend-rust
docker build -t 019304715762.dkr.ecr.us-east-1.amazonaws.com/alpine-chess-analysis-worker:latest -f crates/analysis-worker/Dockerfile .
docker push 019304715762.dkr.ecr.us-east-1.amazonaws.com/alpine-chess-analysis-worker:latest
```

Next Batch job picks up the new worker image automatically.

## Gotchas

- **Server doesn't use the bin file** — its Dockerfile doesn't copy it. It falls back to prod Postgres for book lookups. So inserting into prod DB is what matters for the server.
- **Worker only uses the bin file** — it doesn't query the DB for book lookups. So exporting + rebuilding the worker image is what matters for future game analyses.
- **`result_fen` is NOT NULL** — the schema in the old `data/README.md` was missing this column. Inserts without it will fail.
- **FEN must be normalized** — strip move counters (halfmove clock + fullmove number). Only keep 4 parts separated by spaces.
- **Game stats can be approximate** — if you're manually adding a move, reasonable estimates are fine (e.g. 100 games).

## Bulk Import

The opening book was originally built from KingBase PGN files (2500+ Elo, 100+ games per position, up to 60 plies):

```bash
cargo run --release -p server --bin build-book -- <pgn_directory>
```

## Key Files

| What | Path |
|------|------|
| Server book cache | `backend-rust/crates/server/src/book_cache.rs` |
| Worker book cache | `backend-rust/crates/analysis-worker/src/book_cache.rs` |
| Book check API | `backend-rust/crates/server/src/routes/opening_book.rs` |
| Export tool | `backend-rust/crates/server/src/bin/export_book.rs` |
| Build tool (bulk) | `backend-rust/crates/server/src/bin/build_book.rs` |
| Binary file | `backend-rust/data/opening_book.bin` |
| Old README | `backend-rust/data/README.md` |
