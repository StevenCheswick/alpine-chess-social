# Alpine Chess AWS Infrastructure Guide

This document describes the AWS infrastructure and how to work with each component.

## Architecture Overview

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│   Amplify   │────▶│   App Runner    │────▶│  RDS Postgres│
│  (Frontend) │     │   (Backend API) │     │  (Database)  │
└─────────────┘     └────────┬────────┘     └─────────────┘
                             │
                             ▼
                    ┌─────────────────┐     ┌─────────────┐
                    │    SQS Queue    │────▶│  AWS Batch  │
                    │ (Analysis Jobs) │     │  (Worker)   │
                    └─────────────────┘     └─────────────┘
```

## AWS Account & Region

- **Account ID**: 019304715762
- **Region**: us-east-1
- **IAM**: GitHub Actions uses OIDC role `github-actions-deploy`

---

## 1. Frontend (AWS Amplify)

### Overview
- **Service**: AWS Amplify Hosting
- **Source**: GitHub `main` branch, `frontend/` directory
- **Build**: Vite (React + TypeScript)
- **URL**: https://main.d1234567890.amplifyapp.com (check Amplify console for actual URL)

### How It Works
- Amplify auto-deploys on push to `main` when files in `frontend/` change
- Build settings are in `frontend/amplify.yml`

### Local Development
```bash
cd frontend
npm install
npm run dev  # http://localhost:5173
```

### Environment Variables (Amplify Console)
- `VITE_API_URL` - Backend API URL (App Runner)

### Deploying
Automatic on push to `main`. Manual deploy:
1. Go to AWS Amplify Console
2. Select the app
3. Click "Redeploy this version"

---

## 2. Backend API (AWS App Runner)

### Overview
- **Service**: App Runner `alpine-chess-api`
- **Source**: ECR image `alpine-chess-server:latest`
- **Port**: 8000
- **URL**: https://xxxxxxxx.us-east-1.awsapprunner.com

### Docker Image
Location: `backend-rust/crates/server/Dockerfile`

```bash
# Build locally (from backend-rust/)
docker build -t alpine-chess-server -f crates/server/Dockerfile .
```

### Environment Variables (App Runner Console)
| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_SECRET` | Secret for JWT token signing |
| `SQS_QUEUE_URL` | Analysis job queue URL |
| `RUST_LOG` | Log level (info, debug, etc.) |

### IAM Roles
- **Task Execution Role**: `alpine-chess-task-execution-role` - pulls images from ECR
- **Instance Role**: `alpine-chess-apprunner-instance` - allows SQS and Batch API calls

### Deploying

**Local deploy**:
```bash
cd backend-rust
./deploy.sh
```

This script:
1. Logs into ECR
2. Builds Docker image locally (cached layers = fast rebuilds)
3. Pushes to ECR
4. Triggers App Runner deployment

### Logs
```bash
# View recent logs
aws logs tail /aws/apprunner/alpine-chess-api/3b238f1208be4ad29b5bbd0d6aca957e/application --follow
```

---

## 3. Analysis Worker (AWS Batch)

### Overview
- **Service**: AWS Batch
- **Job Queue**: `alpine-chess-analysis-queue`
- **Job Definition**: `alpine-chess-analysis-worker`
- **Compute**: c7i.xlarge (4 vCPUs) - scales 0-16 vCPUs
- **Source**: ECR image `alpine-chess-analysis-worker:latest`

### How It Works
1. User clicks "Server Analyze" in frontend
2. Backend API sends game IDs to SQS queue
3. Backend checks for active Batch jobs — if none, submits a new one
4. Worker pulls messages from SQS, runs Stockfish analysis
5. Results saved to database

### Docker Image
Location: `backend-rust/crates/analysis-worker/Dockerfile`

Includes:
- Rust binary (analysis-worker)
- Stockfish 18 (downloaded during build)

```bash
# Build locally (from backend-rust/)
docker build -t alpine-chess-analysis-worker -f crates/analysis-worker/Dockerfile .
```

### Environment Variables (Job Definition)
| Variable | Description |
|----------|-------------|
| `SQS_QUEUE_URL` | SQS queue to poll for jobs |
| `DB_SECRET_ARN` | Secrets Manager ARN for DATABASE_URL |
| `STOCKFISH_PATH` | Path to Stockfish binary (default: /usr/local/bin/stockfish) |
| `NODES_PER_POSITION` | Stockfish search depth (default: 100000) |
| `MAX_EMPTY_RECEIVES` | Empty polls before exit (default: 5) |
| `RUST_LOG` | Log level |

### IAM Roles
- **Task Execution Role**: `alpine-chess-task-execution-role` - ECR access
- **Task Role**: `alpine-chess-worker-task-role` - SQS + Secrets Manager access

### Deploying

**Local deploy**:
```bash
cd backend-rust
./deploy-worker.sh
```

### Running a Job

Jobs are triggered **automatically** by the backend API when games are queued.
The backend calls `ensure_worker_running()` which checks for active Batch jobs
and submits a new one if none are running.

```bash
# Check job status
aws batch list-jobs --job-queue alpine-chess-analysis-queue --job-status RUNNING

# View logs
MSYS_NO_PATHCONV=1 aws logs tail /aws/batch/alpine-chess-analysis-worker --follow

# Manual job submission (if needed)
aws batch submit-job \
  --job-name "analysis-worker-$(date +%s)" \
  --job-queue alpine-chess-analysis-queue \
  --job-definition alpine-chess-analysis-worker
```

