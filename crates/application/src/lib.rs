//! # grind-application
//!
//! Use cases + **ports** (traits). Dépend uniquement du domaine.
//! L'infrastructure (SeaORM, Redis…) implémentera ces ports → inversion de dépendance.

use async_trait::async_trait;
use grind_domain::entities::{Follow, Post, PostId, UserId};
use grind_domain::value_objects::{PostContent, Username};
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

    /// Supprime un post (modération admin). `true` si une ligne a été supprimée.
    async fn delete(&self, post: PostId) -> Result<bool, RepoError>;
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
    /// `true` si `user` a déjà liké `post` (read model pour décider du toggle).
    async fn has_liked(&self, user: UserId, post: PostId) -> Result<bool, RepoError>;
}

/// Enregistrement minimal pour l'authentification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUserRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub is_staff: bool,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn by_username(&self, username: &str) -> Result<Option<AuthUserRecord>, RepoError>;
    async fn update_password(&self, user_id: i64, new_hash: &str) -> Result<(), RepoError>;
    /// Crée un utilisateur (non-staff). L'unicité du `username` est garantie par
    /// le backend (contrainte) → `RepoError::Conflict` si déjà pris.
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        display_name: &str,
    ) -> Result<AuthUserRecord, RepoError>;
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
    /// `true` si l'observateur (`viewer`) a liké ce post. `false` si anonyme.
    pub liked_by_me: bool,
}

#[async_trait]
pub trait FeedRepository: Send + Sync {
    /// Posts récents (hors réponses), joints à leur auteur, du plus récent au plus ancien.
    /// `viewer` = observateur courant (`None` si anonyme) → renseigne `liked_by_me`.
    async fn recent(&self, viewer: Option<UserId>, limit: u64) -> Result<Vec<FeedItem>, RepoError>;
    /// Un post par id (page détail).
    async fn by_id(&self, viewer: Option<UserId>, id: i64) -> Result<Option<FeedItem>, RepoError>;
    /// Réponses à un post (thread).
    async fn replies(
        &self,
        viewer: Option<UserId>,
        parent_id: i64,
        limit: u64,
    ) -> Result<Vec<FeedItem>, RepoError>;
    /// Posts d'un auteur (page profil).
    async fn by_author(
        &self,
        viewer: Option<UserId>,
        username: &str,
        limit: u64,
    ) -> Result<Vec<FeedItem>, RepoError>;
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

/// État d'un post vis-à-vis d'un utilisateur après (dé)like : renvoyé au client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LikeState {
    /// `true` si l'utilisateur like désormais le post.
    pub liked: bool,
    /// Nombre total de likes du post après opération.
    pub likes_count: i64,
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

    /// Bascule l'état de like selon l'état courant (orchestration = ici, pas
    /// dans la server function). Like si absent, unlike sinon.
    pub async fn toggle(&self, user: UserId, post: PostId) -> Result<LikeState, AppError> {
        if self.repo.has_liked(user, post).await? {
            let t = self.repo.unlike(user, post).await?;
            Ok(LikeState { liked: false, likes_count: t.likes_count })
        } else {
            let t = self.repo.like(user, post).await?;
            Ok(LikeState { liked: true, likes_count: t.likes_count })
        }
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
    pub is_staff: bool,
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

        let outcome = LoginOutcome {
            user_id: record.id,
            username: record.username.clone(),
            is_staff: record.is_staff,
        };

        match self.hasher.verify(password, &record.password_hash)? {
            PasswordCheck::Invalid => Ok(None),
            PasswordCheck::Valid => Ok(Some(outcome)),
            PasswordCheck::ValidNeedsRehash => {
                // Modernisation transparente : on ré-écrit en argon2. Best-effort :
                // un échec de re-hash ne doit pas empêcher la connexion.
                if let Ok(new_hash) = self.hasher.hash(password) {
                    let _ = self.users.update_password(record.id, &new_hash).await;
                }
                Ok(Some(outcome))
            }
        }
    }
}

/// Crée un compte. Invariant **domaine** : le username est validé par `Username`.
/// Politique **application** : longueur minimale du mot de passe. Unicité :
/// déléguée au repo (contrainte backend) → `Conflict` propre si déjà pris.
pub struct Register<'a, U: UserRepository + ?Sized, H: PasswordHasher + ?Sized> {
    users: &'a U,
    hasher: &'a H,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterOutcome {
    pub user_id: i64,
    pub username: String,
    pub is_staff: bool,
}

impl<'a, U: UserRepository + ?Sized, H: PasswordHasher + ?Sized> Register<'a, U, H> {
    /// Longueur minimale d'un mot de passe (politique applicative, pas domaine).
    pub const MIN_PASSWORD_LEN: usize = 8;

    pub fn new(users: &'a U, hasher: &'a H) -> Self {
        Self { users, hasher }
    }

