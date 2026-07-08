//! Serveur SSR (leptos_axum) : sert l'app hydratée + les server functions,
//! avec injection du `DomainState` (repos SeaORM + cache Redis) dans le context Leptos.
//!
//! Configuration par variables d'environnement :
//! - `DATABASE_URL` : ex. `postgres://grind:grind@postgres/grind` (défaut : SQLite en mémoire) ;
//! - `REDIS_URL`    : ex. `redis://redis:6379` (défaut : cache désactivé / NoopCache) ;
//! - `JWT_SECRET`   : secret de signature des sessions (défaut dev non sûr).

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::sync::Arc;

    use grind_application::Cache;
    use grind_infrastructure::cache::{NoopCache, RedisCache};
    use grind_infrastructure::persistence::{connect_and_migrate, is_empty};
    use grind_infrastructure::security::PasswordService;
    use grind_web::server::{build_router, AppState};
    use grind_web::state::DomainState;
    use leptos::config::get_configuration;

    // --- Profiling continu Grafana Pyroscope : actif uniquement si PYROSCOPE_URL défini.
    //     Taggé `cache=on|off` (selon REDIS_URL) pour comparer AVANT/APRÈS dans la vue diff. ---
    let _pyroscope_guard = match std::env::var("PYROSCOPE_URL") {
        Ok(url) => {
            let cache_tag = if std::env::var("REDIS_URL").is_ok() { "on" } else { "off" };
            let backend = pyroscope_pprofrs::pprof_backend(
                pyroscope_pprofrs::PprofConfig::new().sample_rate(100),
            );
            match pyroscope::PyroscopeAgent::builder(url.as_str(), "grind-web")
                .backend(backend)
                .tags([("cache", cache_tag)].to_vec())
                .build()
            {
                Ok(agent) => match agent.start() {
                    Ok(running) => {
                        leptos::logging::log!("Pyroscope actif (tag cache={cache_tag}).");
                        Some(running)
                    }
                    Err(e) => {
                        leptos::logging::log!("Pyroscope start échec : {e}");
                        None
                    }
                },
                Err(e) => {
                    leptos::logging::log!("Pyroscope build échec : {e}");
                    None
                }
            }
        }
        Err(_) => None,
    };

    // --- Base de données : Postgres en prod (DATABASE_URL), SQLite mémoire par défaut ---
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_owned());
    let db = connect_and_migrate(&database_url)
        .await
        .expect("connexion + migrations BDD");

    // --- Seed idempotent : uniquement si la base est vide (redémarrages Postgres) ---
    if is_empty(&db).await.expect("vérification BDD vide") {
        seed(&db, &PasswordService::new()).await;
        leptos::logging::log!("BDD vide → seed initial appliqué.");
    } else {
        leptos::logging::log!("BDD déjà peuplée → seed ignoré.");
    }

    // --- Cache : Redis si REDIS_URL est fourni et joignable, sinon NoopCache ---
    let cache: Arc<dyn Cache> = match std::env::var("REDIS_URL") {
        Ok(url) => match RedisCache::connect(&url).await {
            Ok(c) => {
                leptos::logging::log!("Redis connecté ({url}).");
                Arc::new(c)
            }
            Err(e) => {
                leptos::logging::log!("Redis injoignable ({e}) → cache désactivé.");
                Arc::new(NoopCache)
            }
        },
        Err(_) => Arc::new(NoopCache),
    };

    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-insecure-secret".to_owned());
    let domain = DomainState::new(db, cache, jwt_secret);

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let app = build_router(AppState { leptos_options, domain });

    leptos::logging::log!("GRIND (hydraté) en écoute sur http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

/// Seed de démonstration : reference data + athlètes + un post d'accueil.
#[cfg(feature = "ssr")]
async fn seed(
    db: &grind_infrastructure::persistence::DatabaseConnection,
    hasher: &grind_infrastructure::security::PasswordService,
) {
    use grind_application::PostRepository;
    use grind_domain::entities::UserId;
    use grind_domain::value_objects::PostContent;

    grind_infrastructure::persistence::seed::seed_reference_and_athletes(db, hasher)
        .await
        .expect("seed reference data");

    let posts = grind_infrastructure::persistence::SeaOrmPostRepository::new(db.clone());
    let content = PostContent::new("Golazo en SSR hydraté ! #Goals").unwrap();
    let _ = posts.insert(UserId(1), &content, None).await;
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Cible WASM / hydrate : pas de binaire serveur.
}
