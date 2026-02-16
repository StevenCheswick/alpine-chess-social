# Server-Side Analysis — AWS Plan

## Problem

Game analysis currently runs client-side via Stockfish WASM in the browser. This works and costs $0, but:

- User must keep the tab open for the entire duration
- Bulk analysis (500+ games) takes 30+ minutes on a fast machine
- WASM Stockfish is 3-5x slower than native
- Mobile users get terrible performance

## Architecture

```
User clicks "Analyze All"
        │
        ▼
  POST /api/games/analyze-server
        │
        ├──▶ Push game IDs to SQS (one message per game)
        │
        └──▶ batch.submit_job() (if not already running)
                    │
                    ▼
            AWS Batch spins up c7g.xlarge Spot
                    │
                    ▼
            Worker polls SQS, processes games
            Writes results to Postgres
                    │
                    ▼
            Queue empty → Worker exits → Instance terminates

Frontend polls GET /api/games/analysis-status
        → Postgres COUNT(*) for pending/completed
```

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Instance type** | c7g.xlarge | Best perf/$, 4 vCPU, ~24 games/min, ~$0.04/hr Spot |
| **SQS message format** | One message per game ID | Simple, Spot-safe (1 game lost on interrupt vs batch), $12/mo at 10M scale is negligible |
| **Job trigger** | Backend calls SubmitJob directly | Immediate start, no extra Lambda/CloudWatch infra |
| **Progress tracking** | Query Postgres | Zero new infra, COUNT(*) fast enough with index |
| **RDS auth** | Secrets Manager | Simple connection string, cheap (~$0.50/mo), rotation available later |
| **Instance fallback** | c7g.xlarge → c6g.xlarge → m6g.xlarge | Diversify for Spot availability |

### Why AWS Batch + Spot

- **Zero idle cost** — instances only exist while jobs are running
- **Spot pricing** — 60-90% off on-demand (~$0.04/hr for c7g.xlarge vs $0.14)
- **Elastic** — scales from 0 to many instances based on queue depth
- **Interruptible is fine** — if Spot reclaims an instance, only 1 in-flight game re-queues
- **Native Stockfish** — 3-5x faster than browser WASM

### Why Not Other Options

| Option | Problem |
|--------|---------|
| Lambda | Per-invocation overhead makes it ~40x more expensive ($0.004/game vs $0.0001/game) |
| EC2 On-Demand | Paying 24/7 for a bursty workload. $100/mo for a c7g.xlarge sitting idle most of the time |
| Fargate Spot | Viable alternative. Slightly more expensive per vCPU-hour but zero AMI management. Consider if ops burden matters more than cost |
| Batch Array Jobs | Cold-start per game (~2-5 sec overhead), less flexible than SQS for priority queues |

## Cost Estimates

Native Stockfish on Graviton 3 (c7g): ~3M nodes/sec single-thread.
At 100K nodes/eval, ~300 evals/game (including puzzle extraction + zugzwang tests):

- **~10 seconds per game per core**
- c7g.xlarge (4 vCPU): ~24 games/min

### Instance Comparison

| Instance | vCPU | RAM | Spot/hr | Games/min | $/1000 games |
|----------|------|-----|---------|-----------|--------------|
| c7g.medium | 1 | 2 GB | $0.01 | 6 | $0.028 |
| c7g.large | 2 | 4 GB | $0.02 | 12 | $0.028 |
| c7g.xlarge | 4 | 8 GB | $0.04 | 24 | $0.028 |
| c7g.2xlarge | 8 | 16 GB | $0.08 | 48 | $0.028 |

Cost per game is constant — larger instances just provide more throughput.

### Per-User Cost (Spot)

| Games | Time on c7g.xlarge | Spot cost |
|-------|-------------------|-----------|
| 500 | ~21 min | $0.014 |
| 2,000 | ~83 min | $0.056 |
| 5,000 | ~3.5 hrs | $0.14 |

### Monthly Cost at Scale (Spot)

| Active users | Games/mo | Compute | SQS | Total |
|-------------|----------|---------|-----|-------|
| 10 | 50K | ~$1.40 | ~$0.06 | ~$1.50 |
| 100 | 500K | ~$14 | ~$0.60 | ~$15 |
| 1,000 | 5M | ~$140 | ~$6 | ~$146 |
| 10,000 | 50M | ~$1,400 | ~$60 | ~$1,460 |

## Implementation Plan

### Phase 1: Worker Binary

New crate: `backend-rust/crates/analysis-worker/`

```rust
// main.rs pseudocode
async fn main() {
    let sqs = SqsClient::new();
    let pool = connect_to_rds_via_secrets_manager().await;

    loop {
        let messages = sqs.receive_messages("analysis-jobs", max: 10).await;
        if messages.is_empty() {
            // Queue drained, exit cleanly
            break;
        }

        for msg in messages {
            let game_id: i64 = msg.body.parse()?;
            process_game(&pool, game_id).await?;
            sqs.delete_message(msg.receipt_handle).await;
        }
    }
}
```

- Reuse analysis logic from `chess-puzzler` crate
- Spawn native Stockfish process (not WASM)
- Write results to `game_analysis` table

### Phase 2: Docker Image

