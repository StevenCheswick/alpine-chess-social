# Multi-stage build for server
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./

# Copy only the crates needed for server
COPY crates/chess-core ./crates/chess-core
COPY crates/chess-puzzler ./crates/chess-puzzler
COPY crates/server ./crates/server

# Remove analysis-worker from workspace members (it has extra deps we don't need)
RUN sed -i 's/"crates\/analysis-worker",//' Cargo.toml

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
