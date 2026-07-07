//! Binaire serveur : Axum + Leptos SSR.
//! Config via env : `DATABASE_URL` (défaut sqlite mémoire), `JWT_SECRET`, `BIND`.

use grind_app::{app_router, build_state};
use grind_infrastructure::persistence::connect_and_migrate;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_owned());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-insecure-secret".to_owned());
    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());

    let db = connect_and_migrate(&db_url)
        .await
        .expect("connexion + migrations BDD");

    let app = app_router(build_state(db, jwt_secret));

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("bind du port");
    tracing::info!("GRIND en écoute sur http://{bind}");
    axum::serve(listener, app).await.expect("serveur axum");
}
