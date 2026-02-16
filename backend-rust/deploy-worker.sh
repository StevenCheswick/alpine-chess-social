#!/bin/bash
# Quick deploy script for analysis worker

set -e

ECR_REPO="019304715762.dkr.ecr.us-east-1.amazonaws.com/alpine-chess-analysis-worker"

echo "==> Logging into ECR..."
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 019304715762.dkr.ecr.us-east-1.amazonaws.com

echo "==> Building Docker image..."
docker build -t $ECR_REPO:latest -f crates/analysis-worker/Dockerfile .

echo "==> Pushing to ECR..."
docker push $ECR_REPO:latest

echo "==> Done! New worker image pushed."
echo "    Next Batch job will use the new image automatically."
echo "    To run a job now: aws batch submit-job --job-name test-\$(date +%s) --job-queue alpine-chess-analysis-queue --job-definition alpine-chess-analysis-worker"
