# grind-rs — migration Rust (Leptos + Axum + SeaORM, Clean Architecture)

Workspace de la réécriture de GRIND. Voir `../MIGRATION_RUST.md` pour le plan complet.

## Couches (règle de dépendance : extérieur → intérieur)

| Crate | Couche | Dépend de |
|-------|--------|-----------|
| `crates/domain` | Domaine (entités sport + invariants, pur, WASM-safe) | rien |
| `crates/application` | Use cases + ports (traits) | domain |
| `crates/infrastructure` | Adapters (auth PBKDF2→argon2, plus tard SeaORM/Redis) | domain, application |
| `crates/shared` | DTOs serde partagés client/serveur | — |
| `app/` *(Phase 4)* | Leptos + Axum (binaire) | tout |

## Build / test

Le repo est sur iCloud → rediriger `target/` hors iCloud pour éviter les tempêtes de sync :

```sh
export CARGO_TARGET_DIR=/tmp/grind-rs-target
cargo test            # tous les crates
cargo test -p grind-domain
cargo test -p grind-infrastructure   # spike auth (vérif hash Django réel)
```
