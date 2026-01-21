# ---- Build stage ----
FROM rust:1.83 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app

# Install system deps + runtimes
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    python3 \
    python3-pip \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Normalise runtime binary names
# (your code expects `python` and `node`)
RUN ln -s /usr/bin/python3 /usr/bin/python || true

# Copy binary
COPY --from=builder /app/target/release/hsemulate /usr/local/bin/hsemulate

# Cloud Run
EXPOSE 8080
ENV PORT=8080

# Optional: sanity check at container start (safe to keep)
RUN node --version && python --version

CMD ["hsemulate", "runtime", "--listen", "0.0.0.0:8080"]
