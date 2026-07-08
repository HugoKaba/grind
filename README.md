# grind-rs — migration Rust (Leptos + Axum + SeaORM, Clean Architecture)

Workspace de la réécriture **full-Rust** de GRIND, à la racine du repo. Voir
`MIGRATION_RUST.md` pour le plan complet. Le projet Django d'origine a été retiré au
cutover : **plus aucun runtime Python** (l'historique git en garde la trace).

## Couches (règle de dépendance : extérieur → intérieur)

| Crate | Couche | Dépend de |
|-------|--------|-----------|
| `crates/domain` | Domaine (entités sport + invariants, pur, WASM-safe) | rien |
| `crates/application` | Use cases + ports (traits) | domain |
| `crates/infrastructure` | Adapters : auth PBKDF2→argon2, SeaORM (Postgres/SQLite), cache Redis | domain, application |
| `crates/shared` | DTOs serde partagés client/serveur | — |
| `web/` | Présentation : app Leptos **hydratée** full-stack (SSR + WASM + server functions), via `cargo-leptos` | tout |

## Lancer avec Docker (Postgres + Redis)

En full-stack Leptos, **back et front sont le même binaire** (le serveur SSR sert le
front hydraté *et* les server functions). Le `docker-compose.yml` orchestre trois
services : `app` (le binaire), `postgres`, `redis`.

```sh
docker compose up --build     # app sur http://localhost:3000
```

Au premier démarrage, l'app applique les migrations SeaORM et seede la base **si elle
est vide** (idempotent : les redémarrages ne re-seedent pas). Le cache Redis sert la
page trending (TTL 30 s) ; si Redis est injoignable, l'app dégrade proprement
(`NoopCache`) sans planter. Config via env (`.env.example`) : `DATABASE_URL`,
`REDIS_URL`, `JWT_SECRET`.

Comptes de démo seedés : `messi` (staff) et `ronaldo`, mot de passe `grind1234`.

## Build / test (local, sans Docker)

Le repo est sur iCloud → rediriger `target/` hors iCloud pour éviter les tempêtes de sync :

```sh
export CARGO_TARGET_DIR=/tmp/grind-rs-target   # target -> target.nosync (hors iCloud)
cargo test --workspace                 # domain/application/infrastructure + intégration web
cargo test -p grind-infrastructure     # auth (hash Django réel), repos SeaORM, cache
cargo test -p grind-web                # server functions via tower::oneshot (e2e)
```

Sans `DATABASE_URL`, le binaire démarre sur **SQLite en mémoire** (dev rapide) ; sans
`REDIS_URL`, le cache est désactivé. La compat **PostgreSQL** des migrations est
couverte par un test *gated* :

```sh
TEST_DATABASE_URL=postgres://user@host/db cargo test -p grind-infrastructure
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
