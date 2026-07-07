# 🦀 Plan de migration GRIND — Django → Rust (Leptos + Axum + SeaORM)

> Statut : **plan validé, non implémenté**
> Date : 2026-07-06
> Cible : réécriture full-stack Rust d'une plateforme sociale type Twitter.

---

## 1. Décisions d'architecture (verrouillées)

| Sujet | Choix | Justification |
|---|---|---|
| **Backend HTTP** | **Axum** | Défaut de `cargo leptos new`, écosystème tokio/tower/hyper, middleware `tower` réutilisables, pairing naturel avec SeaORM async. |
| **Frontend** | **Leptos (SSR + hydration)** en **server functions full-stack** | Un seul binaire sert le SSR et la logique serveur. Pas d'API REST séparée à maintenir. Transition la plus proche des templates Django actuels. |
| **ORM / BDD** | **SeaORM** sur **PostgreSQL existant** | Vrai ORM async (entités, relations, migrations), expérience proche du Django ORM. On garde la BDD Postgres. |
| **Auth mots de passe** | **Hybride** : vérif PBKDF2-SHA256 (compat Django) au login, re-hash **argon2** au 1er login réussi | Zéro friction pour les users existants + modernisation progressive du hachage. |
| **Cache** | **Redis** conservé (crate `fred` ou `redis`) | Réutilise l'instance Redis existante. |
| **Build** | `cargo-leptos` (binaire unique SSR + WASM) | Toolchain standard Leptos. |
| **Organisation du code** | **Clean Architecture** (hexagonale / ports & adapters) | Domaine découplé des frameworks ; use cases testables sans Axum/SeaORM ; règle de dépendance vers l'intérieur. |

---

## 1-bis. Clean Architecture (couches & règle de dépendance)

**Règle d'or : les dépendances pointent toujours vers l'intérieur.** Le domaine ne connaît ni Axum, ni Leptos, ni SeaORM. L'infrastructure implémente des *ports* (traits) définis par la couche application → **inversion de dépendance**.

```
        ┌───────────────────────────────────────────────────────┐
        │  FRAMEWORKS & DRIVERS (le plus externe)               │
        │  Axum · Leptos · SeaORM · Redis · Postgres · config   │
        │  ┌─────────────────────────────────────────────────┐  │
        │  │  INTERFACE ADAPTERS                             │  │
        │  │  server fns (controllers) · DTOs · mappers ·   │  │
        │  │  impl repositories (SeaORM) · impl cache/hash  │  │
        │  │  ┌───────────────────────────────────────────┐ │  │
        │  │  │  APPLICATION (use cases)                 │ │  │
        │  │  │  PostTweet · ToggleLike · FollowUser ·   │ │  │
        │  │  │  ports (traits): TweetRepo, UserRepo,    │ │  │
        │  │  │  PasswordHasher, Clock, Cache            │ │  │
        │  │  │  ┌─────────────────────────────────────┐ │ │  │
        │  │  │  │  DOMAIN (entities + règles)        │ │ │  │
        │  │  │  │  Tweet, User, Follow, Hashtag...   │ │ │  │
        │  │  │  │  invariants: ≤280 chars, no self-  │ │ │  │
        │  │  │  │  follow, unicité like. Erreurs.    │ │ │  │
        │  │  │  └─────────────────────────────────────┘ │ │  │
        │  │  └───────────────────────────────────────────┘ │  │
        │  └─────────────────────────────────────────────────┘  │
        └───────────────────────────────────────────────────────┘
            sens des dépendances : extérieur ──▶ intérieur
```

