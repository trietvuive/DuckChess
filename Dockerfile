FROM rust:1.86-slim AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/

RUN cargo build --release --bin duck_chess && \
    strip target/release/duck_chess

# -------------------------------------------------------------------

FROM python:3.12-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN git clone --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git .

RUN pip install --no-cache-dir -r requirements.txt

RUN mkdir -p /app/engines
COPY --from=builder /build/target/release/duck_chess /app/engines/duck_chess

COPY config.docker.yml /app/config.yml.template
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

ENTRYPOINT ["/app/docker-entrypoint.sh"]
