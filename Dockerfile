FROM node:24-bookworm-slim AS frontend
WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

FROM rust:1.98.1-bookworm AS backend
WORKDIR /build
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /build/target/release/space-weather /usr/local/bin/space-weather
COPY --from=frontend /build/dist /app/frontend
ENV BIND_ADDR=0.0.0.0:3000 STATIC_DIR=/app/frontend
EXPOSE 3000
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/space-weather"]