| Couche | Contient | Dépend de | Connaît un framework ? |
|---|---|---|---|
| **Domain** | Entités métier, value objects, invariants (≤280 car., pas d'auto-follow…), erreurs domaine | rien (std + `chrono`/`serde` seulement) | ❌ jamais |
| **Application** | Use cases + **ports** (traits `TweetRepository`, `UserRepository`, `PasswordHasher`, `Cache`, `Clock`) | Domain | ❌ |
| **Interface Adapters** | Server functions Leptos (controllers), DTOs, mappers, **impl** des repos (SeaORM), impl hasher/cache | Application + Domain | partiellement |
| **Frameworks & Drivers** | Axum, Leptos runtime, SeaORM, Redis, Postgres, `main.rs`, injection de dépendances | tout le reste | ✅ |

**Spécificité Leptos full-stack (client/serveur partagé) :**
- Le **Domain** et les **DTOs** traversent la frontière réseau → doivent être `serde` **et compilables en WASM** (pas de dépendance serveur).
- **Application (use cases), ports impl, SeaORM, Redis, hashing** = code serveur uniquement → gated derrière `#[cfg(feature = "ssr")]`.
- Une **server function** = un *controller* : elle (dé)sérialise le DTO, récupère les use cases injectés (`use_context` / state Axum), appelle le use case, mappe la sortie. Aucune règle métier dans la server function.

**Bénéfices concrets ici :** tester `PostTweet` sans base (repo mocké en mémoire), remplacer SeaORM par autre chose sans toucher au métier, et une frontière nette entre « ce qui est GRIND » (domaine) et « comment c'est servi » (Axum/Leptos).

---

## 2. Périmètre à migrer (issu de l'audit)

### 2.1 Modèles de données (10 + User)

| Django | SeaORM entity | Notes migration |
|---|---|---|
| `auth.User` | `users` | Table `auth_user` existante conservée. Champ `password` = hash PBKDF2 Django. |
| `Profile` (OneToOne User) | `profile` | Compteurs dénormalisés `followers_count`, `following_count`, `tweets_count`. Signal auto-création → à porter en service. |
| `Follow` (unique follower+following) | `follow` | Index sur follower & following. |
| `Tweet` (self-FK `parent_tweet`) | `tweet` | Compteurs `likes/retweets/replies_count`. Relation récursive (replies). |
| `Like` (unique user+tweet) | `like` | |
| `Retweet` (unique user+tweet) | `retweet` | |
| `Hashtag` (slug auto) | `hashtag` | `save()` slugify → à porter côté service. |
| `TweetHashtag` (M2M) | `tweet_hashtag` | Extraction `#regex` à la création de tweet. |
| `Bookmark` (unique user+tweet) | `bookmark` | |
| `Message` (sender/recipient) | `message` | DMs, `is_read`. |
| `Notification` (types: like/retweet/follow/reply/message) | `notification` | Enum `notification_type`. |

### 2.1-bis Nouvelles entités **sport** (décision : vrai domaine sport)

Le domaine n'est plus un micro-blog générique : on enrichit avec le langage ubiquitaire du sport. Renommages de cohérence : **`Tweet` → `Post`**, **`Retweet` → `Repost`**.

| Nouvelle entité | Rôle | Champs clés |
|---|---|---|
| `Sport` | réf. discipline | `name`, `slug` (Football, Basketball, Tennis…) |
| `Team` (Club) | réf. équipe | `name`, `slug`, `sport_id`, `country`, `crest` |
| `AthleteProfile` (enrichit `Profile`) | profil sportif | + `sport_id`, `team_id?`, `position`, `is_pro`, `jersey_number?` |
| `Match` / `Event` | rencontre (live-posting) | `sport_id`, `home_team_id`, `away_team_id`, `kickoff_at`, `status` (scheduled/live/finished), `score` |
| `Post` (ex-`Tweet`) | +contexte sportif | + `sport_id?`, `match_id?`, `team_id?` (post « à propos de ce match ») |
| `TeamFollow` | suivre une équipe | `user_id`, `team_id` (analogue à `Follow` mais cible = équipe) |

Invariants domaine ajoutés : un `Match` a deux équipes **du même sport**, un `AthleteProfile` `is_pro` appartient à une `Team`, un `Post` lié à un `Match` doit référencer un sport cohérent. **Reference data** (`Sport`, `Team`) seedée ; les footballeurs du seed deviennent des `AthleteProfile` reliés à des `Team`.

### 2.2 Fonctionnalités (server functions à écrire)

- **Timeline** : feed des personnes suivies + tweets perso (auth) / trending (anonyme)
- **Tweets** : créer, détail, supprimer, répondre (thread)
- **Interactions** : like/unlike, retweet/unretweet, bookmark/unbookmark
- **Social** : follow/unfollow (+ maj compteurs dénormalisés)
- **Hashtags** : extraction auto à la création, page trending
- **Profils** : page profil, édition
- **Messagerie** : liste conversations, thread, nouveau message (restreint aux suivis)
- **Notifications** : liste + marquage lu
- **Auth** : login, logout, session/JWT, register

### 2.3 Hors ORM / à remplacer

| Django | Remplacement Rust |
|---|---|
| Django Admin | **Admin Leptos custom dès le départ** (`/admin`, gated `is_staff`, via use cases). Aucun Django résiduel au cutover. Cf. §6-bis B. |
| `drf-spectacular` (OpenAPI) | `utoipa` **si** on expose une API REST publique (optionnel en mode server functions). |
| Signals (`post_save`) | Logique explicite dans les services (`create_profile_for_user`). |
| `seed_sports_data.py` | Binaire `seed` Rust (`src/bin/seed.rs`) rejouant les données. |
| Templates Django (14) | Composants Leptos (`view!` macro). |
| Redis via `django_redis` | Crate `fred`/`redis` dans une couche cache. |

---

## 3. Correspondance des concepts Django → Rust

| Concept Django | Équivalent Rust |
|---|---|
| `models.Model` | SeaORM `Entity` + `Model` + `ActiveModel` |
| Migrations Django | SeaORM migrations (`migration/src/`) |
| DRF Serializer | structs `serde` (`Serialize`/`Deserialize`) + DTOs |
| ViewSet / vue | Leptos **server function** (`#[server]`) ou handler Axum |
| Permissions DRF | Extractors Axum + garde dans les server fns (contexte user) |
| `login_required` | Extractor `AuthUser` (rejette si non authentifié) |
| Template `{% %}` | Composant Leptos `#[component]` + `view!` |
| `QuerySet.filter()` | `Entity::find().filter(...)` SeaORM |
| `select_related`/`prefetch_related` | `find_with_related` / `find_also_related` / jointures SeaORM |
| Session/JWT (simplejwt) | `jsonwebtoken` + cookie de session signé (tower) |
| PBKDF2 password | crate `pbkdf2` (compat) + `argon2` (nouveau) |
| `settings.py` / `.env` | `config` crate + `.env` (`dotenvy`) |

---

## 4. Dépendances Rust cibles (Cargo)

```toml
# Backend / full-stack
leptos, leptos_axum, leptos_meta, leptos_router
axum, tower, tower-http           # HTTP + middleware
tokio                             # runtime async

# BDD
sea-orm                           # ORM
sea-orm-migration                 # migrations

# Auth
jsonwebtoken                      # JWT
argon2                            # hachage moderne
pbkdf2, sha2                      # compat vérif Django
cookie / tower-sessions           # sessions

# Cache
fred (ou redis)                   # Redis

# Divers
serde, serde_json                 # (dé)sérialisation
validator                         # validation (max 280 chars, etc.)
regex                             # extraction hashtags
chrono                            # dates
config, dotenvy                   # configuration
tracing, tracing-subscriber       # logs
utoipa                            # OpenAPI (optionnel, si API REST publique)
```

---

## 5. Migration par phases

> Ordre de construction = **de l'intérieur vers l'extérieur** (domaine d'abord, frameworks en dernier). Chaque couche est testable avant que la suivante existe.

### Phase 0 — Cadrage & préparation (0.5 j)
- [ ] **Clarifier le thème** : modèles = clone Twitter, mais seed + README = sport. Décider le vocabulaire cible (Tweet vs Post…).
- [ ] Geler le schéma de la BDD existante (dump `pg_dump --schema-only`) comme référence.
- [ ] Init **workspace** Rust : crates `domain`, `application`, `infrastructure`, `shared`, `app`.
- [ ] CI : `cargo check`, `cargo clippy`, `cargo leptos build` + garde-fou dépendances (le crate `domain` ne doit lier aucun framework).

### Phase 1 — Domaine (couche 1) (2 j)
- [ ] Entités & value objects micro-blog : `Post`, `PostContent` (≤280), `User`, `Username`, `Profile`, `Follow`, `Hashtag`/`Slug`, `Message`, `Notification`.
- [ ] Entités **sport** : `Sport`, `Team`, `AthleteProfile`, `Match`/`Event`, `TeamFollow`.
- [ ] Invariants purs : contenu ≤280, pas d'auto-follow, unicité like/repost/bookmark, `Match` = 2 équipes même sport, `is_pro` ⇒ rattaché à une `Team`, `Post` lié à un `Match` = sport cohérent.
- [ ] `DomainError` typé. **Aucune** dépendance framework.
- [ ] Tests unitaires du domaine (rapides, sans I/O).

### Phase 2 — Application : use cases + ports (couche 2) (2,5 j)
- [ ] **Ports** (traits) : `PostRepository`, `UserRepository`, `FollowRepository`, `MessageRepository`, `SportRepository`, `TeamRepository`, `MatchRepository`, `PasswordHasher`, `Cache`, `Clock`.
- [ ] Use cases micro-blog : `PostContent`, `ReplyPost`, `ToggleLike`, `ToggleRepost`, `ToggleBookmark`, `FollowUser`, `GetTimeline`, `SendMessage`, `MarkNotificationsRead`, `Login`, `Register`.
- [ ] Use cases sport : `FollowTeam`, `PostAboutMatch`, `GetTeamFeed`, `GetMatchFeed`, `ListSports`/`ListTeams`.
- [ ] Use cases **admin** : `ModeratePost`, `VerifyAthlete`, `BanUser`, `ManageSport/Team/Match/Hashtag`.
- [ ] Maj **atomique** des compteurs dénormalisés (déléguée au repo via transaction) ; extraction hashtags + slugify.
- [ ] Tests des use cases avec **repos mockés en mémoire** (aucune BDD).

### Phase 3 — Infrastructure : adapters + migration de données (couche 3) (3,5 j)
- [ ] Migrations SeaORM : tables existantes (préservées) **+ nouvelles tables sport** (`sport`, `team`, `athlete_profile` ou colonnes ajoutées à `profile`, `match`, `team_follow`, colonnes sport sur `post`).
- [ ] Générer les entités SeaORM ; **impl des ports** : `SeaOrm*Repository` (transactions atomiques), `RedisCache`, `SystemClock`.
- [ ] **Backfill** : seed reference data (`Sport`, `Team`) + rattachement des athlètes existants (messi, ronaldo…) à leurs équipes ; script de migration idempotent.
- [ ] **Sécurité** `PbkdfArgonHasher` : parse hash Django (`pbkdf2_sha256$<iter>$<salt>$<hash>`), vérif PBKDF2 ; si OK et hash non-argon2 → re-hash argon2 (login **hybride**). JWT.
- [ ] Porter/étendre `seed_sports_data.py` → `app/src/bin/seed.rs` (sports + équipes + matchs).
- [ ] Tests d'intégration (base jetable) ; un user seedé Django doit se connecter + données sport cohérentes.

### Phase 4 — Présentation : Leptos + Axum + Admin (couche 4) (5,5 j)
- [ ] `main.rs` : bootstrap Axum + `leptos_axum`, `AppState` (injection des impls derrière `Arc<dyn Port>`).
- [ ] Extractor `AuthUser` (JWT + cookie session signé) ; garde d'autorisation = ex-permissions DRF ; garde `is_staff` pour `/admin`.
- [ ] **Server functions = controllers** : (dé)sérialisent le DTO, appellent le use case injecté, mappent la sortie. Zéro métier dedans.
- [ ] Pages publiques : timeline, détail post + thread, profil athlète, **page équipe**, **page match (live)**, trending, bookmarks, notifications, messagerie, login.
- [ ] **Admin Leptos** (`/admin`, gated `is_staff`) : modération posts, vérif/ban athlète, CRUD `Sport`/`Team`/`Match`/`Hashtag` — via les use cases admin (Phase 2). Remplace le Django admin **avant** cutover.
- [ ] Formulaires (post, reply, message, post-about-match) avec validation.

### Phase 5 — Cache, obs & durcissement (1.5 j)
- [ ] Couche cache Redis (timeline, trending, compteurs chauds).
- [ ] `tracing` (logs structurés), gestion d'erreurs unifiée (`thiserror`/`anyhow`).
- [ ] Headers sécurité (CSRF pour formulaires, CORS si API, clickjacking) via `tower-http`.
- [ ] (Optionnel) OpenAPI `utoipa` si une API REST publique est conservée.

### Phase 6 — Bascule & déploiement (1.5 j)
- [ ] `Dockerfile` multi-stage Rust + `docker-compose.yml` (app + Postgres + Redis).
- [ ] Migration de données : **aucune** si on pointe la même BDD (schéma préservé) ; sinon dump/restore.
- [ ] Tests E2E sur les parcours clés.
- [ ] Bascule (blue/green ou reverse-proxy).
- [ ] Gel du code Django (branche d'archive).

**Estimation totale : ~17,5 jours-homme** (hors imprévus). Le passage à un **vrai domaine sport** (+entités/tables) et l'**admin Leptos dès le départ** ont augmenté le périmètre (~+3,5 j vs micro-blog générique). Chemin critique = **Phase 3 (auth hybride + migration/backfill de données)**. Le domaine/use cases (Phases 1-2) restent bas risque et testables sans BDD.

---

## 6. Risques & points de vigilance

| Risque | Impact | Mitigation |
|---|---|---|
| **Hashes PBKDF2 Django** mal parsés | Users bloqués au login | Tests dédiés sur un hash réel Django dès Phase 2 ; format `pbkdf2_sha256$iterations$salt$b64hash`. |
| **Races sur compteurs dénormalisés** | Compteurs faux | Updates atomiques SQL (`SET x = x + 1`) en transaction, pas de read-modify-write applicatif. |
| **Pas de Django Admin** | Perte d'outil d'exploitation | Décision Phase 4 (admin custom minimal ou outil DB externe). |
| **Requêtes N+1** (ex-`prefetch_related`) | Perf timeline | Jointures SeaORM explicites + cache Redis. |
| **Courbe Leptos/WASM** | Vélocité front | Commencer par 1 page de bout en bout (timeline) avant de généraliser. |
| **Thème incohérent** (Twitter vs sport) | Dette produit | ✅ Tranché : vrai domaine sport (§6-bis A). Purger vestiges `qa_*`/`PlayThread`/`Question*`. |
| **Migration de données** (nouvelles tables sport, schéma ≠ 1:1) | Données incohérentes / perte | Migrations idempotentes + backfill scripté et testé sur copie de la BDD ; `Match`/`Team` seedés comme reference data avant rattachement des athlètes. |
| **Admin Leptos requis avant cutover** | Bascule bloquée sans outil d'ops | Prioriser les actions admin réellement nécessaires ; construire un CRUD minimal (pas un clone exhaustif du Django admin). |
| **Perte de `drf-spectacular`** | Doc API | `utoipa` seulement si API REST publique conservée (sinon inutile en full server functions). |

---

## 6-bis. Décisions produit & sécurité

### A. Incohérence de thème (4 couches de nommage télescopées)

L'audit révèle un projet **template Q&A → cloné en Twitter générique → skinné « sport »** uniquement en surface :

| Couche | Vestige trouvé | Preuve |
|---|---|---|
| **Q&A (StackOverflow-like)** | `CanCreateQuestionComment`, docstrings « question/réponse » | `api/permissions.py` |
| **QA platform** | `DB_NAME=qa_platform`, `DB_USER=qa_user` | `.env` |
| **PlayThread** | `🏆 Creating PlayThread sports athletes`, `PlayThread database seeded` | `seed_sports_data.py` |
| **GRIND / Sport** | « GRIND - Sports Social Platform (Twitter/Threads clone) » | `README.md`, nom du dossier |

**Constat clé :** le modèle de domaine (`Tweet`, `Like`, `Retweet`, `Hashtag`) est un **micro-blog générique**. Le « sport » n'existe **pas** dans le domaine — c'est de la donnée de seed (footballeurs) + du texte d'UI. Il faut fixer le **langage ubiquitaire** (Clean Arch) **avant** d'écrire le crate `domain`.

**✅ DÉCIDÉ — B : vrai domaine sport.** Le domaine est enrichi avec `Sport`, `Team`, `AthleteProfile`, `Match/Event`, `TeamFollow` (cf. §2.1-bis). Renommages : `Tweet → Post`, `Retweet → Repost`. Vestiges à purger dans tous les cas : `qa_*`, `PlayThread`, `Question*`.

> **Conséquence schéma :** le round-trip 1:1 avec le schéma Django n'est plus l'objectif. On migre les tables existantes **et** on ajoute les tables sport + un **backfill** (reference data + rattachement des athlètes seedés à des équipes). Voir risque « migration de données » §6.

### B. Migration du Django Admin

Pas d'équivalent Rust clé-en-main. Propositions (combinables) :

| Option | Effort | Applique les règles métier ? | Idée |
|---|---|---|---|
| **1. Garder un Django admin « fin »** (pendant la migration) | ~0 | ✅ (Django) | Le vieux projet Django tourne en **mode admin-only** sur la **même** BDD Postgres, accès staff interne uniquement. Débloque tout de suite, aucune réécriture. Rançon : garder un runtime Python. |
| **2. Outil DB externe** (Adminer / pgAdmin / DBeaver / Beekeeper) | ~0 | ❌ (CRUD brut) | Escape-hatch pour l'exploitation directe. Aucune validation domaine. |
| **3. Admin Leptos custom** (natif Clean Arch) | moyen | ✅ (mêmes use cases) | Pages `/admin` gated `is_staff`, qui **appellent les mêmes use cases** → invariants respectés. Ne construire que le nécessaire (modérer posts, bannir users, gérer hashtags). |
| **4. Framework admin Rust** (loco.rs, create-rust-app) | élevé | partiel | Pas d'intégration propre avec notre stack Leptos+Axum+SeaORM hand-rolled. **Écarté.** |

**✅ DÉCIDÉ — Option 3 : admin Leptos dès le départ.** Pas de Django résiduel → au cutover, **plus aucun runtime Python**. L'admin Leptos (`/admin`, gated `is_staff`) est construit en Phase 4 et doit couvrir les actions d'exploitation **avant** la bascule. Il appelle les **mêmes use cases** → invariants domaine respectés. Adminer/DBeaver reste tolérés comme escape-hatch BDD manuel, hors application. Actions admin cibles : modérer un `Post`, vérifier/bannir un athlète, gérer `Sport`/`Team`/`Match`/`Hashtag`.

### C. Sécurité — ✅ FAIT (passage en variables d'environnement)

Le `settings.py` **ignorait** le `.env` (secrets/DEBUG codés en dur). Corrigé :

| Avant (codé en dur) | Après (env-driven) |
|---|---|
| `SECRET_KEY = 'django-insecure-...'` | `config('SECRET_KEY')` — **sans défaut**, échoue fort si absent ; nouvelle clé forte générée dans `.env` |
| `DEBUG = True` | `config('DEBUG', default=False, cast=bool)` |
| `ALLOWED_HOSTS = []` | `config('ALLOWED_HOSTS', ..., cast=Csv())` |
| Redis `LOCATION` en dur | `config('REDIS_URL', ...)` |
| flags sécu du `.env` non lus | `SECURE_SSL_REDIRECT`, `SESSION_COOKIE_SECURE`, `CSRF_COOKIE_SECURE`, `CSRF_TRUSTED_ORIGINS`, `SECURE_HSTS_SECONDS`, `SECURE_PROXY_SSL_HEADER` |

Ajouts : `.env.example` (committé, sans secret) comme référence. `.env` reste gitignored. **À porter tel quel côté Rust** : couche `config`/`dotenvy`, mêmes clés, secret jamais dans le binaire.

---

## 7. Structure de projet cible (workspace Clean Architecture)

Un crate par couche → la règle de dépendance est **imposée par le compilateur** (le crate `domain` n'a aucune dépendance framework, donc impossible d'y importer Axum/SeaORM par erreur).

> Le workspace Rust est **à la racine du repo** ; l'ancien projet Django est
> déplacé dans `legacy-django/` (il continue de tourner jusqu'au cutover, puis est retiré).

```
.  (racine du repo)
├── Cargo.toml                     # [workspace]
├── legacy-django/                 # ⬅ projet Django existant (manage.py, grind/, core/, api/…)
│
├── crates/
│   ├── domain/                    # ⬅ COUCHE 1 — pur, WASM-safe, zéro framework
│   │   └── src/
│   │       ├── entities/          # Tweet, User, Profile, Follow, Hashtag, Message...
│   │       ├── value_objects/     # TweetContent (≤280), Username, Slug...
│   │       └── error.rs           # DomainError (SelfFollow, ContentTooLong...)
│   │
│   ├── application/               # ⬅ COUCHE 2 — use cases + ports (ssr)
│   │   └── src/
│   │       ├── ports/             # traits: TweetRepository, UserRepository,
│   │       │                      #         PasswordHasher, Cache, Clock
│   │       └── use_cases/         # PostTweet, ToggleLike, FollowUser,
│   │                              # GetTimeline, SendMessage, Login...
│   │
│   ├── infrastructure/            # ⬅ COUCHE 3 — impl des ports (ssr)
│   │   └── src/
│   │       ├── persistence/       # entités SeaORM + impl *Repository
│   │       ├── migration/         # migrations SeaORM
│   │       ├── security/          # PbkdfArgonHasher (compat + argon2), JWT
│   │       └── cache/             # RedisCache
│   │
│   └── shared/                    # DTOs serde partagés client/serveur (WASM-safe)
│
├── app/                           # ⬅ COUCHE 4 — Leptos + Axum (le binaire)
│   └── src/
│       ├── main.rs                # bootstrap Axum + leptos_axum + injection deps
│       ├── app.rs                 # <App/> root + routing Leptos
│       ├── components/            # composants UI
│       ├── pages/                 # timeline, profil, thread, dms, trending...
│       ├── server_fns/            # #[server] = controllers (appellent les use cases)
│       └── bin/seed.rs            # ex-seed_sports_data
│
├── Dockerfile
└── docker-compose.yml
```

**Flux d'une requête** (ex. « poster un tweet ») :
`server_fn PostTweet` (app) → mappe DTO→domaine → `PostTweetUseCase` (application) → via port `TweetRepository` → `SeaOrmTweetRepository` (infrastructure) → Postgres. Le retour remonte en DTO. Le use case ne sait pas que SeaORM ou Leptos existent.

**Injection de dépendances** : `main.rs` construit les impls concrètes (SeaORM, Redis, hasher), les enveloppe dans un `AppState` (Arc<dyn Trait>), fourni aux server functions via le state Axum / `provide_context` Leptos.

---

## 8. Prochaine étape

Au choix pour démarrer l'implémentation :
1. **Scaffold** le workspace Rust (`cargo leptos new` + structure Phase 0/1).
2. **Spike auth** d'abord (Phase 2) — le risque n°1 — pour valider la compat PBKDF2 sur un vrai hash Django.
3. **Générer les entités SeaORM** depuis la BDD existante (Phase 1).
```
