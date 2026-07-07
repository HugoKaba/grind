//! Authentification : émission/validation de JWT + extractors `AuthUser` / `AdminUser`.
//! Le token est accepté via `Authorization: Bearer <jwt>` **ou** un cookie de session `session=<jwt>`.

use axum::extract::FromRequestParts;
use axum::http::{
    header::{AUTHORIZATION, COOKIE},
    request::Parts,
    StatusCode,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub is_staff: bool,
    pub exp: usize,
}

/// Émet un JWT HS256 valable 24 h.
pub fn issue_token(secret: &str, user_id: i64, username: &str, is_staff: bool) -> String {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims { sub: user_id, username: username.to_owned(), is_staff, exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .expect("jwt encoding never fails with HS256 + valid key")
}

/// Construit la valeur d'un cookie de session HttpOnly.
pub fn session_cookie(token: &str) -> String {
    format!("session={token}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400")
}

fn token_from_parts(parts: &Parts) -> Option<&str> {
    // 1) Authorization: Bearer <jwt>
    if let Some(bearer) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        return Some(bearer);
    }
    // 2) Cookie: session=<jwt>
    parts
        .headers
        .get(COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(|c| c.trim())
                .find_map(|c| c.strip_prefix("session="))
        })
}

/// Utilisateur authentifié (Bearer ou cookie de session).
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
    pub is_staff: bool,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = token_from_parts(parts)
            .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token or session cookie"))?;

        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token"))?;

        Ok(AuthUser {
            user_id: data.claims.sub,
            username: data.claims.username,
            is_staff: data.claims.is_staff,
        })
    }
}

/// Utilisateur staff : 403 si `is_staff` est faux. Garde les routes `/admin`.
pub struct AdminUser(pub AuthUser);

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_staff {
            Ok(AdminUser(user))
        } else {
            Err((StatusCode::FORBIDDEN, "staff privileges required"))
        }
    }
}
