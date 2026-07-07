//! Serveur SSR (leptos_axum) : sert l'app hydratée + les server functions,
//! avec injection du `DomainState` (repos SeaORM) dans le context Leptos.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::config::get_configuration;
    use grind_web::server::{build_router, AppState};
    use grind_web::state::DomainState;

    // --- BDD de démo : SQLite en mémoire, migrée + seedée + 1 post ---
    let db = grind_infrastructure::persistence::connect_and_migrate("sqlite::memory:")
        .await
        .expect("db");
    grind_infrastructure::persistence::seed::seed_reference_and_athletes(
        &db,
        &grind_infrastructure::security::PasswordService::new(),
    )
    .await
    .expect("seed");
    {
        use grind_application::PostRepository;
        use grind_domain::entities::UserId;
        use grind_domain::value_objects::PostContent;
        let posts = grind_infrastructure::persistence::SeaOrmPostRepository::new(db.clone());
        let content = PostContent::new("Golazo en SSR hydraté ! #Goals").unwrap();
        let _ = posts.insert(UserId(1), &content, None).await;
    }

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-insecure-secret".to_owned());
    let domain = DomainState::new(db, jwt_secret);

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let app = build_router(AppState { leptos_options, domain });

    leptos::logging::log!("GRIND (hydraté) en écoute sur http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Cible WASM / hydrate : pas de binaire serveur.
}
