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
| `app/` | Présentation : Axum + Leptos SSR + auth JWT/cookie + admin (REST + pages SSR, testé) | tout |
| `web/` | App Leptos **hydratée** full-stack (SSR + WASM + server functions), via `cargo-leptos` | leptos |

## Build / test

Le repo est sur iCloud → rediriger `target/` hors iCloud pour éviter les tempêtes de sync :

```sh
export CARGO_TARGET_DIR=/tmp/grind-rs-target
cargo test --workspace                 # tous les crates (24 tests)
cargo test -p grind-infrastructure     # spike auth (vérif hash Django réel)
cargo test -p grind-app                # intégration présentation→application→infra
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

Vérifié runtime : `login` → 200 + cookie ; `create_post` avec cookie → 200 puis le post
apparaît dans le fil SSR ; sans cookie → « Non authentifié ».

Prochaines étapes : migrer les pages détail/profil + l'admin de `app/` vers `web/`,
puis retirer `app/` (fusion).
