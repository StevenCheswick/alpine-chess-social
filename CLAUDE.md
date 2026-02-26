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
3. **Frontend** (static site, port 5174):
   ```
   cd frontend && python -m http.server 5174
   ```

Start them in this order — backend needs Postgres, frontend needs backend.

## Integration Tests

Integration tests run against the live server (not mocked). The full stack must be up before running them:

```
cd backend-rust && cargo test --test auth_test
```

Test files live in `backend-rust/tests/` with shared helpers in `backend-rust/tests/common/mod.rs`.

## Lichess Reference Code

The Lichess puzzler source is cloned at `C:\Users\steve\OneDrive\Desktop\lichess\lichess-puzzler`. Use it to cross-reference our Rust port (inside `analysis-worker`) against the original Python:

- `tagger/cook.py` — main tagger (our `analysis-worker/src/puzzle/cook.rs`)
- `tagger/model.py` — data model (our `analysis-worker/src/puzzle/mod.rs`)
- `tagger/util.py` — utilities (our `analysis-worker/src/board_utils.rs`)
- `tagger/zugzwang.py` — zugzwang detection (our `analysis-worker/src/tactics/zugzwang.rs`)

## Stale Process Cleanup

Before running `cargo test`, `cargo build`, or `cargo run`, check for and kill stale processes:
```
tasklist | findstr -i "stockfish cargo"
taskkill //PID <pid> //F
```

## Docker Builds

**NEVER use `--no-cache` on Docker builds.** COPY layers already detect source file changes and invalidate automatically. Using `--no-cache` forces a full rebuild of the Rust toolchain and all dependencies from scratch (~15+ minutes). If a COPY layer isn't invalidating, the fix is to check `.dockerignore` or the build context — not to nuke the entire cache. If you genuinely believe `--no-cache` is needed, ask the user first before running it.

## Production Deployment

After pushing changes to `main`:

1. **Frontend** (AWS Amplify): Auto-deploys on push to `main` — no manual step needed.
2. **Backend** (AWS App Runner): Run the deploy script to build, push to ECR, and trigger deployment:
   ```
   cd backend-rust && bash deploy.sh
   ```
3. **Analysis Worker** (AWS Batch/Fargate): If worker code changed, deploy separately:
   ```
   cd backend-rust && bash deploy-worker.sh
   ```

Check backend deploy status:
```
aws apprunner list-operations --service-arn arn:aws:apprunner:us-east-1:019304715762:service/alpine-chess-api/3b238f1208be4ad29b5bbd0d6aca957e --region us-east-1 --max-results 1
```
