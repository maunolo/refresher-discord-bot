# The bot and its admin page in one image: Node builds the page, Rust builds the binary, and a
# slim runtime runs both. Used by Railway (see railway.json); works with plain `docker build`.

FROM node:22-slim AS page
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1.97-slim-trixie AS app
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# sqlx embeds the migrations at compile time.
COPY migrations ./migrations
RUN cargo build --release --locked -p refresher-app

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data
WORKDIR /app
COPY --from=app /src/target/release/refresher-discord-bot ./refresher-discord-bot
COPY --from=page /web/dist ./web/dist
# The database lives on a volume mounted at /data. The web server listens on every interface;
# the public address and the Discord secrets come from the service's variables.
ENV DATABASE_URL=sqlite:/data/camps.db \
    WEB_BIND=0.0.0.0:8080 \
    WEB_STATIC_DIR=/app/web/dist \
    RUST_LOG=info
EXPOSE 8080
CMD ["./refresher-discord-bot"]
