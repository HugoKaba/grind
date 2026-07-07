//! # grind-shared
//!
//! DTOs (dé)sérialisables partagés entre le serveur (Axum/Leptos server fns)
//! et le client (WASM). Doivent rester WASM-safe : `serde` uniquement.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorDto {
    pub id: i64,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostDto {
    pub id: i64,
    pub author: AuthorDto,
    pub content: String,
    pub parent_id: Option<i64>,
    pub sport_id: Option<i64>,
    pub match_id: Option<i64>,
    pub likes_count: i64,
    pub reposts_count: i64,
    pub replies_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub parent_id: Option<i64>,
    pub match_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Résultat de connexion renvoyé au client (le token part en cookie HttpOnly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginDto {
    pub username: String,
    pub is_staff: bool,
}

/// État d'un like renvoyé après un toggle (WASM-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LikeStateDto {
    pub post_id: i64,
    pub liked: bool,
    pub likes_count: i64,
}

/// État d'une relation de suivi renvoyé après un toggle (WASM-safe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowStateDto {
    pub target_username: String,
    pub following: bool,
}

/// État d'un repost renvoyé après un toggle (WASM-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepostStateDto {
    pub post_id: i64,
    pub reposted: bool,
    pub reposts_count: i64,
}

/// État d'un bookmark renvoyé après un toggle (privé, WASM-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkStateDto {
    pub post_id: i64,
    pub bookmarked: bool,
}

/// Élément de fil pour l'affichage (retour de server function, WASM-safe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedItemDto {
    pub id: i64,
    pub author_username: String,
    pub author_display: String,
    pub content: String,
    pub likes_count: i64,
    pub reposts_count: i64,
    pub replies_count: i64,
    pub created_at: String,
    /// Flags viewer-aware (tous `false` si anonyme).
    pub liked_by_me: bool,
    pub reposted_by_me: bool,
    pub bookmarked_by_me: bool,
}

/// Page profil (viewer-aware) : posts de l'athlète + état de suivi pour l'observateur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDto {
    pub username: String,
    /// `true` si l'observateur suit déjà cet athlète.
    pub is_following: bool,
    /// `true` si un bouton Suivre a du sens (connecté, et pas soi-même).
    pub can_follow: bool,
    pub posts: Vec<FeedItemDto>,
}

// --- Domaine sport (WASM-safe) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SportDto {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamDto {
    pub id: i64,
    pub sport_id: i64,
    pub name: String,
    pub slug: String,
    pub country: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchDto {
    pub id: i64,
    pub sport_id: i64,
    pub home_team: String,
    pub away_team: String,
    pub kickoff: String,
    pub status: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
}

/// Catalogue reference data (page /sports).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogDto {
    pub sports: Vec<SportDto>,
    pub teams: Vec<TeamDto>,
    pub matches: Vec<MatchDto>,
}

/// Page équipe (viewer-aware) : infos + état de suivi de l'équipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamPageDto {
    pub team: TeamDto,
    pub is_following: bool,
    pub can_follow: bool,
}

/// État de suivi d'une équipe renvoyé après un toggle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFollowStateDto {
    pub team_slug: String,
    pub following: bool,
}

/// Page match (live) : infos + fil des posts liés au match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchPageDto {
    pub game: MatchDto,
    pub posts: Vec<FeedItemDto>,
}

// --- Messagerie & notifications (WASM-safe) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDto {
    pub id: i64,
    pub sender_username: String,
    pub recipient_username: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationDto {
    pub other_username: String,
    pub last_body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDto {
    pub id: i64,
    pub kind: String,
    pub actor_username: String,
    pub is_read: bool,
    pub created_at: String,
}
