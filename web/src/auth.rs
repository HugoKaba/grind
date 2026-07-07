//! Auth côté serveur pour les server functions (SSR uniquement) :
//! JWT HS256 + cookie de session HttpOnly.

use axum::http::{header::COOKIE, HeaderMap};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub is_staff: bool,
    pub exp: usize,
}

pub fn issue_token(secret: &str, user_id: i64, username: &str, is_staff: bool) -> String {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims { sub: user_id, username: username.to_owned(), is_staff, exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .expect("jwt encoding never fails with HS256")
}

pub fn session_cookie(token: &str) -> String {
    format!("session={token}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400")
}

pub fn decode_token(secret: &str, token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|d| d.claims)
}

pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(|c| c.trim())
        .find_map(|c| c.strip_prefix("session="))
        .map(|s| s.to_owned())
}