### Logs
```bash
# List recent log streams
aws logs describe-log-streams \
  --log-group-name /aws/batch/alpine-chess-analysis-worker \
  --order-by LastEventTime --descending --max-items 5

# Get logs from a stream
aws logs get-log-events \
  --log-group-name /aws/batch/alpine-chess-analysis-worker \
  --log-stream-name worker/default/INSTANCE_ID \
  --limit 100
```

---

## 4. Database (RDS PostgreSQL)

### Overview
- **Service**: RDS PostgreSQL
- **Instance**: `chess-analysis-db`
- **Endpoint**: chess-analysis-db.cohk6egmirrt.us-east-1.rds.amazonaws.com
- **Port**: 5432
- **Database**: alpine_chess

### Connecting
```bash
# Via psql (need PostgreSQL client installed)
psql postgresql://chess_admin:PASSWORD@chess-analysis-db.cohk6egmirrt.us-east-1.rds.amazonaws.com:5432/alpine_chess?sslmode=require
```

### Credentials
- Stored in AWS Secrets Manager: `alpine-chess/database-url`
- Format: `postgres://chess_admin:PASSWORD@host:5432/alpine_chess`

### Schema Migrations
Currently manual. Run SQL files against the database:
```bash
psql $DATABASE_URL -f migrations/001_create_tables.sql
```

---

## 5. SQS Queue (Analysis Jobs)

### Overview
- **Queue Name**: `alpine-chess-analysis-jobs`
- **URL**: https://sqs.us-east-1.amazonaws.com/019304715762/alpine-chess-analysis-jobs
- **Type**: Standard queue
- **Visibility Timeout**: 300 seconds (5 minutes)

### Message Format
Plain text game ID:
```
12345
```

### Monitoring
```bash
# Check queue depth
aws sqs get-queue-attributes \
  --queue-url https://sqs.us-east-1.amazonaws.com/019304715762/alpine-chess-analysis-jobs \
  --attribute-names ApproximateNumberOfMessages ApproximateNumberOfMessagesNotVisible

# Purge queue (delete all messages)
aws sqs purge-queue \
  --queue-url https://sqs.us-east-1.amazonaws.com/019304715762/alpine-chess-analysis-jobs
```

---

## 6. Secrets Manager

### Secrets
| Secret Name | Contents |
|-------------|----------|
| `alpine-chess/database-url` | PostgreSQL connection string |

### Accessing
```bash
# Get secret value
aws secretsmanager get-secret-value --secret-id alpine-chess/database-url --query SecretString --output text

# Update secret
aws secretsmanager put-secret-value --secret-id alpine-chess/database-url --secret-string 'postgres://...'
```

---

## 7. ECR Repositories

| Repository | Description |
|------------|-------------|
| `alpine-chess-server` | Backend API image |
| `alpine-chess-analysis-worker` | Batch worker image |

### Listing Images
```bash
aws ecr describe-images --repository-name alpine-chess-server --query 'imageDetails[*].{tags:imageTags,pushed:imagePushedAt}' --output table
```

---

## Local Development Setup

### Prerequisites
- Docker Desktop
- Rust toolchain (`rustup`)
- Node.js 18+
- PostgreSQL client (optional, for DB access)
- AWS CLI configured with credentials

### Starting Local Stack
```bash
# 1. Start Postgres (Docker)
docker start chess-postgres
# Or create: docker run --name chess-postgres -e POSTGRES_PASSWORD=password -p 5432:5432 -d postgres

# 2. Start Backend
cd backend-rust
cargo run -p server

# 3. Start Frontend
cd frontend
npm run dev
```

### Environment Files

**backend-rust/.env** (create locally, don't commit):
```
DATABASE_URL=postgresql://postgres:password@localhost:5432/alpine_chess
JWT_SECRET=local-dev-secret
SQS_QUEUE_URL=http://localhost:4566/000000000000/analysis-jobs
RUST_LOG=debug
```

**frontend/.env.local**:
```
VITE_API_URL=http://localhost:8000
```

---

## Troubleshooting

### App Runner Deployment Failed
1. Check App Runner logs for startup errors
2. Verify DATABASE_URL is correct
3. Check IAM roles have required permissions

### Batch Job Stuck in RUNNABLE
1. Check compute environment has capacity
2. Verify instance types can fulfill vCPU/memory requirements
3. Check for spot capacity issues

### glibc Version Mismatch
Error: `GLIBC_X.XX not found`
- Builder and runtime Debian versions must match
- Use `rust:bookworm` + `debian:bookworm-slim`

### SQS Access Denied
- Verify App Runner instance role has SQS permissions
- Check SQS queue policy allows the role

---

## Cost Optimization

- **App Runner**: Scales to zero when idle (pay per request)
- **Batch**: Uses spot instances (c7i.xlarge ~$0.05/hr spot)
- **RDS**: Consider reserved instances for production
- **Amplify**: Free tier covers most usage

---

## Security Notes

- Never commit credentials or .env files
- Use Secrets Manager for sensitive values
- IAM roles use least-privilege principle
- RDS is not publicly accessible (VPC only, accessed via App Runner/Batch)
