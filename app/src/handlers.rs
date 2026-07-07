//! Controllers HTTP (équivalents des server functions Leptos) :
//! ils (dé)sérialisent, appellent le use case injecté, mappent la sortie.
//! **Zéro logique métier ici** — tout est dans les use cases.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use serde_json::{json, Value};

use grind_application::{AppError, CreatePost, Login};
use grind_domain::entities::{PostId, UserId};
use grind_shared::{CreatePostRequest, LoginRequest};

use crate::auth::{issue_token, AuthUser};
use crate::state::AppState;
use crate::views;

type ApiError = (StatusCode, String);

/// POST /api/login — auth hybride → JWT.
pub async fn login(
    State(st): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, ApiError> {
    let uc = Login::new(&*st.users, &*st.hasher);
    match uc.execute(&req.username, &req.password).await {
        Ok(Some(out)) => {
            let token = issue_token(&st.jwt_secret, out.user_id, &out.username);
            Ok(Json(json!({
                "token": token,
                "user_id": out.user_id,
                "username": out.username,
            })))
        }
        Ok(None) => Err((StatusCode::UNAUTHORIZED, "invalid credentials".into())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// POST /api/posts — crée un post (auth requise).
pub async fn create_post(
    auth: AuthUser,
    State(st): State<AppState>,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<Value>, ApiError> {
    let uc = CreatePost::new(&*st.posts);
    let parent = req.parent_id.map(PostId);
    match uc.execute(UserId(auth.user_id), &req.content, parent).await {
        Ok(post) => Ok(Json(json!({
            "id": post.id.0,
            "author_id": post.author.0,
            "content": post.content.as_str(),
            "is_reply": post.is_reply(),
        }))),
        Err(AppError::Domain(_)) => {
            Err((StatusCode::BAD_REQUEST, "post content violates a rule (1..=280 chars)".into()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// GET /api/timeline — fil récent (JSON).
pub async fn timeline_json(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let items = st
        .feed
        .recent(50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let arr: Vec<Value> = items
        .into_iter()
        .map(|i| {
            json!({
                "id": i.id,
                "author_username": i.author_username,
                "author_display": i.author_display,
                "content": i.content,
                "likes_count": i.likes_count,
                "reposts_count": i.reposts_count,
                "replies_count": i.replies_count,
                "created_at": i.created_at,
            })
        })
        .collect();

    Ok(Json(Value::Array(arr)))
}

/// GET / — page timeline rendue côté serveur avec Leptos (SSR).
pub async fn timeline_html(State(st): State<AppState>) -> Html<String> {
    let items = st.feed.recent(50).await.unwrap_or_default();
    Html(views::render_timeline(&items))
}
