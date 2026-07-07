//! # grind-app
//!
//! Couche présentation (Clean Architecture, couche 4) : Axum + Leptos SSR.
//! Les controllers appellent les use cases injectés via `AppState`.

pub mod auth;
pub mod handlers;
pub mod state;
pub mod views;

use axum::routing::{delete, get, post};
use axum::Router;

pub use state::{build_state, AppState};

/// Construit le routeur applicatif (réutilisé par le binaire et les tests).
pub fn app_router(state: AppState) -> Router {
    Router::new()
        // pages SSR (Leptos)
        .route("/", get(handlers::timeline_html))
        .route("/post/:id", get(handlers::post_detail_html))
        .route("/u/:username", get(handlers::profile_html))
        // API JSON / controllers
        .route("/api/timeline", get(handlers::timeline_json))
        .route("/api/login", post(handlers::login))
        .route("/api/posts", post(handlers::create_post))
        // admin (staff only)
        .route("/api/admin/posts/:id", delete(handlers::admin_delete_post))
        .with_state(state)
}
