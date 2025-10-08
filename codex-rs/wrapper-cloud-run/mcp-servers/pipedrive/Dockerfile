FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim

WORKDIR /app

# Copy the entire application first
COPY . .

# Create a virtual environment and install dependencies
RUN uv venv .venv && \
    . .venv/bin/activate && \
    uv pip install -e .

# Set environment variables
ENV PATH="/app/.venv/bin:$PATH"
ENV CONTAINER_MODE="true"

# Cloud Run injeta a variável PORT automaticamente (padrão 8080)
# O server.py já lê PORT do ambiente
EXPOSE 8080

# Command to run the MCP server
CMD ["uv", "run", "server.py"]