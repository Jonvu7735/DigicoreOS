# Multi-stage build for ANY DigicoreOS service. Select with a build arg:
#   docker build -f deploy/Dockerfile --build-arg SERVICE=auth-svc -t auth-svc .
# (Build context is the repo root so the whole workspace + Cargo.lock are present.)
#
# SQL migrations are embedded into the binary at compile time
# (`sqlx::migrate!("./migrations")`), so the runtime image needs no SQL files.

FROM rust:1.96-slim AS builder
ARG SERVICE
# Manifest to build: defaults to the core workspace; a vertical (its own separate
# workspace) passes its own, e.g. MANIFEST_PATH=verticals/trade-export-svc/Cargo.toml.
ARG MANIFEST_PATH=Cargo.toml
# Build deps for ring (transitive via jsonwebtoken/rustls): a C toolchain + perl.
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential perl pkg-config \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
# `dirname $MANIFEST_PATH`/target is the workspace target dir for both layouts.
RUN cargo build --release --locked --manifest-path "${MANIFEST_PATH}" -p "${SERVICE}" \
 && cp "$(dirname "${MANIFEST_PATH}")/target/release/${SERVICE}" /build/app-bin

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
# Run as a non-root user.
RUN useradd --system --uid 10001 digicore
COPY --from=builder /build/app-bin /usr/local/bin/digicore-service
USER digicore
ENTRYPOINT ["/usr/local/bin/digicore-service"]
