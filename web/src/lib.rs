//! # grind-web
//!
//! App Leptos **full-stack hydratée** (SSR + WASM). Les server functions sont
//! câblées sur les use cases/repos réels via le context Leptos (`DomainState`),
//! injecté côté serveur. Construite avec `cargo leptos build`.

use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::hooks::use_params_map;
use leptos_router::path;

use grind_shared::{FeedItemDto, LoginDto};

#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod state;

/// Document HTML (shell) — injecté par le serveur, hydraté par le client.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/grind.css" />
        <Title text="GRIND" />
        <Router>
            <nav>
                <A href="/">"Accueil"</A>" · "<A href="/admin">"Admin"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page introuvable.".into_view()>
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/post/:id") view=PostDetail />
                    <Route path=path!("/u/:username") view=Profile />
                    <Route path=path!("/admin") view=Admin />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn Home() -> impl IntoView {
    // Îlot interactif (hydratation client).
    let (count, set_count) = signal(0);

    // Server actions (formulaires → server functions).
    let login = ServerAction::<Login>::new();
    let create = ServerAction::<CreatePost>::new();

    // Le fil se recharge après chaque publication (source = version de l'action).
    let timeline = Resource::new(move || create.version().get(), |_| get_timeline());

    view! {
        <h1>"🏟️ GRIND"</h1>
        <button on:click=move |_| *set_count.write() += 1>"Likes locaux : " {count}</button>

        <h2>"Connexion"</h2>
        <ActionForm action=login>
            <input type="text" name="username" placeholder="username (ex: messi)" />
            <input type="password" name="password" placeholder="mot de passe (grind1234)" />
            <button type="submit">"Se connecter"</button>
        </ActionForm>
        <p>
            {move || match login.value().get() {
                Some(Ok(u)) => format!("Connecté : @{} (staff : {})", u.username, u.is_staff),
                Some(Err(e)) => format!("Échec : {e}"),
                None => String::new(),
            }}
        </p>

        <h2>"Nouveau post"</h2>
        <ActionForm action=create>
            <input type="text" name="content" placeholder="Quoi de neuf ?" />
            <button type="submit">"Publier"</button>
        </ActionForm>

        <h2>"Fil"</h2>
        <Suspense fallback=|| view! { <p>"Chargement du fil…"</p> }>
            {move || {
                timeline
                    .get()
                    .map(|res| match res {
                        Ok(items) => {
                            view! {
                                <ul class="feed">
                                    {items
                                        .into_iter()
                                        .map(|i| {
                                            view! {
                                                <li class="post">
                                                    <strong>"@"{i.author_username}</strong>
                                                    " · "
                                                    <span>{i.content}</span>
                                                    " — "
                                                    <em>{i.likes_count}" ❤"</em>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <p>"Erreur : " {e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn PostDetail() -> impl IntoView {
    let params = use_params_map();
    let post = Resource::new(
        move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()),
        |maybe_id| async move {
            match maybe_id {
                Some(id) => get_post_detail(id).await,
                None => Ok((None, Vec::new())),
            }
        },
    );

    view! {
        <h1>"🏟️ GRIND — Post"</h1>
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                post.get()
                    .map(|res| match res {
                        Ok((Some(p), replies)) => {
                            view! {
                                <article class="post-detail">
                                    <h2>"@"{p.author_display.clone()}</h2>
                                    <p>{p.content.clone()}</p>
                                    <small>{p.likes_count}" ❤ · "{p.replies_count}" 💬"</small>
                                </article>
                                <h3>"Réponses"</h3>
                                <ul class="thread">
                                    {replies
                                        .into_iter()
                                        .map(|r| {
                                            view! {
                                                <li><strong>"@"{r.author_username}</strong>" "{r.content}</li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                        Ok((None, _)) => view! { <p>"Post introuvable."</p> }.into_any(),
                        Err(e) => view! { <p>"Erreur : " {e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Profile() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.read().get("username").unwrap_or_default();
    let posts = Resource::new(username, |name| async move { get_profile(name).await });

    view! {
        <h1>"👤 @"{username}</h1>
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                posts
                    .get()
                    .map(|res| match res {
                        Ok(items) => {
                            view! {
                                <ul class="feed">
                                    {items
                                        .into_iter()
                                        .map(|i| view! { <li>{i.content}" — "{i.likes_count}" ❤"</li> })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <p>"Erreur : " {e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Admin() -> impl IntoView {
    let del = ServerAction::<DeletePost>::new();
    let list = Resource::new(move || del.version().get(), |_| get_timeline());

    view! {
        <h1>"🛠️ Admin — modération"</h1>
        <p>
            {move || match del.value().get() {
                Some(Ok(())) => "Post supprimé.".to_string(),
                Some(Err(e)) => format!("Erreur : {e}"),
                None => String::new(),
            }}
        </p>
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                list.get()
                    .map(|res| match res {
                        Ok(items) => {
                            view! {
                                <ul class="feed">
                                    {items
                                        .into_iter()
                                        .map(|i| {
                                            let id = i.id;
                                            view! {
                                                <li>
                                                    "#"{i.id}" @"{i.author_username}" : "{i.content}" "
                                                    <ActionForm action=del>
                                                        <input type="hidden" name="id" value=id />
                                                        <button type="submit">"Supprimer"</button>
                                                    </ActionForm>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <p>"Erreur : " {e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

/// Mappe un read model `FeedItem` (application) vers son DTO WASM-safe.
#[cfg(feature = "ssr")]
fn to_dto(i: grind_application::FeedItem) -> FeedItemDto {
    FeedItemDto {
        id: i.id,
        author_username: i.author_username,
        author_display: i.author_display,
        content: i.content,
        likes_count: i.likes_count,
        reposts_count: i.reposts_count,
        replies_count: i.replies_count,
        created_at: i.created_at,
    }
}

/// Server function : lit le fil via le use case/repo réel (context `DomainState`).
#[server]
pub async fn get_timeline() -> Result<Vec<FeedItemDto>, ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent du context"))?;

    let items = state
        .feed
        .recent(50)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(items.into_iter().map(to_dto).collect())
}

/// Server function : détail d'un post + son thread.
#[server]
pub async fn get_post_detail(
    id: i64,
) -> Result<(Option<FeedItemDto>, Vec<FeedItemDto>), ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent"))?;
    let post = state
        .feed
        .by_id(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map(to_dto);
    let replies = state
        .feed
        .replies(id, 100)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(to_dto)
        .collect();
    Ok((post, replies))
}

/// Server function : posts d'un athlète (page profil).
#[server]
pub async fn get_profile(username: String) -> Result<Vec<FeedItemDto>, ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent"))?;
    let items = state
        .feed
        .by_author(&username, 50)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(items.into_iter().map(to_dto).collect())
}

/// Server function : suppression d'un post (staff uniquement, auth par cookie).
#[server]
pub async fn delete_post(id: i64) -> Result<(), ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent"))?;
    let headers = leptos_axum::extract::<axum::http::HeaderMap>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let claims = auth::token_from_headers(&headers)
        .and_then(|t| auth::decode_token(&state.jwt_secret, &t))
        .ok_or_else(|| ServerFnError::new("Non authentifié"))?;
    if !claims.is_staff {
        return Err(ServerFnError::new("Réservé au staff"));
    }
    grind_application::DeletePost::new(&*state.posts)
        .execute(grind_domain::entities::PostId(id))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Server function : connexion (auth hybride via use case) → pose un cookie de session.
#[server]
pub async fn login(username: String, password: String) -> Result<LoginDto, ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent"))?;

    let uc = grind_application::Login::new(&*state.users, &*state.hasher);
    let out = uc
        .execute(&username, &password)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Identifiants invalides"))?;

    let token = auth::issue_token(&state.jwt_secret, out.user_id, &out.username, out.is_staff);
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&auth::session_cookie(&token))
            .map_err(|e| ServerFnError::new(e.to_string()))?,
    );

    Ok(LoginDto { username: out.username, is_staff: out.is_staff })
}

/// Server function : crée un post (auth par cookie de session).
#[server]
pub async fn create_post(content: String) -> Result<FeedItemDto, ServerFnError> {
    let state = use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent"))?;

    let headers = leptos_axum::extract::<axum::http::HeaderMap>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let claims = auth::token_from_headers(&headers)
        .and_then(|t| auth::decode_token(&state.jwt_secret, &t))
        .ok_or_else(|| ServerFnError::new("Non authentifié"))?;

    let uc = grind_application::CreatePost::new(&*state.posts);
    let post = uc
        .execute(grind_domain::entities::UserId(claims.sub), &content, None)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(FeedItemDto {
        id: post.id.0,
        author_username: claims.username.clone(),
        author_display: claims.username,
        content: post.content.as_str().to_owned(),
        likes_count: 0,
        reposts_count: 0,
        replies_count: 0,
        created_at: String::new(),
    })
}

/// Point d'entrée d'hydratation côté client (WASM).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
