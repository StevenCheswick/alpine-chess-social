# AWS Infrastructure TODO

## Critical Issues (Must Fix Before Deploy)

### 1. Missing Spot Fleet Role (Verify if Needed)
Batch Spot may require a `spot_iam_fleet_role`. However, if Batch is working in prod, AWS may be using the service-linked role (`AWSServiceRoleForBatch`) instead. Verify before adding.

**File:** `terraform/iam.tf`

```hcl
resource "aws_iam_role" "spot_fleet" {
  name = "${var.project_name}-spot-fleet-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = { Service = "spotfleet.amazonaws.com" }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "spot_fleet" {
  role       = aws_iam_role.spot_fleet.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEC2SpotFleetTaggingRole"
}
```

**File:** `terraform/batch.tf` - add to `compute_resources` block:
```hcl
spot_iam_fleet_role = aws_iam_role.spot_fleet.arn
```

---

### 2. Database Secret in Terraform State
`secret_string = var.database_url` stores the DB password in plaintext in the Terraform state file.

**Options:**
- **A) Remove secret version resource**, set manually after apply:
  ```bash
  aws secretsmanager put-secret-value \
    --secret-id alpine-chess/database-url \
    --secret-string "postgresql://user:pass@host:5432/db"
  ```
- **B) Add lifecycle ignore** (keeps secret out of future diffs):
  ```hcl
  resource "aws_secretsmanager_secret_version" "database" {
    # ...
    lifecycle {
      ignore_changes = [secret_string]
    }
  }
  ```

---

### 3. No x86 Fallback Instance Types
If Graviton (ARM) Spot capacity is exhausted, jobs will be stuck waiting.

**File:** `terraform/variables.tf`

```hcl
variable "batch_instance_types" {
  description = "EC2 instance types for Batch (ARM + x86 fallback)"
  type        = list(string)
  default     = ["c7g.xlarge", "c6g.xlarge", "m6g.xlarge", "c6a.xlarge", "c5.xlarge"]
}
```

Note: If using x86 fallback, Docker image must be multi-arch or have separate x86 build.

---

## Missing Infrastructure

### 4. RDS/Aurora for PostgreSQL
No database resource defined. Options:

| Option | Cost | Notes |
|--------|------|-------|
| RDS Postgres (db.t4g.micro) | ~$12/mo | Cheapest, single-AZ |
| Aurora Serverless v2 | ~$0 idle, scales | Best for bursty workloads |
| External (Railway, Supabase) | Varies | Already using? |

**File to create:** `terraform/rds.tf`

---

### 5. Backend Server (ECS/EC2)
No infrastructure for the Axum backend server.

**Options:**
- ECS Fargate (serverless, ~$15/mo for small workload)
- EC2 t4g.small (~$12/mo)
- App Runner (simpler, slightly more expensive)

**File to create:** `terraform/backend.tf`

---

### 6. ALB + HTTPS
No load balancer or SSL termination.

**Required for:**
- HTTPS (ACM certificate)
- Health checks
- Future horizontal scaling

**File to create:** `terraform/alb.tf`

---

### 7. S3 for Static Assets
Needed for:
- Opening book JSON backup (`autobook_100.json`, `opening_tree.json`)
- Frontend static files (if using S3 + CloudFront)
- Future: user uploads

**File to create:** `terraform/s3.tf`

```hcl
resource "aws_s3_bucket" "assets" {
  bucket = "${var.project_name}-assets"
}
```

---

### 8. CloudFront for Frontend
Static site hosting for React app.

**File to create:** `terraform/cloudfront.tf`

---

## Improvements (Nice to Have)

### 9. On-Demand Fallback Compute Environment
If Spot instances are repeatedly reclaimed, add on-demand fallback:

```hcl
resource "aws_batch_compute_environment" "analysis_ondemand" {
  compute_environment_name = "${var.project_name}-analysis-ondemand"
  type                     = "MANAGED"

  compute_resources {
    type = "EC2"  # On-demand, not SPOT
    # ...
  }
}

# Add to job queue with lower priority
compute_environment_order {
  order               = 2
  compute_environment = aws_batch_compute_environment.analysis_ondemand.arn
}
```

---

### 10. ECR Immutable Tags
Prevent accidental overwrites of production images:

```hcl
image_tag_mutability = "IMMUTABLE"
```

Then use versioned tags (`v1.2.3`) instead of `:latest`.

---

### 11. Private Subnets + NAT Gateway
Current setup uses public subnets. For better security:
- Move Batch instances to private subnets
- Add NAT Gateway for outbound traffic
- Cost: ~$32/mo per NAT Gateway

---

### 12. Remote State Backend
Uncomment in `main.tf` for team collaboration:

```hcl
backend "s3" {
  bucket         = "alpine-chess-terraform-state"
  key            = "analysis-worker/terraform.tfstate"
  region         = "us-east-1"
  dynamodb_table = "terraform-locks"  # For state locking
  encrypt        = true
}
```

---

## Checklist

- [ ] Add Spot Fleet role (critical - verify if needed)
- [ ] Fix database secret handling (critical)
- [ ] Add x86 fallback instances (critical)
- [ ] Add RDS/Aurora
- [ ] Add backend server (ECS)
- [ ] Add ALB + HTTPS
- [ ] Add S3 bucket for assets
- [ ] Add CloudFront for frontend
- [ ] Add on-demand fallback compute env
- [ ] Switch to immutable ECR tags
- [ ] Consider private subnets + NAT
- [ ] Enable remote state backend
