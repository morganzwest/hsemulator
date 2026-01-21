# ---- Build stage ----
FROM rust:1.83 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app

# Needed for TLS + certs
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/hsemulate /usr/local/bin/hsemulate

EXPOSE 8080

# Cloud Run expects the server to bind to $PORT
ENV PORT=8080

CMD ["hsemulate", "runtime", "--listen", "0.0.0.0:8080"]
