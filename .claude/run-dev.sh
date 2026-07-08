#!/bin/zsh
# Lance l'app Leptos (SSR + WASM hydraté) sur http://127.0.0.1:3000
# Wrapper pour garantir l'environnement Rust (cargo/rustc sur le PATH).
. "$HOME/.cargo/env"
cd "$(dirname "$0")/.."
exec cargo leptos serve
