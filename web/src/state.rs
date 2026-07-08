//! État serveur injecté dans le context Leptos (SSR uniquement).
//! Les server functions le récupèrent via `use_context::<DomainState>()`.

use std::sync::Arc;

use grind_application::{
    BookmarkRepository, Cache, FeedRepository, FollowRepository, HashtagRepository, LikeRepository,
    MatchRepository, MessageRepository, NotificationRepository, PasswordHasher, PostRepository,
    RepostRepository, SportCatalog, TeamFollowRepository, UserRepository,
};
use grind_infrastructure::persistence::{
    DatabaseConnection, SeaOrmBookmarkRepository, SeaOrmFeedRepository, SeaOrmFollowRepository,
    SeaOrmHashtagRepository, SeaOrmLikeRepository, SeaOrmMatchRepository, SeaOrmMessageRepository,
    SeaOrmNotificationRepository, SeaOrmPostRepository, SeaOrmRepostRepository, SeaOrmSportCatalog,
    SeaOrmTeamFollowRepository, SeaOrmUserRepository,
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
    pub catalog: Arc<dyn SportCatalog>,
    pub matches: Arc<dyn MatchRepository>,
    pub team_follows: Arc<dyn TeamFollowRepository>,
    pub messages: Arc<dyn MessageRepository>,
    pub notifications: Arc<dyn NotificationRepository>,
    pub hashtags: Arc<dyn HashtagRepository>,
    pub cache: Arc<dyn Cache>,
    pub users: Arc<dyn UserRepository>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub jwt_secret: Arc<String>,
}

impl DomainState {
    /// `cache` : `RedisCache` en prod, `NoopCache` en dev/tests (cf. `main.rs`).
    pub fn new(db: DatabaseConnection, cache: Arc<dyn Cache>, jwt_secret: String) -> Self {
        Self {
            feed: Arc::new(SeaOrmFeedRepository::new(db.clone())),
            posts: Arc::new(SeaOrmPostRepository::new(db.clone())),
            likes: Arc::new(SeaOrmLikeRepository::new(db.clone())),
            reposts: Arc::new(SeaOrmRepostRepository::new(db.clone())),
            bookmarks: Arc::new(SeaOrmBookmarkRepository::new(db.clone())),
            follows: Arc::new(SeaOrmFollowRepository::new(db.clone())),
            catalog: Arc::new(SeaOrmSportCatalog::new(db.clone())),
            matches: Arc::new(SeaOrmMatchRepository::new(db.clone())),
            team_follows: Arc::new(SeaOrmTeamFollowRepository::new(db.clone())),
            messages: Arc::new(SeaOrmMessageRepository::new(db.clone())),
            notifications: Arc::new(SeaOrmNotificationRepository::new(db.clone())),
            hashtags: Arc::new(SeaOrmHashtagRepository::new(db.clone())),
            cache,
            users: Arc::new(SeaOrmUserRepository::new(db)),
            hasher: Arc::new(PasswordService::new()),
            jwt_secret: Arc::new(jwt_secret),
        }
    }
}
