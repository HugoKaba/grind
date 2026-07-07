//! Construction du routeur Axum (SSR) : server functions + routes Leptos.
//! Extrait ici pour être réutilisé par le binaire **et** les tests d'intégration.

use axum::body::Body;
use axum::extract::{FromRef, State};
use axum::http::Request;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use leptos::config::LeptosOptions;
use leptos::prelude::*;
use leptos_axum::{
    file_and_error_handler, generate_route_list, handle_server_fns_with_context, LeptosRoutes,
};

use crate::state::DomainState;
use crate::{shell, App};

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub domain: DomainState,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(s: &AppState) -> Self {
        s.leptos_options.clone()
    }
}
impl FromRef<AppState> for DomainState {
    fn from_ref(s: &AppState) -> Self {
        s.domain.clone()
    }
}

/// Handler des server functions : injecte le `DomainState` dans le context Leptos.
async fn server_fns(State(domain): State<DomainState>, req: Request<Body>) -> impl IntoResponse {
    handle_server_fns_with_context(move || provide_context(domain.clone()), req).await
}

/// Routeur complet : server functions (`/api`) + routes Leptos (SSR + hydratation).
pub fn build_router(app_state: AppState) -> Router {
    let leptos_options = app_state.leptos_options.clone();
    let routes = generate_route_list(App);
    Router::new()
        .route("/api/*fn_name", get(server_fns).post(server_fns))
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let domain = app_state.domain.clone();
                move || provide_context(domain.clone())
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(file_and_error_handler::<AppState, _>(shell))
        .with_state(app_state)
}

/// Routeur minimal : server functions uniquement (pour les tests d'intégration).
pub fn api_router(domain: DomainState) -> Router {
    Router::new()
        .route("/api/*fn_name", get(server_fns).post(server_fns))
        .with_state(domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header::SET_COOKIE, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use grind_infrastructure::persistence::{
        connect_and_migrate, seed::seed_reference_and_athletes, SeaOrmPostRepository,
    };
    use grind_infrastructure::security::PasswordService;

    async fn setup() -> Router {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();
        seed_reference_and_athletes(&db, &PasswordService::new())
            .await
            .unwrap();
        {
            use grind_application::PostRepository;
            use grind_domain::entities::UserId;
            use grind_domain::value_objects::PostContent;
            let posts = SeaOrmPostRepository::new(db.clone());
            posts
                .insert(UserId(1), &PostContent::new("Seed post").unwrap(), None)
                .await
                .unwrap();
        }
        api_router(DomainState::new(db, "test-secret".to_owned()))
    }

    fn form(uri: &str, cookie: Option<&str>, body: &str) -> Request<Body> {
        let mut b = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded");
        if let Some(c) = cookie {
            b = b.header("cookie", c);
        }
        b.body(Body::from(body.to_owned())).unwrap()
    }

    async fn body_string(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn session_of(resp: &axum::response::Response) -> String {
        resp.headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn server_functions_end_to_end() {
        let app = setup().await;

        // login (messi, staff) → 200 + cookie de session
        let resp = app
            .clone()
            .oneshot(form("/api/login", None, "username=messi&password=grind1234"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let messi = session_of(&resp);
        assert!(messi.starts_with("session="));

        // mauvais mot de passe → pas 200
        let resp = app
            .clone()
            .oneshot(form("/api/login", None, "username=messi&password=WRONG"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // create_post sans cookie → pas 200
        let resp = app
            .clone()
            .oneshot(form("/api/create_post", None, "content=hack"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // create_post avec cookie → 200 + contenu
        let resp = app
            .clone()
            .oneshot(form("/api/create_post", Some(&messi), "content=Via server fn"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Via server fn"));

        // get_timeline contient le nouveau post
        let resp = app
            .clone()
            .oneshot(form("/api/get_timeline", None, ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Via server fn"));

        // toggle_like sans cookie → refusé
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_like", None, "id=1"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // toggle_like (messi) sur le post seedé (id=1) → like : liked=true, count=1
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_like", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("\"liked\":true"), "attendu liked=true, reçu: {body}");
        assert!(body.contains("\"likes_count\":1"), "attendu likes_count=1, reçu: {body}");

        // get_timeline vu par messi (qui vient de liker id=1) → liked_by_me=true présent
        let resp = app
            .clone()
            .oneshot(form("/api/get_timeline", Some(&messi), ""))
            .await
            .unwrap();
        let body = body_string(resp).await;
        assert!(
            body.contains("\"liked_by_me\":true"),
            "messi doit voir son propre like: {body}"
        );

        // get_timeline anonyme → aucun liked_by_me=true (viewer-aware)
        let resp = app
            .clone()
            .oneshot(form("/api/get_timeline", None, ""))
            .await
            .unwrap();
        let body = body_string(resp).await;
        assert!(
            !body.contains("\"liked_by_me\":true"),
            "un anonyme ne doit voir aucun like personnel: {body}"
        );

        // re-toggle (messi) → unlike : liked=false, count=0
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_like", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("\"liked\":false"), "attendu liked=false, reçu: {body}");
        assert!(body.contains("\"likes_count\":0"), "attendu likes_count=0, reçu: {body}");

        // delete_post non-staff (ronaldo) → refusé
        let resp = app
            .clone()
            .oneshot(form("/api/login", None, "username=ronaldo&password=grind1234"))
            .await
            .unwrap();
        let ronaldo = session_of(&resp);
        let resp = app
            .clone()
            .oneshot(form("/api/delete_post", Some(&ronaldo), "id=1"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // delete_post staff (messi) → 200
        let resp = app
            .clone()
            .oneshot(form("/api/delete_post", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
