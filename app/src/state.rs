//! État applicatif : injection de dépendances (impls concrètes derrière des ports).
//! Les server functions / handlers ne connaissent que les traits (`Arc<dyn Port>`).

use std::sync::Arc;

use grind_application::{
    FeedRepository, FollowRepository, LikeRepository, PasswordHasher, PostRepository,
    UserRepository,
};
use grind_infrastructure::persistence::{
    DatabaseConnection, SeaOrmFeedRepository, SeaOrmFollowRepository, SeaOrmLikeRepository,
    SeaOrmPostRepository, SeaOrmUserRepository,
};
use grind_infrastructure::security::PasswordService;

#[derive(Clone)]
pub struct AppState {
    pub posts: Arc<dyn PostRepository>,
    pub follows: Arc<dyn FollowRepository>,
    pub likes: Arc<dyn LikeRepository>,
    pub users: Arc<dyn UserRepository>,
    pub feed: Arc<dyn FeedRepository>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub jwt_secret: Arc<String>,
}

/// Construit l'état à partir d'une connexion BDD migrée + un secret JWT.
pub fn build_state(db: DatabaseConnection, jwt_secret: String) -> AppState {
    AppState {
        posts: Arc::new(SeaOrmPostRepository::new(db.clone())),
        follows: Arc::new(SeaOrmFollowRepository::new(db.clone())),
        likes: Arc::new(SeaOrmLikeRepository::new(db.clone())),
        users: Arc::new(SeaOrmUserRepository::new(db.clone())),
        feed: Arc::new(SeaOrmFeedRepository::new(db)),
        hasher: Arc::new(PasswordService::new()),
        jwt_secret: Arc::new(jwt_secret),
    }
}
