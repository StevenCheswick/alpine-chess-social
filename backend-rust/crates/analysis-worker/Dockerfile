# Multi-stage build for analysis-worker
# Target: ARM64 (AWS Graviton) for cost efficiency

# ============================================
# Stage 1: Build Rust binary
# ============================================
# Use rust:bookworm to match runtime glibc version
FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy crates
COPY crates/chess-core ./crates/chess-core
COPY crates/chess-puzzler ./crates/chess-puzzler
COPY crates/analysis-worker ./crates/analysis-worker

# Create minimal workspace Cargo.toml for Docker build
RUN echo '[workspace]\n\
resolver = "2"\n\
members = [\n\
    "crates/chess-core",\n\
    "crates/chess-puzzler",\n\
    "crates/analysis-worker",\n\
]\n\
\n\
[workspace.dependencies]\n\
tokio = { version = "1", features = ["full"] }\n\
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "chrono", "json"] }\n\
serde = { version = "1", features = ["derive"] }\n\
serde_json = "1"\n\
tracing = "0.1"\n\
tracing-subscriber = { version = "0.3", features = ["env-filter"] }\n\
thiserror = "2"\n\
anyhow = "1"\n\
dotenvy = "0.15"\n\
shakmaty = "0.27"\n\
regex = "1"\n\
aws-config = "1"\n\
aws-sdk-sqs = "1"\n\
aws-sdk-secretsmanager = "1"\n\
num_cpus = "1.16"\n\
bincode = "1"\n' > Cargo.toml

# Build release binary
RUN cargo build --release -p analysis-worker

# ============================================
# Stage 2: Download Stockfish
# ============================================
FROM debian:bookworm-slim AS stockfish

RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*

WORKDIR /stockfish

# Download Stockfish 18 for the target architecture
# Linux builds use .tar format; ARM64 uses android-armv8 (works on Linux ARM64)
ARG TARGETARCH
RUN set -ex && \
    mkdir -p /tmp/sf && cd /tmp/sf && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -fsSL -o stockfish.tar "https://github.com/official-stockfish/Stockfish/releases/download/sf_18/stockfish-android-armv8.tar"; \
    else \
        curl -fsSL -o stockfish.tar "https://github.com/official-stockfish/Stockfish/releases/download/sf_18/stockfish-ubuntu-x86-64-avx2.tar"; \
    fi && \
    tar -xf stockfish.tar && \
    find . -name "stockfish-*" -type f -executable | head -1 | xargs -I{} cp {} /stockfish/stockfish && \
    chmod +x /stockfish/stockfish && \
    rm -rf /tmp/sf && \
    ls -la /stockfish/

# ============================================
# Stage 3: Runtime image
# ============================================
# Use Debian Bookworm (same as rust:bookworm) to match glibc version
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (SSL for AWS SDK)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/analysis-worker /app/analysis-worker

# Copy opening book data
COPY data/opening_book.bin /app/data/opening_book.bin

# Copy Stockfish from stockfish stage
COPY --from=stockfish /stockfish/stockfish /usr/local/bin/stockfish
RUN chmod +x /usr/local/bin/stockfish

# Default environment variables
ENV STOCKFISH_PATH=/usr/local/bin/stockfish
ENV NODES_PER_POSITION=100000
ENV MAX_EMPTY_RECEIVES=5
ENV VISIBILITY_TIMEOUT_SECS=300
ENV RUST_LOG=info

# Run as non-root user
RUN useradd -m -u 1000 worker
USER worker

ENTRYPOINT ["/app/analysis-worker"]
