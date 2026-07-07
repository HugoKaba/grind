//! Implémentations SeaORM des ports de `grind-application`.
//! Les compteurs dénormalisés sont mis à jour **atomiquement** en base
//! (`SET x = x + 1` dans une transaction), jamais par read-modify-write applicatif.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};

use grind_application::{FollowRepository, LikeRepository, LikeToggle, PostRepository, RepoError};
use grind_domain::entities::{Follow, MatchId, Post, PostId, SportId, TeamId, UserId};
use grind_domain::value_objects::PostContent;

use super::entities::{follow, post, post_like};

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
}
