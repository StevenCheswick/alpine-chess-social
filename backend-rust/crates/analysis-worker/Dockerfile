# Multi-stage build for analysis-worker

# ============================================
# Stage 1: Build Rust binary
# ============================================
FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# --- Dependency caching layer ---
# Copy only Cargo manifests first so dependency compilation is cached
# and doesn't re-run when only source code changes.
COPY crates/chess-core/Cargo.toml ./crates/chess-core/Cargo.toml
COPY crates/analysis-worker/Cargo.toml ./crates/analysis-worker/Cargo.toml

# Create minimal workspace Cargo.toml
RUN echo '[workspace]\n\
resolver = "2"\n\
members = [\n\
    "crates/chess-core",\n\
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
shakmaty = "0.29"\n\
regex = "1"\n\
aws-config = "1"\n\
aws-sdk-sqs = "1"\n\
aws-sdk-secretsmanager = "1"\n\
num_cpus = "1.16"\n\
bincode = "1"\n' > Cargo.toml

# Create dummy source files to compile dependencies only
RUN mkdir -p crates/chess-core/src && echo "" > crates/chess-core/src/lib.rs && \
    mkdir -p crates/analysis-worker/src/bin && echo "" > crates/analysis-worker/src/lib.rs && echo "fn main(){}" > crates/analysis-worker/src/main.rs && echo "fn main(){}" > crates/analysis-worker/src/bin/classify_openings.rs

# Build dependencies (this layer is cached until Cargo.toml files change)
RUN cargo build --release -p analysis-worker

# --- Source build layer ---
# Now copy real source code and invalidate cargo fingerprints so it recompiles
COPY crates/chess-core ./crates/chess-core
COPY crates/analysis-worker ./crates/analysis-worker
RUN rm -rf target/release/.fingerprint/analysis-worker-* target/release/.fingerprint/chess-core-*

# Rebuild — only the project crates recompile, deps are cached
RUN cargo build --release -p analysis-worker

# ============================================
# Stage 2: Download Stockfish
# ============================================
FROM debian:bookworm-slim AS stockfish

RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*

WORKDIR /stockfish

# Download Stockfish 18 for the target architecture
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
