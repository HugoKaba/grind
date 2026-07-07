//! # grind-application
//!
//! Use cases + **ports** (traits). Dépend uniquement du domaine.
//! L'infrastructure (SeaORM, Redis…) implémentera ces ports → inversion de dépendance.

use async_trait::async_trait;
use grind_domain::entities::{Follow, Post, PostId, UserId};
use grind_domain::value_objects::PostContent;
use grind_domain::DomainError;

/// Erreur d'un port d'infrastructure (BDD indisponible, contrainte, etc.).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepoError {
    #[error("entity not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("backend error: {0}")]
    Backend(String),
}

/// Erreur applicative : soit une violation métier, soit un échec d'infra.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("auth error: {0}")]
    Auth(String),
}

// ---------------------------------------------------------------------------
// Ports (interfaces) — implémentés par l'infrastructure.
// ---------------------------------------------------------------------------

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn insert(
        &self,
        author: UserId,
        content: &PostContent,
        parent: Option<PostId>,
    ) -> Result<Post, RepoError>;
}

#[async_trait]
pub trait FollowRepository: Send + Sync {
    async fn add(&self, follow: &Follow) -> Result<bool, RepoError>;
    async fn exists(&self, follower: UserId, following: UserId) -> Result<bool, RepoError>;
}

/// Résultat d'un (dé)like : compteur mis à jour **atomiquement** côté repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LikeToggle {
    /// `true` si l'état a réellement changé (like créé / supprimé).
    pub changed: bool,
    /// Nombre de likes du post après opération.
    pub likes_count: i64,
}

#[async_trait]
pub trait LikeRepository: Send + Sync {
    /// Ajoute un like (idempotent) + incrémente atomiquement `likes_count`.
    async fn like(&self, user: UserId, post: PostId) -> Result<LikeToggle, RepoError>;
    /// Retire un like (idempotent) + décrémente atomiquement `likes_count`.
    async fn unlike(&self, user: UserId, post: PostId) -> Result<LikeToggle, RepoError>;
}

/// Enregistrement minimal pour l'authentification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUserRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn by_username(&self, username: &str) -> Result<Option<AuthUserRecord>, RepoError>;
    async fn update_password(&self, user_id: i64, new_hash: &str) -> Result<(), RepoError>;
}

/// Résultat de vérification d'un mot de passe (port infra, cf. auth hybride).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheck {
    Invalid,
    Valid,
    /// Correct mais hash legacy (Django PBKDF2) → à ré-écrire en argon2.
    ValidNeedsRehash,
}

pub trait PasswordHasher: Send + Sync {
    fn verify(&self, password: &str, stored: &str) -> Result<PasswordCheck, AppError>;
    fn hash(&self, password: &str) -> Result<String, AppError>;
}

/// Ligne de fil d'actualité (read model dénormalisé pour la présentation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedItem {
    pub id: i64,
    pub author_username: String,
    pub author_display: String,
    pub content: String,
    pub likes_count: i64,
    pub reposts_count: i64,
    pub replies_count: i64,
    pub created_at: String,
}

#[async_trait]
pub trait FeedRepository: Send + Sync {
    /// Posts récents (hors réponses), joints à leur auteur, du plus récent au plus ancien.
    async fn recent(&self, limit: u64) -> Result<Vec<FeedItem>, RepoError>;
}

// ---------------------------------------------------------------------------
// Use cases — orchestrent domaine + ports. Zéro dépendance framework.
// ---------------------------------------------------------------------------

/// Crée un post : valide le contenu (invariant domaine) puis persiste.
pub struct CreatePost<'a, R: PostRepository + ?Sized> {
    repo: &'a R,
}

impl<'a, R: PostRepository + ?Sized> CreatePost<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        author: UserId,
        raw_content: &str,
        parent: Option<PostId>,
    ) -> Result<Post, AppError> {
        let content = PostContent::new(raw_content)?; // invariant ≤280, non vide
        let post = self.repo.insert(author, &content, parent).await?;
        Ok(post)
    }
}

/// Fait suivre un utilisateur par un autre (idempotent, refuse l'auto-follow).
pub struct FollowUser<'a, R: FollowRepository + ?Sized> {
    repo: &'a R,
}

impl<'a, R: FollowRepository + ?Sized> FollowUser<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    /// Retourne `true` si une nouvelle relation a été créée, `false` si elle existait déjà.
    pub async fn execute(&self, follower: UserId, following: UserId) -> Result<bool, AppError> {
        let follow = Follow::new(follower, following)?; // invariant : pas d'auto-follow
        if self.repo.exists(follower, following).await? {
            return Ok(false);
        }
        let created = self.repo.add(&follow).await?;
        Ok(created)
    }
}

