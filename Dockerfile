# The API image: a cargo-chef staged build so dependency compilation is
# cached across code changes. Ships the server plus the sde_import
# bootstrap binary with the runtime assets (served images, docs).

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY migrations migrations
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY migrations migrations
# The OpenGraph renderer compiles its fonts and card textures in with
# `include_bytes!`, so the assets tree has to exist at build time.
COPY assets assets
RUN cargo build --release --bin mutamarket --bin sde_import

FROM debian:bookworm-slim AS runtime
# Model decodes run on varying blocking threads; without this glibc grows
# one arena per thread and the freed forests never return to the OS.
ENV MALLOC_ARENA_MAX=2
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/mutamarket /app/target/release/sde_import /usr/local/bin/
COPY assets assets
# The api writes the OpenGraph cache and the synced store creatives.
RUN useradd --system --create-home --uid 10001 app \
    && mkdir -p storage/app/public/og storage/sde assets/img/ads \
    && chown -R app:app /app
USER app
CMD ["mutamarket"]
