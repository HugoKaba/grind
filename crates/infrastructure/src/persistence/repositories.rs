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
    AuthUserRecord, BookmarkRepository, FeedItem, FeedRepository, FollowRepository, LikeRepository,
    LikeToggle, MatchRepository, MatchRow, PostRepository, RepoError, RepostRepository,
    RepostToggle, SportCatalog, SportRow, TeamFollowRepository, TeamRow, UserRepository,
};
use grind_domain::entities::{Follow, MatchId, Post, PostId, SportId, TeamId, UserId};
use grind_domain::value_objects::PostContent;

use super::entities::{
    bookmark, follow, match_event, post, post_like, repost, sport, team, team_follow, users,
};

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

    async fn insert_about_match(
        &self,
        author: UserId,
        content: &PostContent,
        sport: SportId,
        match_id: MatchId,
    ) -> Result<Post, RepoError> {
        let model = post::ActiveModel {
            author_id: Set(author.0),
            content: Set(content.as_str().to_owned()),
            parent_id: Set(None),
            sport_id: Set(Some(sport.0)),
            match_id: Set(Some(match_id.0)),
            team_id: Set(None),
            likes_count: Set(0),
            reposts_count: Set(0),
            replies_count: Set(0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(db_err)?;
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

    async fn remove(&self, follower: UserId, following: UserId) -> Result<bool, RepoError> {
        let res = follow::Entity::delete_many()
            .filter(follow::Column::FollowerId.eq(follower.0))
            .filter(follow::Column::FollowingId.eq(following.0))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected > 0)
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

async fn reposts_count<C: ConnectionTrait>(conn: &C, post_id: i64) -> Result<i64, RepoError> {
    let model = post::Entity::find_by_id(post_id)
        .one(conn)
        .await
        .map_err(db_err)?
        .ok_or(RepoError::NotFound)?;
    Ok(model.reposts_count)
}

pub struct SeaOrmRepostRepository {
    db: DatabaseConnection,
}

impl SeaOrmRepostRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RepostRepository for SeaOrmRepostRepository {
    async fn repost(&self, user: UserId, post_id: PostId) -> Result<RepostToggle, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let existing = repost::Entity::find()
            .filter(repost::Column::UserId.eq(user.0))
            .filter(repost::Column::PostId.eq(post_id.0))
            .one(&txn)
            .await
            .map_err(db_err)?;

        if existing.is_some() {
            let count = reposts_count(&txn, post_id.0).await?;
            txn.commit().await.map_err(db_err)?;
            return Ok(RepostToggle { changed: false, reposts_count: count });
        }

        repost::ActiveModel {
            user_id: Set(user.0),
            post_id: Set(post_id.0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(db_err)?;

        post::Entity::update_many()
            .col_expr(post::Column::RepostsCount, Expr::col(post::Column::RepostsCount).add(1))
            .filter(post::Column::Id.eq(post_id.0))
            .exec(&txn)
            .await
            .map_err(db_err)?;

        let count = reposts_count(&txn, post_id.0).await?;
        txn.commit().await.map_err(db_err)?;
        Ok(RepostToggle { changed: true, reposts_count: count })
    }

    async fn unrepost(&self, user: UserId, post_id: PostId) -> Result<RepostToggle, RepoError> {
        let txn = self.db.begin().await.map_err(db_err)?;

        let deleted = repost::Entity::delete_many()
            .filter(repost::Column::UserId.eq(user.0))
            .filter(repost::Column::PostId.eq(post_id.0))
            .exec(&txn)
            .await
            .map_err(db_err)?;

        let changed = deleted.rows_affected > 0;
        if changed {
            post::Entity::update_many()
                .col_expr(post::Column::RepostsCount, Expr::col(post::Column::RepostsCount).sub(1))
                .filter(post::Column::Id.eq(post_id.0))
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        let count = reposts_count(&txn, post_id.0).await?;
        txn.commit().await.map_err(db_err)?;
        Ok(RepostToggle { changed, reposts_count: count })
    }

    async fn has_reposted(&self, user: UserId, post_id: PostId) -> Result<bool, RepoError> {
        let found = repost::Entity::find()
            .filter(repost::Column::UserId.eq(user.0))
            .filter(repost::Column::PostId.eq(post_id.0))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.is_some())
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmBookmarkRepository {
    db: DatabaseConnection,
}

impl SeaOrmBookmarkRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BookmarkRepository for SeaOrmBookmarkRepository {
    async fn bookmark(&self, user: UserId, post_id: PostId) -> Result<bool, RepoError> {
        let existing = self.has_bookmarked(user, post_id).await?;
        if existing {
            return Ok(false);
        }
        bookmark::ActiveModel {
            user_id: Set(user.0),
            post_id: Set(post_id.0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(db_err)?;
        Ok(true)
    }

    async fn unbookmark(&self, user: UserId, post_id: PostId) -> Result<bool, RepoError> {
        let deleted = bookmark::Entity::delete_many()
            .filter(bookmark::Column::UserId.eq(user.0))
            .filter(bookmark::Column::PostId.eq(post_id.0))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(deleted.rows_affected > 0)
    }

    async fn has_bookmarked(&self, user: UserId, post_id: PostId) -> Result<bool, RepoError> {
        let found = bookmark::Entity::find()
            .filter(bookmark::Column::UserId.eq(user.0))
            .filter(bookmark::Column::PostId.eq(post_id.0))
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

/// Macro : génère une fonction « set des post_id de `viewer` dans la table
/// d'interaction donnée » — une requête bornée, vide si anonyme. Factorise les
/// trois relations (like/repost/bookmark) qui ont la même forme (user_id, post_id).
macro_rules! interaction_set_fn {
    ($fn_name:ident, $entity:ty, $col:ty) => {
        async fn $fn_name(
            db: &DatabaseConnection,
            viewer: Option<UserId>,
            post_ids: &[i64],
        ) -> Result<std::collections::HashSet<i64>, RepoError> {
            let Some(viewer) = viewer else {
                return Ok(std::collections::HashSet::new());
            };
            let rows = <$entity>::find()
                .filter(<$col>::UserId.eq(viewer.0))
                .filter(<$col>::PostId.is_in(post_ids.to_vec()))
                .all(db)
                .await
                .map_err(db_err)?;
            Ok(rows.into_iter().map(|r| r.post_id).collect())
        }
    };
}

interaction_set_fn!(liked_set, post_like::Entity, post_like::Column);
interaction_set_fn!(reposted_set, repost::Entity, repost::Column);
interaction_set_fn!(bookmarked_set, bookmark::Entity, bookmark::Column);

/// Joint une liste de posts à leurs auteurs **et** aux états de l'observateur
/// (liké/reposté/bookmarké), en requêtes bornées — pas de N+1.
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
    let reposted = reposted_set(db, viewer, &post_ids).await?;
    let bookmarked = bookmarked_set(db, viewer, &post_ids).await?;

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
                reposted_by_me: reposted.contains(&p.id),
                bookmarked_by_me: bookmarked.contains(&p.id),
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

    async fn following(&self, viewer: UserId, limit: u64) -> Result<Vec<FeedItem>, RepoError> {
        // 1. Ids suivis par l'observateur…
        let mut authors: Vec<i64> = follow::Entity::find()
            .filter(follow::Column::FollowerId.eq(viewer.0))
            .all(&self.db)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|f| f.following_id)
            .collect();
        // …plus soi-même (on voit toujours ses propres posts dans son fil).
        authors.push(viewer.0);

        let posts = post::Entity::find()
            .filter(post::Column::ParentId.is_null())
            .filter(post::Column::AuthorId.is_in(authors))
            .order_by_desc(post::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        join_authors(&self.db, Some(viewer), posts).await
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

    async fn by_match(
        &self,
        viewer: Option<UserId>,
        match_id: i64,
        limit: u64,
    ) -> Result<Vec<FeedItem>, RepoError> {
        let posts = post::Entity::find()
            .filter(post::Column::MatchId.eq(match_id))
            .order_by_desc(post::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        join_authors(&self.db, viewer, posts).await
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmSportCatalog {
    db: DatabaseConnection,
}

impl SeaOrmSportCatalog {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn to_team_row(t: team::Model) -> TeamRow {
    TeamRow {
        id: t.id,
        sport_id: t.sport_id,
        name: t.name,
        slug: t.slug,
        country: t.country,
    }
}

#[async_trait]
impl SportCatalog for SeaOrmSportCatalog {
    async fn list_sports(&self) -> Result<Vec<SportRow>, RepoError> {
        let rows = sport::Entity::find().all(&self.db).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|s| SportRow { id: s.id, name: s.name, slug: s.slug })
            .collect())
    }

    async fn list_teams(&self) -> Result<Vec<TeamRow>, RepoError> {
        let rows = team::Entity::find().all(&self.db).await.map_err(db_err)?;
        Ok(rows.into_iter().map(to_team_row).collect())
    }

    async fn team_by_slug(&self, slug: &str) -> Result<Option<TeamRow>, RepoError> {
        let found = team::Entity::find()
            .filter(team::Column::Slug.eq(slug))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.map(to_team_row))
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmMatchRepository {
    db: DatabaseConnection,
}

impl SeaOrmMatchRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// Mappe un `match_event` vers son read model, en résolvant les noms d'équipes.
async fn to_match_row(
    db: &DatabaseConnection,
    m: match_event::Model,
) -> Result<MatchRow, RepoError> {
    let team_name = |id: i64| {
        let db = db.clone();
        async move {
            team::Entity::find_by_id(id)
                .one(&db)
                .await
                .map_err(db_err)
                .map(|t| t.map(|t| t.name).unwrap_or_default())
        }
    };
    Ok(MatchRow {
        id: m.id,
        sport_id: m.sport_id,
        home_team: team_name(m.home_team_id).await?,
        away_team: team_name(m.away_team_id).await?,
        kickoff: m.kickoff_at.to_rfc3339(),
        status: m.status,
        home_score: m.home_score,
        away_score: m.away_score,
    })
}

#[async_trait]
impl MatchRepository for SeaOrmMatchRepository {
    async fn list(&self) -> Result<Vec<MatchRow>, RepoError> {
        let rows = match_event::Entity::find()
            .order_by_asc(match_event::Column::KickoffAt)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for m in rows {
            out.push(to_match_row(&self.db, m).await?);
        }
        Ok(out)
    }

    async fn by_id(&self, id: i64) -> Result<Option<MatchRow>, RepoError> {
        let Some(m) = match_event::Entity::find_by_id(id).one(&self.db).await.map_err(db_err)? else {
            return Ok(None);
        };
        Ok(Some(to_match_row(&self.db, m).await?))
    }
}

// ---------------------------------------------------------------------------

pub struct SeaOrmTeamFollowRepository {
    db: DatabaseConnection,
}

impl SeaOrmTeamFollowRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TeamFollowRepository for SeaOrmTeamFollowRepository {
    async fn follow(&self, user: UserId, team_id: TeamId) -> Result<bool, RepoError> {
        if self.has_followed(user, team_id).await? {
            return Ok(false);
        }
        team_follow::ActiveModel {
            user_id: Set(user.0),
            team_id: Set(team_id.0),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(db_err)?;
        Ok(true)
    }

    async fn unfollow(&self, user: UserId, team_id: TeamId) -> Result<bool, RepoError> {
        let res = team_follow::Entity::delete_many()
            .filter(team_follow::Column::UserId.eq(user.0))
            .filter(team_follow::Column::TeamId.eq(team_id.0))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected > 0)
    }

    async fn has_followed(&self, user: UserId, team_id: TeamId) -> Result<bool, RepoError> {
        let found = team_follow::Entity::find()
            .filter(team_follow::Column::UserId.eq(user.0))
            .filter(team_follow::Column::TeamId.eq(team_id.0))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(found.is_some())
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
        // Unfollow : idempotent (true la 1re fois, false ensuite).
        assert!(follows.remove(UserId(1), UserId(2)).await.unwrap());
        assert!(!follows.remove(UserId(1), UserId(2)).await.unwrap());
        assert!(!follows.exists(UserId(1), UserId(2)).await.unwrap());
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

    #[tokio::test]
    async fn following_feed_shows_followed_authors_and_self_only() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_reference_and_athletes(&db, &PasswordService::new()).await.unwrap();
        // 3e utilisateur non suivi.
        let users = SeaOrmUserRepository::new(db.clone());
        let stranger = users.create("stranger", "$argon2id$h", "Stranger").await.unwrap();

        let posts = SeaOrmPostRepository::new(db.clone());
        let c = |s: &str| PostContent::new(s).unwrap();
        posts.insert(UserId(1), &c("post de messi"), None).await.unwrap(); // self
        posts.insert(UserId(2), &c("post de ronaldo"), None).await.unwrap(); // suivi
        posts.insert(UserId(stranger.id), &c("post d'un inconnu"), None).await.unwrap(); // non suivi

        // messi (1) suit ronaldo (2), pas l'inconnu.
        SeaOrmFollowRepository::new(db.clone())
            .add(&Follow::new(UserId(1), UserId(2)).unwrap())
            .await
            .unwrap();

        let feed = SeaOrmFeedRepository::new(db);
        let items = feed.following(UserId(1), 50).await.unwrap();
        let contents: Vec<&str> = items.iter().map(|i| i.content.as_str()).collect();

        assert!(contents.contains(&"post de messi"), "doit inclure ses propres posts");
        assert!(contents.contains(&"post de ronaldo"), "doit inclure les suivis");
        assert!(!contents.contains(&"post d'un inconnu"), "doit exclure les non-suivis");
    }

    #[tokio::test]
    async fn repost_counts_atomically_and_bookmark_is_private() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_reference_and_athletes(&db, &PasswordService::new()).await.unwrap();
        let posts = SeaOrmPostRepository::new(db.clone());
        let p = posts.insert(UserId(1), &PostContent::new("Golazo!").unwrap(), None).await.unwrap();

        // Repost : compteur atomique + idempotence.
        let reposts = SeaOrmRepostRepository::new(db.clone());
        assert_eq!(reposts.repost(UserId(2), p.id).await.unwrap().reposts_count, 1);
        assert!(!reposts.repost(UserId(2), p.id).await.unwrap().changed);
        assert!(reposts.has_reposted(UserId(2), p.id).await.unwrap());
        assert_eq!(reposts.unrepost(UserId(2), p.id).await.unwrap().reposts_count, 0);

        // Le compteur sur le post reflète bien l'opération.
        let model = post::Entity::find_by_id(p.id.0).one(&db).await.unwrap().unwrap();
        assert_eq!(model.reposts_count, 0);

        // Bookmark : idempotent, sans compteur public.
        let bm = SeaOrmBookmarkRepository::new(db.clone());
        assert!(bm.bookmark(UserId(2), p.id).await.unwrap());
        assert!(!bm.bookmark(UserId(2), p.id).await.unwrap());
        assert!(bm.has_bookmarked(UserId(2), p.id).await.unwrap());
        assert!(bm.unbookmark(UserId(2), p.id).await.unwrap());
        assert!(!bm.has_bookmarked(UserId(2), p.id).await.unwrap());
    }

    #[tokio::test]
    async fn sport_catalog_match_feed_and_team_follow() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        let report = seed_reference_and_athletes(&db, &PasswordService::new()).await.unwrap();
        assert_eq!(report.matches, 1);

        // Catalogue.
        let catalog = SeaOrmSportCatalog::new(db.clone());
        assert_eq!(catalog.list_sports().await.unwrap().len(), 1);
        assert_eq!(catalog.list_teams().await.unwrap().len(), 2);
        let team = catalog.team_by_slug("inter-miami").await.unwrap().unwrap();
        assert_eq!(team.name, "Inter Miami CF");

        // Match seedé (id=1), noms d'équipes résolus.
        let matches = SeaOrmMatchRepository::new(db.clone());
        let m = matches.by_id(1).await.unwrap().unwrap();
        assert_eq!(m.home_team, "Inter Miami CF");
        assert_eq!(m.away_team, "Al Nassr FC");
        assert_eq!(m.status, "live");

        // Post about match → sport dérivé + visible dans le feed du match.
        let posts = SeaOrmPostRepository::new(db.clone());
        let content = PostContent::new("GOOOAL ! #Football").unwrap();
        let p = posts
            .insert_about_match(UserId(1), &content, SportId(m.sport_id), MatchId(m.id))
            .await
            .unwrap();
        assert_eq!(p.match_id, Some(MatchId(1)));
        let feed = SeaOrmFeedRepository::new(db.clone());
        let items = feed.by_match(None, 1, 50).await.unwrap();
        assert!(items.iter().any(|i| i.content.contains("GOOOAL")));

        // Suivi d'équipe : toggle idempotent.
        let tf = SeaOrmTeamFollowRepository::new(db);
        assert!(tf.follow(UserId(1), TeamId(team.id)).await.unwrap());
        assert!(!tf.follow(UserId(1), TeamId(team.id)).await.unwrap());
        assert!(tf.has_followed(UserId(1), TeamId(team.id)).await.unwrap());
        assert!(tf.unfollow(UserId(1), TeamId(team.id)).await.unwrap());
        assert!(!tf.has_followed(UserId(1), TeamId(team.id)).await.unwrap());
    }
}