    pub async fn execute(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> Result<RegisterOutcome, AppError> {
        // 1. Invariant domaine : format du username ([a-z0-9_], 1..=30).
        let username = Username::new(username)?;
        // 2. Politique applicative : mot de passe assez long.
        if password.chars().count() < Self::MIN_PASSWORD_LEN {
            return Err(AppError::Auth(format!(
                "mot de passe trop court (min {})",
                Self::MIN_PASSWORD_LEN
            )));
        }
        // 3. Hachage argon2 (jamais le mot de passe en clair au repo).
        let hash = self.hasher.hash(password)?;
        // 4. Nom affiché : le username par défaut si vide.
        let display = {
            let d = display_name.trim();
            if d.is_empty() { username.as_str() } else { d }
        };
        // 5. Création (unicité = contrainte backend → Conflict si déjà pris).
        let rec = self.users.create(username.as_str(), &hash, display).await?;
        Ok(RegisterOutcome {
            user_id: rec.id,
            username: rec.username,
            is_staff: rec.is_staff,
        })
    }
}

/// Supprime un post (modération). L'autorisation (staff) est vérifiée en amont
/// par l'extractor `AdminUser` de la couche présentation.
pub struct DeletePost<'a, R: PostRepository + ?Sized> {
    repo: &'a R,
}

impl<'a, R: PostRepository + ?Sized> DeletePost<'a, R> {
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post: PostId) -> Result<bool, AppError> {
        Ok(self.repo.delete(post).await?)
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
        async fn delete(&self, post: PostId) -> Result<bool, RepoError> {
            let mut rows = self.rows.lock().unwrap();
            let before = rows.len();
            rows.retain(|p| p.id != post);
            Ok(rows.len() != before)
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
        async fn has_liked(&self, user: UserId, post: PostId) -> Result<bool, RepoError> {
            Ok(self.likes.lock().unwrap().contains(&(user, post)))
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

    #[tokio::test]
    async fn toggle_like_flips_state_from_current() {
        let repo = InMemoryLikes::default();
        let uc = ToggleLike::new(&repo);
        let post = PostId(3);

        // pas encore liké → toggle = like
        let s = uc.toggle(UserId(1), post).await.unwrap();
        assert_eq!(s, LikeState { liked: true, likes_count: 1 });
        // déjà liké → toggle = unlike
        let s = uc.toggle(UserId(1), post).await.unwrap();
        assert_eq!(s, LikeState { liked: false, likes_count: 0 });
    }

    #[derive(Default)]
    struct InMemoryUsers {
        record: Option<AuthUserRecord>,
        created: Mutex<Vec<AuthUserRecord>>,
        updated_to: Mutex<Option<String>>,
    }

    #[async_trait]
    impl UserRepository for InMemoryUsers {
        async fn by_username(&self, username: &str) -> Result<Option<AuthUserRecord>, RepoError> {
            if let Some(r) = self.record.clone().filter(|r| r.username == username) {
                return Ok(Some(r));
            }
            Ok(self
                .created
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.username == username)
                .cloned())
        }
        async fn update_password(&self, _user_id: i64, new_hash: &str) -> Result<(), RepoError> {
            *self.updated_to.lock().unwrap() = Some(new_hash.to_owned());
            Ok(())
        }
        async fn create(
            &self,
            username: &str,
            password_hash: &str,
            display_name: &str,
        ) -> Result<AuthUserRecord, RepoError> {
            let taken = self.record.as_ref().is_some_and(|r| r.username == username);
            let mut created = self.created.lock().unwrap();
            if taken || created.iter().any(|r| r.username == username) {
                return Err(RepoError::Conflict(format!("username '{username}' déjà pris")));
            }
            let rec = AuthUserRecord {
                id: 100 + created.len() as i64,
                username: username.to_owned(),
                password_hash: password_hash.to_owned(),
                is_staff: false,
            };
            let _ = display_name; // le mock ne stocke pas le display name
            created.push(rec.clone());
            Ok(rec)
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
            record: Some(AuthUserRecord {
                id: 1,
                username: "messi".into(),
                password_hash: hash.into(),
                is_staff: true,
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn login_unknown_user_returns_none() {
        let users = InMemoryUsers::default();
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

    #[tokio::test]
    async fn register_creates_non_staff_user_with_hashed_password() {
        let users = InMemoryUsers::default();
        let hasher = FakeHasher;
        let out = Register::new(&users, &hasher)
            .execute("newpro", "grind1234", "New Pro")
            .await
            .unwrap();
        assert_eq!(out.username, "newpro");
        assert!(!out.is_staff, "un compte auto-créé n'est jamais staff");

        // Le mot de passe stocké est le hash, pas le clair.
        let stored = users.by_username("newpro").await.unwrap().unwrap();
        assert_eq!(stored.password_hash, "$argon2id$rehashed");
    }

    #[tokio::test]
    async fn register_rejects_duplicate_username() {
        let users = user("VALID"); // "messi" existe déjà
        let hasher = FakeHasher;
        let err = Register::new(&users, &hasher)
            .execute("messi", "grind1234", "")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Repo(RepoError::Conflict(_))));
    }

    #[tokio::test]
    async fn register_rejects_invalid_username() {
        let users = InMemoryUsers::default();
        let hasher = FakeHasher;
        // Majuscule interdite → invariant domaine.
        let err = Register::new(&users, &hasher)
            .execute("Messi", "grind1234", "")
            .await
            .unwrap_err();
        assert_eq!(err, AppError::Domain(DomainError::InvalidUsername));
    }

    #[tokio::test]
    async fn register_rejects_short_password() {
        let users = InMemoryUsers::default();
        let hasher = FakeHasher;
        let err = Register::new(&users, &hasher)
            .execute("newpro", "short", "")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Auth(_)));
        // Rien n'a été créé.
        assert!(users.by_username("newpro").await.unwrap().is_none());
    }
}