/// Like / unlike un post. La cohérence du compteur est garantie par le repo.
pub struct ToggleLike<'a, R: LikeRepository + ?Sized> {
    repo: &'a R,
}

impl<'a, R: LikeRepository + ?Sized> ToggleLike<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub async fn like(&self, user: UserId, post: PostId) -> Result<LikeToggle, AppError> {
        Ok(self.repo.like(user, post).await?)
    }

    pub async fn unlike(&self, user: UserId, post: PostId) -> Result<LikeToggle, AppError> {
        Ok(self.repo.unlike(user, post).await?)
    }
}

/// Authentifie un utilisateur (auth **hybride** : vérif du hash, puis re-hash
/// argon2 transparent si le hash stocké est un legacy Django PBKDF2).
pub struct Login<'a, U: UserRepository + ?Sized, H: PasswordHasher + ?Sized> {
    users: &'a U,
    hasher: &'a H,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginOutcome {
    pub user_id: i64,
    pub username: String,
}

impl<'a, U: UserRepository + ?Sized, H: PasswordHasher + ?Sized> Login<'a, U, H> {
    pub fn new(users: &'a U, hasher: &'a H) -> Self {
        Self { users, hasher }
    }

    /// `Ok(None)` = identifiants invalides (utilisateur inconnu ou mauvais mot de passe).
    pub async fn execute(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<LoginOutcome>, AppError> {
        let Some(record) = self.users.by_username(username).await? else {
            return Ok(None);
        };

        match self.hasher.verify(password, &record.password_hash)? {
            PasswordCheck::Invalid => Ok(None),
            PasswordCheck::Valid => Ok(Some(LoginOutcome {
                user_id: record.id,
                username: record.username,
            })),
            PasswordCheck::ValidNeedsRehash => {
                // Modernisation transparente : on ré-écrit en argon2. Best-effort :
                // un échec de re-hash ne doit pas empêcher la connexion.
                if let Ok(new_hash) = self.hasher.hash(password) {
                    let _ = self.users.update_password(record.id, &new_hash).await;
                }
                Ok(Some(LoginOutcome {
                    user_id: record.id,
                    username: record.username,
                }))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests : use cases avec repos mockés en mémoire (aucune BDD).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryPosts {
        rows: Mutex<Vec<Post>>,
    }

    #[async_trait]
    impl PostRepository for InMemoryPosts {
        async fn insert(
            &self,
            author: UserId,
            content: &PostContent,
            parent: Option<PostId>,
        ) -> Result<Post, RepoError> {
            let mut rows = self.rows.lock().unwrap();
            let post = Post {
                id: PostId(rows.len() as i64 + 1),
                author,
                content: content.clone(),
                parent,
                sport: None,
                match_id: None,
                team: None,
            };
            rows.push(post.clone());
            Ok(post)
        }
    }

    #[derive(Default)]
    struct InMemoryFollows {
        rows: Mutex<Vec<Follow>>,
    }

    #[async_trait]
    impl FollowRepository for InMemoryFollows {
        async fn add(&self, follow: &Follow) -> Result<bool, RepoError> {
            self.rows.lock().unwrap().push(*follow);
            Ok(true)
        }
        async fn exists(&self, follower: UserId, following: UserId) -> Result<bool, RepoError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .any(|f| f.follower == follower && f.following == following))
        }
    }

    #[tokio::test]
    async fn create_post_persists_valid_content() {
        let repo = InMemoryPosts::default();
        let uc = CreatePost::new(&repo);
        let post = uc.execute(UserId(1), "  Great match! #Football ", None).await.unwrap();
        assert_eq!(post.content.as_str(), "Great match! #Football");
        assert_eq!(post.id, PostId(1));
    }

    #[tokio::test]
    async fn create_post_rejects_too_long() {
        let repo = InMemoryPosts::default();
        let uc = CreatePost::new(&repo);
        let err = uc.execute(UserId(1), &"a".repeat(281), None).await.unwrap_err();
        assert_eq!(err, AppError::Domain(DomainError::InvalidPostContent));
        assert!(repo.rows.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn follow_is_idempotent_and_refuses_self() {
        let repo = InMemoryFollows::default();
        let uc = FollowUser::new(&repo);

        assert!(uc.execute(UserId(1), UserId(2)).await.unwrap()); // created
        assert!(!uc.execute(UserId(1), UserId(2)).await.unwrap()); // already exists

        let err = uc.execute(UserId(1), UserId(1)).await.unwrap_err();
        assert_eq!(err, AppError::Domain(DomainError::SelfFollow));
    }

    #[derive(Default)]
    struct InMemoryLikes {
        likes: Mutex<Vec<(UserId, PostId)>>,
    }

    #[async_trait]
    impl LikeRepository for InMemoryLikes {
        async fn like(&self, user: UserId, post: PostId) -> Result<LikeToggle, RepoError> {
            let mut likes = self.likes.lock().unwrap();
            let changed = if likes.contains(&(user, post)) {
                false
            } else {
                likes.push((user, post));
                true
            };
            let count = likes.iter().filter(|(_, p)| *p == post).count() as i64;
            Ok(LikeToggle { changed, likes_count: count })
        }
        async fn unlike(&self, user: UserId, post: PostId) -> Result<LikeToggle, RepoError> {
            let mut likes = self.likes.lock().unwrap();
            let before = likes.len();
            likes.retain(|pair| *pair != (user, post));
            let changed = likes.len() != before;
            let count = likes.iter().filter(|(_, p)| *p == post).count() as i64;
            Ok(LikeToggle { changed, likes_count: count })
        }
    }

    #[tokio::test]
    async fn toggle_like_is_idempotent_and_counts() {
        let repo = InMemoryLikes::default();
        let uc = ToggleLike::new(&repo);
        let post = PostId(7);

        let r = uc.like(UserId(1), post).await.unwrap();
        assert_eq!(r, LikeToggle { changed: true, likes_count: 1 });
        // re-like : pas de changement, compteur stable
        let r = uc.like(UserId(1), post).await.unwrap();
        assert_eq!(r, LikeToggle { changed: false, likes_count: 1 });
        // autre user
        let r = uc.like(UserId(2), post).await.unwrap();
        assert_eq!(r, LikeToggle { changed: true, likes_count: 2 });
        // unlike
        let r = uc.unlike(UserId(1), post).await.unwrap();
        assert_eq!(r, LikeToggle { changed: true, likes_count: 1 });
    }

    struct InMemoryUsers {
        record: Option<AuthUserRecord>,
        updated_to: Mutex<Option<String>>,
    }

    #[async_trait]
    impl UserRepository for InMemoryUsers {
        async fn by_username(&self, username: &str) -> Result<Option<AuthUserRecord>, RepoError> {
            Ok(self
                .record
                .clone()
                .filter(|r| r.username == username))
        }
        async fn update_password(&self, _user_id: i64, new_hash: &str) -> Result<(), RepoError> {
            *self.updated_to.lock().unwrap() = Some(new_hash.to_owned());
            Ok(())
        }
    }

    /// Hasher factice : le "hash stocké" encode directement l'issue attendue.
    struct FakeHasher;
    impl PasswordHasher for FakeHasher {
        fn verify(&self, _password: &str, stored: &str) -> Result<PasswordCheck, AppError> {
            Ok(match stored {
                "VALID" => PasswordCheck::Valid,
                "LEGACY" => PasswordCheck::ValidNeedsRehash,
                _ => PasswordCheck::Invalid,
            })
        }
        fn hash(&self, _password: &str) -> Result<String, AppError> {
            Ok("$argon2id$rehashed".to_owned())
        }
    }

    fn user(hash: &str) -> InMemoryUsers {
        InMemoryUsers {
            record: Some(AuthUserRecord { id: 1, username: "messi".into(), password_hash: hash.into() }),
            updated_to: Mutex::new(None),
        }
    }

    #[tokio::test]
    async fn login_unknown_user_returns_none() {
        let users = InMemoryUsers { record: None, updated_to: Mutex::new(None) };
        let hasher = FakeHasher;
        let out = Login::new(&users, &hasher).execute("ghost", "x").await.unwrap();
        assert_eq!(out, None);
    }

    #[tokio::test]
    async fn login_wrong_password_returns_none() {
        let users = user("VALID");
        let hasher = FakeHasher;
        let out = Login::new(&users, &hasher).execute("messi", "wrong").await.unwrap();
        // stored "VALID" always verifies here, so simulate wrong via "INVALID" hash instead:
        assert!(out.is_some());
        let users = user("INVALID");
        let out = Login::new(&users, &hasher).execute("messi", "whatever").await.unwrap();
        assert_eq!(out, None);
    }

    #[tokio::test]
    async fn login_argon2_ok_without_rehash() {
        let users = user("VALID");
        let hasher = FakeHasher;
        let out = Login::new(&users, &hasher).execute("messi", "grind1234").await.unwrap();
        assert_eq!(out.unwrap().user_id, 1);
        assert!(users.updated_to.lock().unwrap().is_none(), "pas de re-hash attendu");
    }

    #[tokio::test]
    async fn login_legacy_hash_triggers_rehash() {
        let users = user("LEGACY");
        let hasher = FakeHasher;
        let out = Login::new(&users, &hasher).execute("messi", "grind1234").await.unwrap();
        assert_eq!(out.unwrap().username, "messi");
        assert_eq!(
            users.updated_to.lock().unwrap().as_deref(),
            Some("$argon2id$rehashed"),
            "le hash legacy doit être ré-écrit en argon2"
        );
    }
}