```dockerfile
FROM amazonlinux:2023-arm64

# Install native Stockfish 17 (ARM)
RUN curl -L <stockfish-arm-release> -o /usr/local/bin/stockfish && chmod +x /usr/local/bin/stockfish

# Copy Rust binary (cross-compiled for aarch64)
COPY target/aarch64-unknown-linux-gnu/release/analysis-worker /usr/local/bin/

ENV STOCKFISH_PATH=/usr/local/bin/stockfish

ENTRYPOINT ["analysis-worker"]
```

### Phase 3: Infrastructure (Terraform)

```hcl
# SQS Queue
resource "aws_sqs_queue" "analysis_jobs" {
  name                       = "analysis-jobs"
  visibility_timeout_seconds = 300  # 5 min per game max
  message_retention_seconds  = 86400  # 1 day
}

# Secrets Manager for DB connection
resource "aws_secretsmanager_secret" "db_connection" {
  name = "analysis-worker/db-connection"
}

# ECR Repository
resource "aws_ecr_repository" "analysis_worker" {
  name = "analysis-worker"
}

# Batch Compute Environment
resource "aws_batch_compute_environment" "analysis" {
  compute_environment_name = "analysis-spot"
  type                     = "MANAGED"

  compute_resources {
    type                = "SPOT"
    allocation_strategy = "SPOT_CAPACITY_OPTIMIZED"

    instance_type = ["c7g.xlarge", "c6g.xlarge", "m6g.xlarge"]

    min_vcpus     = 0
    max_vcpus     = 64  # Up to 16 c7g.xlarge instances

    subnets            = var.private_subnet_ids
    security_group_ids = [aws_security_group.batch.id]
    instance_role      = aws_iam_instance_profile.batch.arn
  }
}

# Batch Job Definition
resource "aws_batch_job_definition" "analysis" {
  name = "analysis-worker"
  type = "container"

  container_properties = jsonencode({
    image  = "${aws_ecr_repository.analysis_worker.repository_url}:latest"
    vcpus  = 4
    memory = 7500  # Leave headroom from 8GB

    environment = [
      { name = "SQS_QUEUE_URL", value = aws_sqs_queue.analysis_jobs.url },
      { name = "DB_SECRET_ARN", value = aws_secretsmanager_secret.db_connection.arn },
    ]
  })
}
```

### Phase 4: Backend Endpoints

```rust
// POST /api/games/analyze-server
async fn analyze_server(
    user: AuthUser,
    pool: &PgPool,
    sqs: &SqsClient,
    batch: &BatchClient,
    body: AnalyzeRequest,
) -> Result<Json<AnalyzeResponse>> {
    // Get unanalyzed game IDs
    let game_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM user_games WHERE user_id = $1 AND analyzed_at IS NULL LIMIT 1000"
    ).bind(user.id).fetch_all(pool).await?;

    // Push to SQS
    for id in &game_ids {
        sqs.send_message(&queue_url, id.to_string()).await?;
    }

    // Start Batch job if not running
    let running = batch.list_jobs(status: "RUNNING").await?;
    if running.is_empty() {
        batch.submit_job("analysis-worker", "analysis-queue").await?;
    }

    Ok(Json(AnalyzeResponse { queued: game_ids.len() }))
}

// GET /api/games/analysis-status
async fn analysis_status(user: AuthUser, pool: &PgPool) -> Result<Json<StatusResponse>> {
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_games WHERE user_id = $1 AND analyzed_at IS NULL"
    ).bind(user.id).fetch_one(pool).await?;

    let completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_games WHERE user_id = $1 AND analyzed_at IS NOT NULL"
    ).bind(user.id).fetch_one(pool).await?;

    Ok(Json(StatusResponse { pending, completed }))
}
```

### Phase 5: Frontend Changes

```tsx
// "Analyze on Server" button
const analyzeOnServer = async () => {
  setServerAnalyzing(true);
  await fetch('/api/games/analyze-server', { method: 'POST' });
  pollStatus();
};

// Poll every 3 seconds
const pollStatus = () => {
  const interval = setInterval(async () => {
    const res = await fetch('/api/games/analysis-status');
    const { pending, completed } = await res.json();
    setProgress({ pending, completed });

    if (pending === 0) {
      clearInterval(interval);
      setServerAnalyzing(false);
      refreshGames();
    }
  }, 3000);
};
```

## Hybrid Approach

Keep both paths:

| Scenario | Method |
|----------|--------|
| Single game analysis (from game page) | Client-side WebSocket + WASM (instant, free) |
| Bulk "analyze all my games" | Server-side AWS Batch (background, fast) |

This gives instant feedback for single games while offloading expensive bulk work.

## Checkpointing & Fault Tolerance

- Each game is an independent SQS message
- If Spot interrupts mid-game: message returns to queue after visibility timeout (5 min)
- Only 1 game's work is lost per interruption
- Completed games are already in Postgres — no re-work
- Batch auto-restarts jobs if compute environment has capacity

## Future Considerations

- **Priority queues**: Paid users get separate SQS queue with dedicated Batch job queue
- **Rate limiting**: Cap free tier at N games/month server-side
- **Notifications**: SNS/WebSocket push when analysis completes instead of polling
