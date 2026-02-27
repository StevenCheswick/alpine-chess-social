# Multi-stage build with cargo-chef for faster rebuilds
# Dependencies are cached separately — only source code recompiles on changes

FROM rust:bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# Stage 1: Generate dependency recipe
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/chess-core ./crates/chess-core
COPY crates/server ./crates/server

# Create clean workspace Cargo.toml
RUN echo '[workspace]' > Cargo.toml.new && \
    echo 'resolver = "2"' >> Cargo.toml.new && \
    echo 'members = ["crates/server", "crates/chess-core"]' >> Cargo.toml.new && \
    echo '' >> Cargo.toml.new && \
    grep -A1000 '^\[workspace.dependencies\]' Cargo.toml >> Cargo.toml.new && \
    mv Cargo.toml.new Cargo.toml

RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (CACHED unless Cargo.toml/lock changes)
FROM chef AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/Cargo.toml Cargo.toml
COPY --from=planner /app/Cargo.lock Cargo.lock
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build source (only this layer rebuilds on code changes)
COPY crates/chess-core ./crates/chess-core
COPY crates/server ./crates/server

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
