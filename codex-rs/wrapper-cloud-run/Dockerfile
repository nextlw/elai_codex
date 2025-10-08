# Multi-stage Docker build for Codex Cloud Wrapper
# Optimized for Google Cloud Run, GKE, and App Engine
# Following Docker best practices for Rust applications

# ============================================
# Stage 1: Build the wrapper-cloud-run binary
# ============================================
FROM rust:1.90-slim AS builder

# Install minimal build dependencies for compiling Rust with SSL support
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory for the build
WORKDIR /build

# Copy only the files needed for this specific crate (not the entire workspace)
# This allows the wrapper to be built independently
COPY wrapper-cloud-run/Cargo.toml wrapper-cloud-run/Cargo.lock ./
COPY wrapper-cloud-run/src ./src

# Build the wrapper binary in release mode with optimizations
# The binary name matches the package name in Cargo.toml
RUN cargo build --release --bin codex-wrapper-cloud-run

# ============================================
# Stage 2: Minimal runtime image
# ============================================
# Use debian:trixie-slim to match GLIBC version from rust:1.90-slim
FROM debian:trixie-slim

# Install runtime dependencies including Google Cloud SDK and Python 3
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

# Create non-root user for security best practices
RUN useradd -m -u 1000 -s /bin/bash appuser

# Set working directory
WORKDIR /app

# Copy the compiled wrapper binary from builder stage
COPY --from=builder /build/target/release/codex-wrapper-cloud-run /app/codex-wrapper-cloud-run

# Copy the pre-compiled codex binary (musl version for static linking)
# The process.rs expects to find "codex" in the same directory as the wrapper
# Note: We'll copy this from the build context root and rename it
COPY wrapper-cloud-run/codex-x86_64-unknown-linux-musl /app/codex

# Copy config.toml for MCP and model configuration
COPY wrapper-cloud-run/config.toml /app/config.toml

# Copy MCP Pipedrive server
COPY wrapper-cloud-run/mcp-servers/pipedrive /app/mcp-servers/pipedrive

# Install Python dependencies for MCP Pipedrive using venv + pip
RUN cd /app/mcp-servers/pipedrive && \
    python3 -m venv .venv && \
    . .venv/bin/activate && \
    pip install --no-cache-dir -e .

# Add venv to PATH for MCP server
ENV PATH="/app/mcp-servers/pipedrive/.venv/bin:$PATH"

# Ensure both binaries are executable
RUN chmod +x /app/codex-wrapper-cloud-run /app/codex

# Change ownership to non-root user
RUN chown -R appuser:appuser /app

# Switch to non-root user for runtime
USER appuser

# Expose port 8080 (Cloud Run requirement)
EXPOSE 8080

# Set environment variables
# ENV PORT=8080
ENV RUST_LOG=info

# Health check for container orchestrators
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the wrapper server
ENTRYPOINT ["/app/codex-wrapper-cloud-run"]