# Multi-stage build for server
FROM rust:1.84-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo.toml
COPY Cargo.toml ./

# Copy only the crates needed for server
COPY crates/chess-core ./crates/chess-core
COPY crates/chess-puzzler ./crates/chess-puzzler
COPY crates/server ./crates/server

# Create a clean workspace Cargo.toml (remove package, dev-deps, and analysis-worker)
RUN echo '[workspace]' > Cargo.toml.new && \
    echo 'resolver = "2"' >> Cargo.toml.new && \
    echo 'members = ["crates/server", "crates/chess-core", "crates/chess-puzzler"]' >> Cargo.toml.new && \
    echo '' >> Cargo.toml.new && \
    grep -A1000 '^\[workspace.dependencies\]' Cargo.toml >> Cargo.toml.new && \
    mv Cargo.toml.new Cargo.toml

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

ENV HOST=0.0.0.0
ENV PORT=8000
ENV RUST_LOG=info

EXPOSE 8000

ENTRYPOINT ["/app/server"]
