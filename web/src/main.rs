//! Serveur SSR (leptos_axum) : sert l'app hydratée + les server functions,
//! avec injection du `DomainState` (repos SeaORM) dans le context Leptos.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::body::Body;
    use axum::extract::{FromRef, State};
    use axum::http::Request;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_axum::{
        file_and_error_handler, generate_route_list, handle_server_fns_with_context, LeptosRoutes,
    };

    use grind_web::state::DomainState;
    use grind_web::{shell, App};

    #[derive(Clone)]
    struct AppState {
        leptos_options: LeptosOptions,
        domain: DomainState,
    }

    impl FromRef<AppState> for LeptosOptions {
        fn from_ref(s: &AppState) -> Self {
            s.leptos_options.clone()
        }
    }
    impl FromRef<AppState> for DomainState {
        fn from_ref(s: &AppState) -> Self {
            s.domain.clone()
        }
    }

    async fn server_fns(State(app): State<AppState>, req: Request<Body>) -> impl IntoResponse {
        handle_server_fns_with_context(
            move || provide_context(app.domain.clone()),
            req,
        )
        .await
    }

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
    let routes = generate_route_list(App);

    let app_state = AppState { leptos_options: leptos_options.clone(), domain };

    let app = Router::new()
        .route("/api/*fn_name", get(server_fns).post(server_fns))
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let domain = app_state.domain.clone();
                move || provide_context(domain.clone())
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(file_and_error_handler::<AppState, _>(shell))
        .with_state(app_state);

    leptos::logging::log!("GRIND (hydraté) en écoute sur http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Cible WASM / hydrate : pas de binaire serveur.
}
