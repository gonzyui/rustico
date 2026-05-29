FROM rust:1.95-slim-bookworm AS builder

WORKDIR /usr/src/rustico

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src

RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false appuser

WORKDIR /app
COPY --from=builder /usr/src/rustico/target/release/rustico /app/rustico
COPY assets /app/assets
COPY config /app/config
RUN mkdir -p /app/data && chown -R appuser:appuser /app

USER appuser

CMD ["/app/rustico"]
