# syntax=docker/dockerfile:1

# ─────────────────────────────────────────────────────────────
# Étage 1 — build : compile le serveur SSR + le client WASM via cargo-leptos.
# ─────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS builder

# Outils front : cible WASM + wasm-opt (binaryen) + cargo-leptos.
RUN rustup target add wasm32-unknown-unknown \
    && apt-get update && apt-get install -y --no-install-recommends binaryen \
    && rm -rf /var/lib/apt/lists/*
# cargo-leptos 0.3.x embarque le wasm-bindgen-cli qui correspond au crate
# wasm-bindgen 0.2.126 verrouillé dans Cargo.lock (0.2.x → mismatch de schéma WASM).
RUN cargo install cargo-leptos --locked --version ^0.3

WORKDIR /app
COPY . .

# Compile en release : produit target/release/grind-web (serveur) + target/site (assets).
RUN cargo leptos build --release

# ─────────────────────────────────────────────────────────────
# Étage 2 — runtime : image minimale, uniquement le binaire + les assets.
# ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/grind-web /app/grind-web
COPY --from=builder /app/target/site /app/site

# Leptos lit sa config via l'environnement (pas de Cargo.toml au runtime).
ENV LEPTOS_OUTPUT_NAME=grind \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_ENV=PROD \
    RUST_LOG=info

EXPOSE 3000
CMD ["/app/grind-web"]
