# grind-rs — migration Rust (Leptos + Axum + SeaORM, Clean Architecture)

Workspace de la réécriture de GRIND, **à la racine du repo**. Voir `MIGRATION_RUST.md`
pour le plan complet. L'ancien projet Django vit dans `legacy-django/` (retiré au cutover).

## Couches (règle de dépendance : extérieur → intérieur)

| Crate | Couche | Dépend de |
|-------|--------|-----------|
| `crates/domain` | Domaine (entités sport + invariants, pur, WASM-safe) | rien |
| `crates/application` | Use cases + ports (traits) | domain |
| `crates/infrastructure` | Adapters (auth PBKDF2→argon2, plus tard SeaORM/Redis) | domain, application |
| `crates/shared` | DTOs serde partagés client/serveur | — |
| `web/` | Présentation : app Leptos **hydratée** full-stack (SSR + WASM + server functions), via `cargo-leptos` | tout |

## Build / test

Le repo est sur iCloud → rediriger `target/` hors iCloud pour éviter les tempêtes de sync :

```sh
export CARGO_TARGET_DIR=/tmp/grind-rs-target   # target -> target.nosync (hors iCloud)
cargo test --workspace                 # logique métier (domain/application/infrastructure)
cargo test -p grind-infrastructure     # spike auth (vérif hash Django réel) + repos SeaORM
```

### Frontend hydraté (`web/`)

```sh
cd web
cargo leptos build      # compile le serveur SSR + le client WASM (hydratation)
cargo leptos serve      # sert l'app hydratée sur http://127.0.0.1:3000
```

`web/` réalise le pipeline **server functions + hydratation câblé sur les use cases réels** :
- `get_timeline` (lecture), `login` (pose un cookie de session HttpOnly), `create_post`
  (auth par cookie) — toutes câblées sur les use cases via `DomainState` injecté dans le
  context Leptos ;
- UI hydratée : formulaires (`ActionForm`) connexion + publication, fil `<Suspense>` qui se
  recharge après chaque post, îlot compteur réactif.

Routes SSR + hydratées : `/` (fil), `/post/:id` (détail + thread), `/u/:username` (profil),
`/admin` (modération, `delete_post` réservé au staff via le cookie de session).

Vérifié runtime : login → cookie ; create_post/delete_post gated ; pages détail/profil/admin
rendues en SSR ; non-staff → « Réservé au staff ».
