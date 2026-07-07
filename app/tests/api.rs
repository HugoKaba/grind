//! Test d'intégration bout-en-bout : présentation → application → infrastructure,
//! sur SQLite en mémoire, via `tower::ServiceExt::oneshot` (pas de vrai réseau).

use axum::body::Body;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use grind_app::{app_router, build_state};
use grind_infrastructure::persistence::{connect_and_migrate, seed::seed_reference_and_athletes};
use grind_infrastructure::security::PasswordService;

async fn test_router() -> Router {
    let db = connect_and_migrate("sqlite::memory:").await.unwrap();
    seed_reference_and_athletes(&db, &PasswordService::new())
        .await
        .unwrap();
    app_router(build_state(db, "test-secret".to_owned()))
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

fn json_request(method: &str, uri: &str, token: Option<&str>, body: serde_json::Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        builder = builder.header(AUTHORIZATION, format!("Bearer {t}"));
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn login_create_post_and_timeline_flow() {
    let app = test_router().await;

    // 1) Login avec un athlète seedé (mot de passe argon2 "grind1234").
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/login",
            None,
            serde_json::json!({ "username": "messi", "password": "grind1234" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let token = body["token"].as_str().expect("token").to_owned();
    assert_eq!(body["username"], "messi");

    // 2) Mauvais mot de passe → 401.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/login",
            None,
            serde_json::json!({ "username": "messi", "password": "wrong" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 3) Créer un post SANS token → 401.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/posts",
            None,
            serde_json::json!({ "content": "hello", "parent_id": null, "match_id": null }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 4) Créer un post AVEC token → 200.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/posts",
            Some(&token),
            serde_json::json!({ "content": "Golazo! #Goals", "parent_id": null, "match_id": null }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let post = body_json(resp).await;
    assert_eq!(post["content"], "Golazo! #Goals");

    // 5) Contenu invalide (>280) → 400.
    let long = "a".repeat(281);
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/posts",
            Some(&token),
            serde_json::json!({ "content": long, "parent_id": null, "match_id": null }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 6) Timeline JSON contient le post.
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/timeline").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let feed = body_json(resp).await;
    let arr = feed.as_array().unwrap();
    assert!(arr.iter().any(|p| p["content"] == "Golazo! #Goals"));
    assert!(arr.iter().any(|p| p["author_username"] == "messi"));

    // 7) Page HTML (SSR Leptos) rend le contenu.
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let html = String::from_utf8(resp.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(html.contains("GRIND"));
    assert!(html.contains("Golazo!"));
}

#[tokio::test]
async fn admin_moderation_cookie_and_pages() {
    let app = test_router().await;

    // Login messi (staff) — vérifie is_staff + cookie de session émis.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/login",
            None,
            serde_json::json!({ "username": "messi", "password": "grind1234" }),
        ))
        .await
        .unwrap();
    assert!(resp.headers().get("set-cookie").is_some(), "cookie de session attendu");
    let messi = body_json(resp).await;
    assert_eq!(messi["is_staff"], true);
    let messi_token = messi["token"].as_str().unwrap().to_owned();

    // Login ronaldo (non-staff).
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/login",
            None,
            serde_json::json!({ "username": "ronaldo", "password": "grind1234" }),
        ))
        .await
        .unwrap();
    let ronaldo = body_json(resp).await;
    assert_eq!(ronaldo["is_staff"], false);
    let ronaldo_token = ronaldo["token"].as_str().unwrap().to_owned();

    // Auth par COOKIE : messi crée un post via le cookie de session (pas de Bearer).
    let cookie_req = Request::builder()
        .method("POST")
        .uri("/api/posts")
        .header("content-type", "application/json")
        .header("cookie", format!("session={messi_token}"))
        .body(Body::from(
            serde_json::json!({ "content": "à modérer", "parent_id": null, "match_id": null }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(cookie_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let post = body_json(resp).await;
    let post_id = post["id"].as_i64().unwrap();

    // Non-staff ne peut pas modérer → 403.
    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/api/admin/posts/{post_id}"),
            Some(&ronaldo_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Sans token → 401.
    let resp = app
        .clone()
        .oneshot(json_request("DELETE", &format!("/api/admin/posts/{post_id}"), None, serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Staff supprime → 200, puis 404 à la seconde tentative.
    let del = |token: &str| {
        json_request("DELETE", &format!("/api/admin/posts/{post_id}"), Some(token), serde_json::json!({}))
    };
    let resp = app.clone().oneshot(del(&messi_token)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app.clone().oneshot(del(&messi_token)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Page profil (SSR).
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/u/messi").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let html = String::from_utf8(resp.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(html.contains("messi"));

    // Page détail d'un post inexistant → 404.
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/post/999999").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
