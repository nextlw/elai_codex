# Multi-stage build for Codex Gateway
# Stage 1: Build environment
FROM rust:1.90-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy entire workspace (needed for dependencies)
COPY . .

# Build the gateway in release mode
WORKDIR /build/codex-rs
RUN cargo build --release --package codex-gateway

# Stage 2: Runtime environment
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 gateway

# Create Codex home directory with proper permissions
RUN mkdir -p /home/gateway/.codex && \
    chown -R gateway:gateway /home/gateway/.codex

# Copy binary from builder
COPY --from=builder /build/codex-rs/target/release/codex-gateway /usr/local/bin/codex-gateway

# Set ownership
RUN chown gateway:gateway /usr/local/bin/codex-gateway

# Switch to non-root user
USER gateway

# Set CODEX_HOME environment variable
ENV CODEX_HOME=/home/gateway/.codex

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV PORT=8080

# Run the gateway
CMD ["codex-gateway"]
