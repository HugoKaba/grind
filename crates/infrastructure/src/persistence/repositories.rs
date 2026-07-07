//! Implémentations SeaORM des ports de `grind-application`.
//! Les compteurs dénormalisés sont mis à jour **atomiquement** en base
//! (`SET x = x + 1` dans une transaction), jamais par read-modify-write applicatif.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use grind_application::{
    AuthUserRecord, FeedItem, FeedRepository, FollowRepository, LikeRepository, LikeToggle,
    PostRepository, RepoError, UserRepository,
};
use grind_domain::entities::{Follow, MatchId, Post, PostId, SportId, TeamId, UserId};
use grind_domain::value_objects::PostContent;

use super::entities::{follow, post, post_like, users};

fn db_err(e: DbErr) -> RepoError {
    RepoError::Backend(e.to_string())
}

fn to_domain_post(m: post::Model) -> Result<Post, RepoError> {
    Ok(Post {
        id: PostId(m.id),
        author: UserId(m.author_id),
        content: PostContent::new(m.content)
            .map_err(|_| RepoError::Backend("stored post content violates invariant".into()))?,
        parent: m.parent_id.map(PostId),
        sport: m.sport_id.map(SportId),
        match_id: m.match_id.map(MatchId),
        team: m.team_id.map(TeamId),
    })
}

async fn likes_count<C: ConnectionTrait>(conn: &C, post_id: i64) -> Result<i64, RepoError> {
    let model = post::Entity::find_by_id(post_id)
        .one(conn)
        .await
        .map_err(db_err)?
        .ok_or(RepoError::NotFound)?;
    Ok(model.likes_count)
}

// ---------------------------------------------------------------------------

pub struct SeaOrmPostRepository {
    db: DatabaseConnection,
}

