# grind-rs — migration Rust (Leptos + Axum + SeaORM, Clean Architecture)

Workspace de la réécriture **full-Rust** de GRIND, à la racine du repo. Voir
`MIGRATION_RUST.md` pour le plan complet. Le projet Django d'origine a été retiré au
cutover : **plus aucun runtime Python** (l'historique git en garde la trace).

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
cargo test --workspace                 # domain/application/infrastructure + intégration web
cargo test -p grind-infrastructure     # spike auth (vérif hash Django réel) + repos SeaORM
cargo test -p grind-web                # server functions via tower::oneshot (login, post, admin)
```

### Frontend hydraté (`web/`)

```sh
cd web
cargo leptos build      # compile le serveur SSR + le client WASM (hydratation)
cargo leptos serve      # sert l'app hydratée sur http://127.0.0.1:3000
```

`web/` réalise le pipeline **server functions + hydratation câblé sur les use cases réels**.
Chaque server function est un *controller* pur : elle (dé)sérialise le DTO, récupère les
use cases via `DomainState` (injecté dans le context Leptos), appelle le use case, mappe la
sortie. Aucune règle métier dans la couche présentation.

Fonctionnalités (toutes câblées de bout en bout, testées aux 4 couches) :
- **Auth** : `register` (+ auto-login), `login` (hybride PBKDF2 Django → argon2, cookie de
  session HttpOnly), garde `is_staff`.
- **Posts** : publication, réponse (thread), suppression (modération admin).
- **Interactions** *viewer-aware* : like, repost, bookmark (état persistant `*_by_me`).
- **Social** : follow/unfollow, fil « following » (suivis + soi) vs récent (anonyme).
- **Sport** : catalogue sports/équipes, suivi d'équipe, fil de match (live), post-about-match.
- **Messagerie** : DMs restreints aux personnes suivies, threads, conversations.
- **Notifications** : générées à l'envoi d'un message, liste + marquage lu.
- **Hashtags** : extraction auto des `#tags` à la publication + page trending.

Routes SSR + hydratées : `/` (fil), `/post/:id` (détail + thread), `/u/:username` (profil),
`/sports`, `/trending`, `/team/:slug`, `/match/:id`, `/messages` (+ `/:username`),
`/notifications`, `/admin`.
