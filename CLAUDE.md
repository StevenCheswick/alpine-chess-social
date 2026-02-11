# Alpine Chess - Development Guide

## Starting the Full Stack

Before doing any development, testing, or debugging, **all three services must be running**:

1. **Postgres** (Docker):
   ```
   docker start chess-postgres
   ```
2. **Backend** (Rust/Axum, port 8000):
   ```
   cd backend-rust && cargo run -p server --bin server
   ```
3. **Frontend** (React/Vite, port 5173):
   ```
   cd frontend && npm run dev
   ```

Start them in this order — backend needs Postgres, frontend needs backend.

## Integration Tests

Integration tests run against the live server (not mocked). The full stack must be up before running them:

```
cd backend-rust && cargo test --test auth_test
```

Test files live in `backend-rust/tests/` with shared helpers in `backend-rust/tests/common/mod.rs`.

## Puzzle Classifier Tests

The chess-puzzler crate is a port of Lichess's puzzle tagger. Tests live in `backend-rust/tests/classify_test.rs`:

- **21 individual theme tests** (10 puzzles each, hardcoded) — run with `cargo test --test classify_test`
- **Bulk validation** against 10K Lichess puzzles — run with `cargo test --test classify_test bulk_validate -- --ignored --nocapture`

The bulk test uses Stockfish at 100K nodes. CP-dependent tags (advantage/crushing/equality) are excluded from comparison because Lichess uses 40M nodes for their evals.

## Lichess Reference Code

The Lichess puzzler source is cloned at `C:\Users\steve\OneDrive\Desktop\lichess\lichess-puzzler`. Use it to cross-reference our Rust port against the original Python:

- `tagger/cook.py` — main tagger (our `chess-puzzler/src/puzzle/cook.rs`)
- `tagger/model.py` — data model (our `chess-puzzler/src/puzzle/mod.rs`)
- `tagger/util.py` — utilities (our `chess-puzzler/src/board_utils.rs`)
- `tagger/zugzwang.py` — zugzwang detection (our `chess-puzzler/src/tactics/zugzwang.rs`)

## Stale Process Cleanup

Before running `cargo test`, `cargo build`, or `cargo run`, check for and kill stale processes:
```
tasklist | findstr -i "stockfish cargo"
taskkill //PID <pid> //F
```
