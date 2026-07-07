//! État serveur injecté dans le context Leptos (SSR uniquement).
//! Les server functions le récupèrent via `use_context::<DomainState>()`.

use std::sync::Arc;

use grind_application::{
    BookmarkRepository, FeedRepository, FollowRepository, LikeRepository, PasswordHasher,
    PostRepository, RepostRepository, UserRepository,
};
use grind_infrastructure::persistence::{
    DatabaseConnection, SeaOrmBookmarkRepository, SeaOrmFeedRepository, SeaOrmFollowRepository,
    SeaOrmLikeRepository, SeaOrmPostRepository, SeaOrmRepostRepository, SeaOrmUserRepository,
};
use grind_infrastructure::security::PasswordService;

#[derive(Clone)]
pub struct DomainState {
    pub feed: Arc<dyn FeedRepository>,
    pub posts: Arc<dyn PostRepository>,
    pub likes: Arc<dyn LikeRepository>,
    pub reposts: Arc<dyn RepostRepository>,
    pub bookmarks: Arc<dyn BookmarkRepository>,
    pub follows: Arc<dyn FollowRepository>,
    pub users: Arc<dyn UserRepository>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub jwt_secret: Arc<String>,
}

impl DomainState {
    pub fn new(db: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            feed: Arc::new(SeaOrmFeedRepository::new(db.clone())),
            posts: Arc::new(SeaOrmPostRepository::new(db.clone())),
            likes: Arc::new(SeaOrmLikeRepository::new(db.clone())),
            reposts: Arc::new(SeaOrmRepostRepository::new(db.clone())),
            bookmarks: Arc::new(SeaOrmBookmarkRepository::new(db.clone())),
            follows: Arc::new(SeaOrmFollowRepository::new(db.clone())),
            users: Arc::new(SeaOrmUserRepository::new(db)),
            hasher: Arc::new(PasswordService::new()),
            jwt_secret: Arc::new(jwt_secret),
        }
    }
}
