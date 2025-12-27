# Dockerfile for Polymarket HFT Bot
# Multi-stage build for minimal production image

# Stage 1: Builder
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY benches ./benches
COPY examples ./examples
COPY tests ./tests

# Build release binary with optimizations
RUN cargo build --release --bin polymarket_hft_bot

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash hftbot

# Copy binary from builder
COPY --from=builder /app/target/release/polymarket_hft_bot /usr/local/bin/

# Set ownership
RUN chown hftbot:hftbot /usr/local/bin/polymarket_hft_bot

# Switch to non-root user
USER hftbot
WORKDIR /home/hftbot

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep polymarket_hft_bot || exit 1

# Environment variables (override at runtime)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the bot
CMD ["polymarket_hft_bot"]