impl SeaOrmPostRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PostRepository for SeaOrmPostRepository {
    async fn insert(
        &self,
        author: UserId,
        content: &PostContent,
        parent: Option<PostId>,
    ) -> Result<Post, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let model = post::ActiveModel {
            author_id: Set(author.0),
            content: Set(content.as_str().to_owned()),
            parent_id: Set(parent.map(|p| p.0)),
            sport_id: Set(None),
            match_id: Set(None),
            team_id: Set(None),
            likes_count: Set(0),
            reposts_count: Set(0),
            replies_count: Set(0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(db_err)?;

        // Réponse → incrément atomique du compteur du parent.
        if let Some(parent_id) = parent {
            post::Entity::update_many()
                .col_expr(
                    post::Column::RepliesCount,
                    Expr::col(post::Column::RepliesCount).add(1),
                )
                .filter(post::Column::Id.eq(parent_id.0))
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        txn.commit().await.map_err(db_err)?;
        to_domain_post(model)
    }

    async fn delete(&self, post: PostId) -> Result<bool, RepoError> {
        let res = post::Entity::delete_by_id(post.0)
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected > 0)
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmFollowRepository {
    db: DatabaseConnection,
}

impl SeaOrmFollowRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl FollowRepository for SeaOrmFollowRepository {
    async fn add(&self, f: &Follow) -> Result<bool, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let already = follow::Entity::find()
            .filter(follow::Column::FollowerId.eq(f.follower.0))
            .filter(follow::Column::FollowingId.eq(f.following.0))
            .one(&txn)
            .await
            .map_err(db_err)?;

        if already.is_some() {
            txn.commit().await.map_err(db_err)?;
            return Ok(false);
        }

        follow::ActiveModel {
            follower_id: Set(f.follower.0),
            following_id: Set(f.following.0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(db_err)?;

        txn.commit().await.map_err(db_err)?;
        Ok(true)
    }

    async fn exists(&self, follower: UserId, following: UserId) -> Result<bool, RepoError> {
        let found = follow::Entity::find()
            .filter(follow::Column::FollowerId.eq(follower.0))
            .filter(follow::Column::FollowingId.eq(following.0))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.is_some())
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmLikeRepository {
    db: DatabaseConnection,
}

impl SeaOrmLikeRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LikeRepository for SeaOrmLikeRepository {
    async fn like(&self, user: UserId, post_id: PostId) -> Result<LikeToggle, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let existing = post_like::Entity::find()
            .filter(post_like::Column::UserId.eq(user.0))
            .filter(post_like::Column::PostId.eq(post_id.0))
            .one(&txn)
            .await
            .map_err(db_err)?;

        if existing.is_some() {
            let count = likes_count(&txn, post_id.0).await?;
            txn.commit().await.map_err(db_err)?;
            return Ok(LikeToggle { changed: false, likes_count: count });
        }

        post_like::ActiveModel {
            user_id: Set(user.0),
            post_id: Set(post_id.0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(db_err)?;

        post::Entity::update_many()
            .col_expr(post::Column::LikesCount, Expr::col(post::Column::LikesCount).add(1))
            .filter(post::Column::Id.eq(post_id.0))
            .exec(&txn)
            .await
            .map_err(db_err)?;

        let count = likes_count(&txn, post_id.0).await?;
        txn.commit().await.map_err(db_err)?;
        Ok(LikeToggle { changed: true, likes_count: count })
    }

    async fn unlike(&self, user: UserId, post_id: PostId) -> Result<LikeToggle, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let deleted = post_like::Entity::delete_many()
            .filter(post_like::Column::UserId.eq(user.0))
            .filter(post_like::Column::PostId.eq(post_id.0))
            .exec(&txn)
            .await
            .map_err(db_err)?;

        let changed = deleted.rows_affected > 0;
        if changed {
            post::Entity::update_many()
                .col_expr(post::Column::LikesCount, Expr::col(post::Column::LikesCount).sub(1))
                .filter(post::Column::Id.eq(post_id.0))
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        let count = likes_count(&txn, post_id.0).await?;
        txn.commit().await.map_err(db_err)?;
        Ok(LikeToggle { changed, likes_count: count })
    }

    async fn has_liked(&self, user: UserId, post_id: PostId) -> Result<bool, RepoError> {
        let found = post_like::Entity::find()
            .filter(post_like::Column::UserId.eq(user.0))
            .filter(post_like::Column::PostId.eq(post_id.0))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.is_some())
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmUserRepository {
    db: DatabaseConnection,
}

impl SeaOrmUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn by_username(&self, username: &str) -> Result<Option<AuthUserRecord>, RepoError> {
        let found = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.map(|u| AuthUserRecord {
            id: u.id,
            username: u.username,
            password_hash: u.password,
            is_staff: u.is_staff,
        }))
    }

    async fn update_password(&self, user_id: i64, new_hash: &str) -> Result<(), RepoError> {
        users::Entity::update_many()
            .col_expr(users::Column::Password, Expr::value(new_hash.to_owned()))
            .filter(users::Column::Id.eq(user_id))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        display_name: &str,
    ) -> Result<AuthUserRecord, RepoError> {
        let res = users::ActiveModel {
            username: Set(username.to_owned()),
            password: Set(password_hash.to_owned()),
            display_name: Set(display_name.to_owned()),
            is_staff: Set(false),
            ..Default::default()
        }
        .insert(&self.db)
        .await;

        match res {
            Ok(u) => Ok(AuthUserRecord {
                id: u.id,
                username: u.username,
                password_hash: u.password,
                is_staff: u.is_staff,
            }),
            Err(e) => {
                // Violation de la contrainte d'unicité (username déjà pris) →
                // Conflict. Détection portable SQLite/Postgres par le message.
                let msg = e.to_string().to_lowercase();
                if msg.contains("unique") || msg.contains("duplicate") {
                    Err(RepoError::Conflict(format!("username '{username}' déjà pris")))
                } else {
                    Err(db_err(e))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmFeedRepository {
    db: DatabaseConnection,
}

impl SeaOrmFeedRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// Set des `post_id` likés par `viewer` parmi `post_ids`, en **une** requête.
/// Vide si `viewer` est anonyme (`None`). Sert à renseigner `liked_by_me` sans N+1.
async fn liked_set(
    db: &DatabaseConnection,
    viewer: Option<UserId>,
    post_ids: &[i64],
) -> Result<std::collections::HashSet<i64>, RepoError> {
    let Some(viewer) = viewer else {
        return Ok(std::collections::HashSet::new());
    };
    let rows = post_like::Entity::find()
        .filter(post_like::Column::UserId.eq(viewer.0))
        .filter(post_like::Column::PostId.is_in(post_ids.to_vec()))
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(rows.into_iter().map(|r| r.post_id).collect())
}

/// Joint une liste de posts à leurs auteurs **et** à l'état de like de l'observateur,
/// en **deux** requêtes bornées (auteurs + likes) — pas de N+1.
async fn join_authors(
    db: &DatabaseConnection,
    viewer: Option<UserId>,
    posts: Vec<post::Model>,
) -> Result<Vec<FeedItem>, RepoError> {
    if posts.is_empty() {
        return Ok(Vec::new());
    }
    let author_ids: Vec<i64> = posts.iter().map(|p| p.author_id).collect();
    let post_ids: Vec<i64> = posts.iter().map(|p| p.id).collect();
    let authors = users::Entity::find()
        .filter(users::Column::Id.is_in(author_ids))
        .all(db)
        .await
        .map_err(db_err)?;
    let liked = liked_set(db, viewer, &post_ids).await?;

    Ok(posts
        .into_iter()
        .map(|p| {
            let author = authors.iter().find(|u| u.id == p.author_id);
            FeedItem {
                id: p.id,
                author_username: author.map(|u| u.username.clone()).unwrap_or_default(),
                author_display: author.map(|u| u.display_name.clone()).unwrap_or_default(),
                content: p.content,
                likes_count: p.likes_count,
                reposts_count: p.reposts_count,
                replies_count: p.replies_count,
                created_at: p.created_at.to_rfc3339(),
                liked_by_me: liked.contains(&p.id),
            }
        })
        .collect())
}

#[async_trait]
impl FeedRepository for SeaOrmFeedRepository {
    async fn recent(&self, viewer: Option<UserId>, limit: u64) -> Result<Vec<FeedItem>, RepoError> {
        let posts = post::Entity::find()
            .filter(post::Column::ParentId.is_null())
            .order_by_desc(post::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        join_authors(&self.db, viewer, posts).await
    }

    async fn by_id(&self, viewer: Option<UserId>, id: i64) -> Result<Option<FeedItem>, RepoError> {
        let Some(p) = post::Entity::find_by_id(id).one(&self.db).await.map_err(db_err)? else {
            return Ok(None);
        };
        Ok(join_authors(&self.db, viewer, vec![p]).await?.into_iter().next())
    }

    async fn replies(
        &self,
        viewer: Option<UserId>,
        parent_id: i64,
        limit: u64,
    ) -> Result<Vec<FeedItem>, RepoError> {
        let posts = post::Entity::find()
            .filter(post::Column::ParentId.eq(parent_id))
            .order_by_asc(post::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        join_authors(&self.db, viewer, posts).await
    }

    async fn by_author(
        &self,
        viewer: Option<UserId>,
        username: &str,
        limit: u64,
    ) -> Result<Vec<FeedItem>, RepoError> {
        let Some(user) = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(db_err)?
        else {
            return Ok(Vec::new());
        };
        let posts = post::Entity::find()
            .filter(post::Column::AuthorId.eq(user.id))
            .order_by_desc(post::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        join_authors(&self.db, viewer, posts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::connect_and_migrate;
    use crate::persistence::seed::seed_reference_and_athletes;
    use crate::security::PasswordService;

    #[tokio::test]
    async fn full_slice_seed_post_reply_like_follow() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        let hasher = PasswordService::new();

        // Backfill : sports + équipes + athlètes (messi=1, ronaldo=2).
        let report = seed_reference_and_athletes(&db, &hasher).await.unwrap();
        assert_eq!(report.athletes, 2);

        let posts = SeaOrmPostRepository::new(db.clone());
        let content = PostContent::new("Hat-trick tonight! #Goals").unwrap();
        let p = posts.insert(UserId(1), &content, None).await.unwrap();
        assert_eq!(p.author, UserId(1));
        assert!(!p.is_reply());

        // Réponse → replies_count du parent incrémenté atomiquement.
        let reply = PostContent::new("Congrats champ!").unwrap();
        let r = posts.insert(UserId(2), &reply, Some(p.id)).await.unwrap();
        assert!(r.is_reply());
        let parent = post::Entity::find_by_id(p.id.0).one(&db).await.unwrap().unwrap();
        assert_eq!(parent.replies_count, 1);

        // Like : compteur atomique + idempotence.
        let likes = SeaOrmLikeRepository::new(db.clone());
        assert_eq!(likes.like(UserId(2), p.id).await.unwrap().likes_count, 1);
        let again = likes.like(UserId(2), p.id).await.unwrap();
        assert!(!again.changed);
        assert_eq!(again.likes_count, 1);
        assert_eq!(likes.unlike(UserId(2), p.id).await.unwrap().likes_count, 0);

        // Follow : idempotent + exists.
        let follows = SeaOrmFollowRepository::new(db.clone());
        let rel = Follow::new(UserId(1), UserId(2)).unwrap();
        assert!(follows.add(&rel).await.unwrap());
        assert!(!follows.add(&rel).await.unwrap());
        assert!(follows.exists(UserId(1), UserId(2)).await.unwrap());
    }

    #[tokio::test]
    async fn feed_liked_by_me_is_viewer_aware() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_reference_and_athletes(&db, &PasswordService::new()).await.unwrap();

        let posts = SeaOrmPostRepository::new(db.clone());
        let p = posts
            .insert(UserId(1), &PostContent::new("Golazo!").unwrap(), None)
            .await
            .unwrap();

        // ronaldo (2) like le post ; messi (1) non.
        let likes = SeaOrmLikeRepository::new(db.clone());
        likes.like(UserId(2), p.id).await.unwrap();

        let feed = SeaOrmFeedRepository::new(db.clone());

        // Vu par ronaldo → liked_by_me = true.
        let seen = feed.recent(Some(UserId(2)), 10).await.unwrap();
        assert!(seen.iter().find(|i| i.id == p.id.0).unwrap().liked_by_me);

        // Vu par messi → liked_by_me = false (il n'a pas liké).
        let seen = feed.recent(Some(UserId(1)), 10).await.unwrap();
        assert!(!seen.iter().find(|i| i.id == p.id.0).unwrap().liked_by_me);

        // Vu par un anonyme → liked_by_me = false.
        let seen = feed.recent(None, 10).await.unwrap();
        assert!(!seen.iter().find(|i| i.id == p.id.0).unwrap().liked_by_me);
    }

    #[tokio::test]
    async fn create_user_enforces_unique_username() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        let users = SeaOrmUserRepository::new(db);

        let rec = users.create("newpro", "$argon2id$hash", "New Pro").await.unwrap();
        assert!(!rec.is_staff);
        assert_eq!(rec.username, "newpro");

        // by_username retrouve le compte fraîchement créé.
        let found = users.by_username("newpro").await.unwrap().unwrap();
        assert_eq!(found.id, rec.id);

        // Doublon → Conflict (contrainte d'unicité).
        let err = users.create("newpro", "$argon2id$other", "Dup").await.unwrap_err();
        assert!(matches!(err, RepoError::Conflict(_)), "attendu Conflict, reçu: {err:?}");
    }
}
