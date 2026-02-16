#!/bin/bash
# Quick deploy script for backend server

set -e

ECR_REPO="019304715762.dkr.ecr.us-east-1.amazonaws.com/alpine-chess-server"
APP_RUNNER_ARN="arn:aws:apprunner:us-east-1:019304715762:service/alpine-chess-api/3b238f1208be4ad29b5bbd0d6aca957e"

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
