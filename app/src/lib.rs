//! # grind-app
//!
//! Couche présentation (Clean Architecture, couche 4) : Axum + Leptos SSR.
//! Les controllers appellent les use cases injectés via `AppState`.

pub mod auth;
pub mod handlers;
pub mod state;
pub mod views;

use axum::routing::{get, post};
use axum::Router;

pub use state::{build_state, AppState};

/// Construit le routeur applicatif (réutilisé par le binaire et les tests).
pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::timeline_html))
        .route("/api/timeline", get(handlers::timeline_json))
        .route("/api/login", post(handlers::login))
        .route("/api/posts", post(handlers::create_post))
        .with_state(state)
}
