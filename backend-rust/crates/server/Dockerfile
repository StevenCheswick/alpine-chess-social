# Multi-stage build for server
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo.toml first (for dependency caching)
COPY Cargo.toml Cargo.lock* ./

# Copy crates
COPY crates/chess-core ./crates/chess-core
COPY crates/chess-puzzler ./crates/chess-puzzler
COPY crates/server ./crates/server

# Create stub for analysis-worker (referenced in workspace but not needed for server build)
RUN mkdir -p crates/analysis-worker/src && \
    echo '[package]\nname = "analysis-worker"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]' > crates/analysis-worker/Cargo.toml && \
    echo 'fn main() {}' > crates/analysis-worker/src/main.rs

# Build release binary
RUN cargo build --release -p server

# Runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server

# Environment
ENV HOST=0.0.0.0
ENV PORT=8000
ENV RUST_LOG=info

EXPOSE 8000

ENTRYPOINT ["/app/server"]
