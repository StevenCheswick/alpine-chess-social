#!/bin/bash
# Quick deploy script for backend server

set -e

ECR_REPO="019304715762.dkr.ecr.us-east-1.amazonaws.com/alpine-chess-server"
APP_RUNNER_ARN="arn:aws:apprunner:us-east-1:019304715762:service/alpine-chess-api/3b238f1208be4ad29b5bbd0d6aca957e"

# Ensure code is committed and pushed before deploying
echo "==> Checking git status..."
cd "$(git rev-parse --show-toplevel)"
if [ -n "$(git status --porcelain -uno)" ]; then
    echo "ERROR: Uncommitted changes detected. Commit and push before deploying."
    git status --short -uno
    exit 1
fi
if [ -n "$(git log @{u}..HEAD 2>/dev/null)" ]; then
    echo "ERROR: Unpushed commits. Run 'git push' before deploying."
    git log --oneline @{u}..HEAD
    exit 1
fi
echo "    Git is clean and up to date with remote."
cd - > /dev/null

echo "==> Logging into ECR..."
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 019304715762.dkr.ecr.us-east-1.amazonaws.com

echo "==> Building Docker image..."
docker build -t $ECR_REPO:latest -f crates/server/Dockerfile .

echo "==> Pushing to ECR..."
docker push $ECR_REPO:latest

echo "==> Triggering App Runner deployment..."
if aws apprunner start-deployment --service-arn $APP_RUNNER_ARN --region us-east-1 2>/dev/null; then
    echo "==> Deployment triggered!"
else
    echo "==> App Runner already deploying (auto-triggered from ECR push)"
fi

echo "==> Done! Image pushed to ECR."
echo "    Check status: aws apprunner list-operations --service-arn $APP_RUNNER_ARN --region us-east-1 --max-results 1"
