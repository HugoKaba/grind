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

        // register nouveau compte → 200 + cookie de session (auto-login)
        let resp = app
            .clone()
            .oneshot(form(
                "/api/register",
                None,
                "username=newpro&display_name=New+Pro&password=grind1234",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(session_of(&resp).starts_with("session="));

        // register username déjà pris (messi seedé) → refusé (Conflict)
        let resp = app
            .clone()
            .oneshot(form(
                "/api/register",
                None,
                "username=messi&display_name=&password=grind1234",
            ))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // register mot de passe trop court → refusé (politique applicative)
        let resp = app
            .clone()
            .oneshot(form(
                "/api/register",
                None,
                "username=tooshort&display_name=&password=abc",
            ))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // le compte créé peut se connecter (hash argon2 vérifiable)
        let resp = app
            .clone()
            .oneshot(form("/api/login", None, "username=newpro&password=grind1234"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

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

        // reply sans cookie → refusé
        let resp = app
            .clone()
            .oneshot(form("/api/reply", None, "parent_id=1&content=hack"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // reply (messi) au post seedé id=1 → 200, puis visible dans le thread
        let resp = app
            .clone()
            .oneshot(form("/api/reply", Some(&messi), "parent_id=1&content=Belle analyse"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let resp = app
            .clone()
            .oneshot(form("/api/get_post_detail", None, "id=1"))
            .await
            .unwrap();
        assert!(body_string(resp).await.contains("Belle analyse"), "le thread doit contenir la réponse");

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

        // toggle_repost (messi) sur id=1 → reposted=true, count=1 ; re-toggle → 0
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_repost", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("\"reposted\":true") && body.contains("\"reposts_count\":1"), "{body}");
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_repost", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert!(body_string(resp).await.contains("\"reposted\":false"));

        // toggle_repost sans cookie → refusé
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_repost", None, "id=1"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // toggle_bookmark (messi) sur id=1 → bookmarked=true ; re-toggle → false
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_bookmark", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"bookmarked\":true"));
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_bookmark", Some(&messi), "id=1"))
            .await
            .unwrap();
        assert!(body_string(resp).await.contains("\"bookmarked\":false"));

        // toggle_follow sans cookie → refusé
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_follow", None, "username=ronaldo"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // messi suit ronaldo → following=true
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_follow", Some(&messi), "username=ronaldo"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"following\":true"));

        // get_profile de ronaldo vu par messi → is_following=true (viewer-aware)
        let resp = app
            .clone()
            .oneshot(form("/api/get_profile", Some(&messi), "username=ronaldo"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"is_following\":true"));

        // messi re-toggle → unfollow : following=false
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_follow", Some(&messi), "username=ronaldo"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"following\":false"));

        // messi tente de se suivre lui-même → refusé (invariant domaine SelfFollow)
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_follow", Some(&messi), "username=messi"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // --- Fil personnalisé (following) + admin_list_posts ---
        // ronaldo publie un post.
        let resp = app
            .clone()
            .oneshot(form("/api/login", None, "username=ronaldo&password=grind1234"))
            .await
            .unwrap();
        let ronaldo = session_of(&resp);
        let resp = app
            .clone()
            .oneshot(form("/api/create_post", Some(&ronaldo), "content=Golazo de ronaldo"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // messi suit ronaldo → son fil personnalisé inclut désormais le post de ronaldo.
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_follow", Some(&messi), "username=ronaldo"))
            .await
            .unwrap();
        assert!(body_string(resp).await.contains("\"following\":true"));
        let resp = app
            .clone()
            .oneshot(form("/api/get_timeline", Some(&messi), ""))
            .await
            .unwrap();
        assert!(
            body_string(resp).await.contains("Golazo de ronaldo"),
            "le fil de messi doit inclure les posts des personnes suivies"
        );

        // admin_list_posts : messi (staff) voit tout ; ronaldo (non-staff) refusé.
        let resp = app
            .clone()
            .oneshot(form("/api/admin_list_posts", Some(&messi), ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Golazo de ronaldo"));
        let resp = app
            .clone()
            .oneshot(form("/api/admin_list_posts", Some(&ronaldo), ""))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // --- Domaine sport ---
        // Catalogue : contient les équipes seedées.
        let resp = app
            .clone()
            .oneshot(form("/api/get_catalog", None, ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Inter Miami CF"));

        // Suivi d'équipe : messi suit Inter Miami → following=true.
        let resp = app
            .clone()
            .oneshot(form("/api/toggle_team_follow", Some(&messi), "slug=inter-miami"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"following\":true"));

        // post_about_match (messi) sur le match seedé (id=1) → visible dans le fil du match.
        let resp = app
            .clone()
            .oneshot(form("/api/post_about_match", Some(&messi), "match_id=1&content=Quel match !"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let resp = app
            .clone()
            .oneshot(form("/api/get_match", None, "id=1"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Quel match !"), "le fil du match doit contenir le post");

        // post_about_match sans cookie → refusé.
        let resp = app
            .clone()
            .oneshot(form("/api/post_about_match", None, "match_id=1&content=hack"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // --- Messagerie & notifications ---
        // (à ce stade messi suit ronaldo → il peut lui écrire)
        // send_message sans cookie → refusé.
        let resp = app
            .clone()
            .oneshot(form("/api/send_message", None, "recipient=ronaldo&body=hi"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // messi écrit à ronaldo (suivi) → 200.
        let resp = app
            .clone()
            .oneshot(form("/api/send_message", Some(&messi), "recipient=ronaldo&body=Bien joue hier"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // messi écrit à newpro (non suivi) → refusé (restriction produit).
        let resp = app
            .clone()
            .oneshot(form("/api/send_message", Some(&messi), "recipient=newpro&body=coucou"))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::OK);

        // Le thread messi↔ronaldo contient le message.
        let resp = app
            .clone()
            .oneshot(form("/api/get_thread", Some(&messi), "username=ronaldo"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("Bien joue hier"));

        // ronaldo a reçu une notification de type "message".
        let resp = app
            .clone()
            .oneshot(form("/api/get_notifications", Some(&ronaldo), ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_string(resp).await.contains("\"kind\":\"message\""));

        // ronaldo marque tout comme lu → 200.
        let resp = app
            .clone()
            .oneshot(form("/api/mark_notifications_read", Some(&ronaldo), ""))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

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
