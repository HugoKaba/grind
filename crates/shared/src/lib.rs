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
