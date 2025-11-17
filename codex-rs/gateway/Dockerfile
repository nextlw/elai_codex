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

# Strip debug symbols to reduce binary size
RUN strip target/release/codex-gateway

# Stage 2: Runtime environment
FROM debian:trixie-slim

# Install runtime dependencies including Google Cloud SDK and Python 3
# (Python support for MCP servers integration)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    apt-transport-https \
    gnupg \
    lsb-release \
    python3 \
    python3-pip \
    python3-venv \
    && echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list \
    && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg \
    && apt-get update && apt-get install -y --no-install-recommends google-cloud-cli \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 gateway

# Create Codex home directory with proper permissions
RUN mkdir -p /home/gateway/.codex && \
    chown -R gateway:gateway /home/gateway/.codex

# Copy config.toml from repository (for Cloud Run - no volumes available)
COPY --from=builder /build/.github/codex/home/config.toml /home/gateway/.codex/config.toml

# Copy binary from builder
COPY --from=builder /build/codex-rs/target/release/codex-gateway /usr/local/bin/codex-gateway

# Set ownership for binary and config
RUN chown gateway:gateway /usr/local/bin/codex-gateway && \
    chown -R gateway:gateway /home/gateway/.codex

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
