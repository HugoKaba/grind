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

use grind_shared::{
    BookmarkStateDto, CatalogDto, FeedItemDto, FollowStateDto, LikeStateDto, LoginDto, MatchPageDto,
    ProfileDto, RepostStateDto, TeamFollowStateDto, TeamPageDto,
};
// DTOs uniquement nommés dans les mappers SSR (server fns) → gated pour éviter
// un warning d'import inutilisé côté client WASM.
#[cfg(feature = "ssr")]
use grind_shared::{MatchDto, SportDto, TeamDto};

#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod server;
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
                <A href="/">"Accueil"</A>" · "<A href="/sports">"Sports"</A>" · "
                <A href="/admin">"Admin"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page introuvable.".into_view()>
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/post/:id") view=PostDetail />
                    <Route path=path!("/u/:username") view=Profile />
                    <Route path=path!("/sports") view=Sports />
                    <Route path=path!("/team/:slug") view=Team />
                    <Route path=path!("/match/:id") view=MatchPage />
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
    let register = ServerAction::<Register>::new();
    let create = ServerAction::<CreatePost>::new();
    let like = ServerAction::<ToggleLike>::new();
    let repost = ServerAction::<ToggleRepost>::new();
    let bookmark = ServerAction::<ToggleBookmark>::new();

    // Le fil se recharge après toute action (publication / like / repost / bookmark).
    let timeline = Resource::new(
        move || {
            (
                create.version().get(),
                like.version().get(),
                repost.version().get(),
                bookmark.version().get(),
            )
        },
        |_| get_timeline(),
    );

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

        <h2>"Inscription"</h2>
        <ActionForm action=register>
            <input type="text" name="username" placeholder="username ([a-z0-9_])" />
            <input type="text" name="display_name" placeholder="nom affiché (optionnel)" />
            <input type="password" name="password" placeholder="mot de passe (min 8)" />
            <button type="submit">"Créer le compte"</button>
        </ActionForm>
        <p>
            {move || match register.value().get() {
                Some(Ok(u)) => format!("Compte créé et connecté : @{}", u.username),
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
                                            let id = i.id;
                                            // Libellés selon l'état persistant renvoyé par le serveur.
                                            let like_label = if i.liked_by_me { "❤ Liké" } else { "🤍 Liker" };
                                            let repost_label = if i.reposted_by_me { "🔁 Reposté" } else { "🔁 Repost" };
                                            let bm_label = if i.bookmarked_by_me { "🔖 Enregistré" } else { "🔖 Enregistrer" };
                                            view! {
                                                <li class="post">
                                                    <strong>"@"{i.author_username}</strong>
                                                    " · "
                                                    <span>{i.content}</span>
                                                    " — "
                                                    <em>{i.likes_count}" ❤ · "{i.reposts_count}" 🔁"</em>
                                                    " "
                                                    <ActionForm action=like>
                                                        <input type="hidden" name="id" value=id />
                                                        <button type="submit">{like_label}</button>
                                                    </ActionForm>
                                                    <ActionForm action=repost>
                                                        <input type="hidden" name="id" value=id />
                                                        <button type="submit">{repost_label}</button>
                                                    </ActionForm>
                                                    <ActionForm action=bookmark>
                                                        <input type="hidden" name="id" value=id />
                                                        <button type="submit">{bm_label}</button>
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

#[component]
fn PostDetail() -> impl IntoView {
    let params = use_params_map();
    let post_id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok());
    let reply = ServerAction::<Reply>::new();
    // Le thread se recharge après chaque réponse publiée (source = id + version).
    let post = Resource::new(
        move || (post_id(), reply.version().get()),
        |(maybe_id, _)| async move {
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
                            let parent_id = p.id;
                            view! {
                                <article class="post-detail">
                                    <h2>"@"{p.author_display.clone()}</h2>
                                    <p>{p.content.clone()}</p>
                                    <small>{p.likes_count}" ❤ · "{p.replies_count}" 💬"</small>
                                </article>
                                <h3>"Répondre"</h3>
                                <ActionForm action=reply>
                                    <input type="hidden" name="parent_id" value=parent_id />
                                    <input type="text" name="content" placeholder="Votre réponse…" />
                                    <button type="submit">"Répondre"</button>
                                </ActionForm>
                                <p>
                                    {move || match reply.value().get() {
                                        Some(Ok(_)) => "Réponse publiée.".to_string(),
                                        Some(Err(e)) => format!("Échec : {e}"),
                                        None => String::new(),
                                    }}
                                </p>
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
    let follow = ServerAction::<ToggleFollow>::new();
    // Le profil se recharge après chaque (dé)suivi (source = username + version).
    let profile = Resource::new(
        move || (username(), follow.version().get()),
        |(name, _)| async move { get_profile(name).await },
    );

    view! {
        <h1>"👤 @"{username}</h1>
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                profile
                    .get()
                    .map(|res| match res {
                        Ok(p) => {
                            let follow_btn = p.can_follow.then(|| {
                                let label = if p.is_following { "Ne plus suivre" } else { "Suivre" };
                                let uname = p.username.clone();
                                view! {
                                    <ActionForm action=follow>
                                        <input type="hidden" name="username" value=uname />
                                        <button type="submit">{label}</button>
                                    </ActionForm>
                                }
                            });
                            view! {
                                <div class="profile-header">{follow_btn}</div>
                                <ul class="feed">
                                    {p.posts
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
    let list = Resource::new(move || del.version().get(), |_| admin_list_posts());

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

#[component]
fn Sports() -> impl IntoView {
    let catalog = Resource::new(|| (), |_| get_catalog());
    view! {
        <h1>"🏟️ Sports"</h1>
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                catalog
                    .get()
                    .map(|res| match res {
                        Ok(c) => {
                            view! {
                                <h2>"Équipes"</h2>
                                <ul>
                                    {c.teams
                                        .into_iter()
                                        .map(|t| {
                                            let href = format!("/team/{}", t.slug);
                                            view! { <li><A href=href>{t.name}</A>" ("{t.country}")"</li> }
                                        })
                                        .collect_view()}
                                </ul>
                                <h2>"Matchs"</h2>
                                <ul>
                                    {c.matches
                                        .into_iter()
                                        .map(|m| {
                                            let href = format!("/match/{}", m.id);
                                            view! {
                                                <li>
                                                    <A href=href>{m.home_team}" vs "{m.away_team}</A>
                                                    " — "{m.status}
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
fn Team() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.read().get("slug").unwrap_or_default();
    let follow = ServerAction::<ToggleTeamFollow>::new();
    let team = Resource::new(
        move || (slug(), follow.version().get()),
        |(s, _)| async move { get_team(s).await },
    );
    view! {
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                team.get()
                    .map(|res| match res {
                        Ok(Some(p)) => {
                            let follow_btn = p.can_follow.then(|| {
                                let label = if p.is_following { "Ne plus suivre" } else { "Suivre l'équipe" };
                                let slug = p.team.slug.clone();
                                view! {
                                    <ActionForm action=follow>
                                        <input type="hidden" name="slug" value=slug />
                                        <button type="submit">{label}</button>
                                    </ActionForm>
                                }
                            });
                            view! {
                                <h1>"⚽ "{p.team.name.clone()}</h1>
                                <p>"Pays : "{p.team.country.clone()}</p>
                                <div>{follow_btn}</div>
                            }
                                .into_any()
                        }
                        Ok(None) => view! { <p>"Équipe introuvable."</p> }.into_any(),
                        Err(e) => view! { <p>"Erreur : " {e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn MatchPage() -> impl IntoView {
    let params = use_params_map();
    let match_id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok());
    let post = ServerAction::<PostAboutMatch>::new();
    let game = Resource::new(
        move || (match_id(), post.version().get()),
        |(maybe_id, _)| async move {
            match maybe_id {
                Some(id) => get_match(id).await,
                None => Ok(None),
            }
        },
    );
    view! {
        <Suspense fallback=|| view! { <p>"Chargement…"</p> }>
            {move || {
                game.get()
                    .map(|res| match res {
                        Ok(Some(p)) => {
                            let id = p.game.id;
                            let score = match (p.game.home_score, p.game.away_score) {
                                (Some(h), Some(a)) => format!("{h} - {a}"),
                                _ => "—".to_string(),
                            };
                            view! {
                                <h1>{p.game.home_team.clone()}" vs "{p.game.away_team.clone()}</h1>
                                <p>"Score : "{score}" · "{p.game.status.clone()}</p>
                                <h3>"Poster sur ce match"</h3>
                                <ActionForm action=post>
                                    <input type="hidden" name="match_id" value=id />
                                    <input type="text" name="content" placeholder="Votre réaction live…" />
                                    <button type="submit">"Publier"</button>
                                </ActionForm>
                                <h3>"Fil du match"</h3>
                                <ul class="feed">
                                    {p.posts
                                        .into_iter()
                                        .map(|i| view! { <li><strong>"@"{i.author_username}</strong>" "{i.content}</li> })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                        Ok(None) => view! { <p>"Match introuvable."</p> }.into_any(),
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
        liked_by_me: i.liked_by_me,
        reposted_by_me: i.reposted_by_me,
        bookmarked_by_me: i.bookmarked_by_me,
    }
}

/// Server function : lit le fil. Connecté → fil personnalisé (suivis + soi) ;
/// anonyme → fil récent/trending. L'observateur renseigne aussi `liked_by_me`.
#[server(endpoint = "get_timeline")]
pub async fn get_timeline() -> Result<Vec<FeedItemDto>, ServerFnError> {
    let state = domain_state()?;
    let viewer = optional_viewer(&state).await;

    let items = match viewer {
        Some(v) => state.feed.following(v, 50).await,
        None => state.feed.recent(None, 50).await,
    }
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(items.into_iter().map(to_dto).collect())
}

/// Server function : liste **tous** les posts récents pour la modération admin
/// (gated `is_staff`). Distincte de `get_timeline` (fil personnalisé), car
/// l'admin doit voir l'ensemble des posts, pas seulement ceux qu'il suit.
#[server(endpoint = "admin_list_posts")]
pub async fn admin_list_posts() -> Result<Vec<FeedItemDto>, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    if !claims.is_staff {
        return Err(ServerFnError::new("Réservé au staff"));
    }
    let items = state
        .feed
        .recent(None, 100)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(items.into_iter().map(to_dto).collect())
}

/// Server function : détail d'un post + son thread.
#[server(endpoint = "get_post_detail")]
pub async fn get_post_detail(
    id: i64,
) -> Result<(Option<FeedItemDto>, Vec<FeedItemDto>), ServerFnError> {
    let state = domain_state()?;
    let viewer = optional_viewer(&state).await;
    let post = state
        .feed
        .by_id(viewer, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map(to_dto);
    let replies = state
        .feed
        .replies(viewer, id, 100)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(to_dto)
        .collect();
    Ok((post, replies))
}

/// Server function : page profil d'un athlète (posts + état de suivi viewer-aware).
#[server(endpoint = "get_profile")]
pub async fn get_profile(username: String) -> Result<ProfileDto, ServerFnError> {
    let state = domain_state()?;
    let viewer = optional_viewer(&state).await;

    let posts: Vec<FeedItemDto> = state
        .feed
        .by_author(viewer, &username, 50)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(to_dto)
        .collect();

    // État de suivi : nécessite l'id de la cible + un observateur ≠ cible.
    let (is_following, can_follow) = match viewer {
        Some(viewer_id) => {
            let target = state
                .users
                .by_username(&username)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            match target {
                Some(t) if t.id != viewer_id.0 => {
                    let following = state
                        .follows
                        .exists(viewer_id, grind_domain::entities::UserId(t.id))
                        .await
                        .map_err(|e| ServerFnError::new(e.to_string()))?;
                    (following, true)
                }
                // profil inexistant ou soi-même → pas de bouton Suivre
                _ => (false, false),
            }
        }
        None => (false, false),
    };

    Ok(ProfileDto { username, is_following, can_follow, posts })
}

/// Server function : bascule le suivi d'un athlète (par username) pour l'utilisateur connecté.
/// Controller pur : l'orchestration follow/unfollow vit dans le use case `FollowUser`.
#[server(endpoint = "toggle_follow")]
pub async fn toggle_follow(username: String) -> Result<FollowStateDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;

    // Résolution username → id (mapping controller ; le use case parle en UserId).
    let target = state
        .users
        .by_username(&username)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Athlète introuvable"))?;

    let uc = grind_application::FollowUser::new(&*state.follows);
    let s = uc
        .toggle(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::UserId(target.id),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(FollowStateDto { target_username: username, following: s.following })
}

/// Récupère l'état serveur injecté (context Leptos) ou échoue proprement.
#[cfg(feature = "ssr")]
fn domain_state() -> Result<state::DomainState, ServerFnError> {
    use_context::<state::DomainState>()
        .ok_or_else(|| ServerFnError::new("DomainState absent du context"))
}

/// Extrait et valide les claims du cookie de session (auth par cookie).
/// Rejette si non authentifié. Utilisé par toutes les server fns protégées.
#[cfg(feature = "ssr")]
async fn require_claims(state: &state::DomainState) -> Result<auth::Claims, ServerFnError> {
    let headers = leptos_axum::extract::<axum::http::HeaderMap>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    auth::token_from_headers(&headers)
        .and_then(|t| auth::decode_token(&state.jwt_secret, &t))
        .ok_or_else(|| ServerFnError::new("Non authentifié"))
}

/// Observateur courant pour les lectures publiques : `Some(id)` si un cookie de
/// session valide est présent, `None` sinon (anonyme autorisé, pas de rejet).
#[cfg(feature = "ssr")]
async fn optional_viewer(state: &state::DomainState) -> Option<grind_domain::entities::UserId> {
    let headers = leptos_axum::extract::<axum::http::HeaderMap>().await.ok()?;
    let claims = auth::token_from_headers(&headers)
        .and_then(|t| auth::decode_token(&state.jwt_secret, &t))?;
    Some(grind_domain::entities::UserId(claims.sub))
}

/// Émet un JWT et le pose en cookie de session `Set-Cookie` sur la réponse.
/// Partagé par `login` et `register` (auto-login après inscription).
#[cfg(feature = "ssr")]
fn set_session_cookie(
    secret: &str,
    user_id: i64,
    username: &str,
    is_staff: bool,
) -> Result<(), ServerFnError> {
    let token = auth::issue_token(secret, user_id, username, is_staff);
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&auth::session_cookie(&token))
            .map_err(|e| ServerFnError::new(e.to_string()))?,
    );
    Ok(())
}

/// Server function : suppression d'un post (staff uniquement, auth par cookie).
#[server(endpoint = "delete_post")]
pub async fn delete_post(id: i64) -> Result<(), ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
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
#[server(endpoint = "login")]
pub async fn login(username: String, password: String) -> Result<LoginDto, ServerFnError> {
    let state = domain_state()?;

    let uc = grind_application::Login::new(&*state.users, &*state.hasher);
    let out = uc
        .execute(&username, &password)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Identifiants invalides"))?;

    set_session_cookie(&state.jwt_secret, out.user_id, &out.username, out.is_staff)?;
    Ok(LoginDto { username: out.username, is_staff: out.is_staff })
}

/// Server function : inscription (use case `Register`) → auto-login (cookie posé).
#[server(endpoint = "register")]
pub async fn register(
    username: String,
    password: String,
    display_name: String,
) -> Result<LoginDto, ServerFnError> {
    let state = domain_state()?;

    let uc = grind_application::Register::new(&*state.users, &*state.hasher);
    let out = uc
        .execute(&username, &password, &display_name)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Auto-login : on pose la session directement après création du compte.
    set_session_cookie(&state.jwt_secret, out.user_id, &out.username, out.is_staff)?;
    Ok(LoginDto { username: out.username, is_staff: out.is_staff })
}

/// Server function : crée un post (auth par cookie de session).
#[server(endpoint = "create_post")]
pub async fn create_post(content: String) -> Result<FeedItemDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;

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
        liked_by_me: false,
        reposted_by_me: false,
        bookmarked_by_me: false,
    })
}

/// Server function : bascule le like d'un post pour l'utilisateur connecté.
/// Controller pur : l'orchestration like/unlike vit dans le use case `ToggleLike`.
#[server(endpoint = "toggle_like")]
pub async fn toggle_like(id: i64) -> Result<LikeStateDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;

    let uc = grind_application::ToggleLike::new(&*state.likes);
    let s = uc
        .toggle(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::PostId(id),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(LikeStateDto { post_id: id, liked: s.liked, likes_count: s.likes_count })
}

/// Server function : répond à un post (crée un post enfant, `parent_id` renseigné).
/// Le compteur `replies_count` du parent est incrémenté atomiquement par le repo.
#[server(endpoint = "reply")]
pub async fn reply(parent_id: i64, content: String) -> Result<FeedItemDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;

    let post = grind_application::CreatePost::new(&*state.posts)
        .execute(
            grind_domain::entities::UserId(claims.sub),
            &content,
            Some(grind_domain::entities::PostId(parent_id)),
        )
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
        liked_by_me: false,
        reposted_by_me: false,
        bookmarked_by_me: false,
    })
}

/// Server function : bascule le repost d'un post (controller pur ; orchestration
/// dans `ToggleRepost`).
#[server(endpoint = "toggle_repost")]
pub async fn toggle_repost(id: i64) -> Result<RepostStateDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let s = grind_application::ToggleRepost::new(&*state.reposts)
        .toggle(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::PostId(id),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(RepostStateDto { post_id: id, reposted: s.reposted, reposts_count: s.reposts_count })
}

/// Server function : bascule le bookmark d'un post (privé, controller pur).
#[server(endpoint = "toggle_bookmark")]
pub async fn toggle_bookmark(id: i64) -> Result<BookmarkStateDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let s = grind_application::ToggleBookmark::new(&*state.bookmarks)
        .toggle(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::PostId(id),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(BookmarkStateDto { post_id: id, bookmarked: s.bookmarked })
}

// --- Domaine sport ---

#[cfg(feature = "ssr")]
fn team_to_dto(t: grind_application::TeamRow) -> TeamDto {
    TeamDto { id: t.id, sport_id: t.sport_id, name: t.name, slug: t.slug, country: t.country }
}

#[cfg(feature = "ssr")]
fn match_to_dto(m: grind_application::MatchRow) -> MatchDto {
    MatchDto {
        id: m.id,
        sport_id: m.sport_id,
        home_team: m.home_team,
        away_team: m.away_team,
        kickoff: m.kickoff,
        status: m.status,
        home_score: m.home_score,
        away_score: m.away_score,
    }
}

/// Server function : catalogue reference data (sports + équipes + matchs).
#[server(endpoint = "get_catalog")]
pub async fn get_catalog() -> Result<CatalogDto, ServerFnError> {
    let state = domain_state()?;
    let err = |e: grind_application::RepoError| ServerFnError::new(e.to_string());
    let sports = state
        .catalog
        .list_sports()
        .await
        .map_err(err)?
        .into_iter()
        .map(|s| SportDto { id: s.id, name: s.name, slug: s.slug })
        .collect();
    let teams = state.catalog.list_teams().await.map_err(err)?.into_iter().map(team_to_dto).collect();
    let matches = state.matches.list().await.map_err(err)?.into_iter().map(match_to_dto).collect();
    Ok(CatalogDto { sports, teams, matches })
}

/// Server function : page équipe (infos + état de suivi viewer-aware).
#[server(endpoint = "get_team")]
pub async fn get_team(slug: String) -> Result<Option<TeamPageDto>, ServerFnError> {
    let state = domain_state()?;
    let viewer = optional_viewer(&state).await;
    let Some(team) = state
        .catalog
        .team_by_slug(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Ok(None);
    };

    let (is_following, can_follow) = match viewer {
        Some(v) => {
            let following = state
                .team_follows
                .has_followed(v, grind_domain::entities::TeamId(team.id))
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            (following, true)
        }
        None => (false, false),
    };
    Ok(Some(TeamPageDto { team: team_to_dto(team), is_following, can_follow }))
}

/// Server function : bascule le suivi d'une équipe (par slug) pour l'utilisateur connecté.
#[server(endpoint = "toggle_team_follow")]
pub async fn toggle_team_follow(slug: String) -> Result<TeamFollowStateDto, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let team = state
        .catalog
        .team_by_slug(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Équipe introuvable"))?;

    let s = grind_application::FollowTeam::new(&*state.team_follows)
        .toggle(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::TeamId(team.id),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(TeamFollowStateDto { team_slug: slug, following: s.following })
}

/// Server function : page match (infos + fil des posts liés au match, live).
#[server(endpoint = "get_match")]
pub async fn get_match(id: i64) -> Result<Option<MatchPageDto>, ServerFnError> {
    let state = domain_state()?;
    let viewer = optional_viewer(&state).await;
    let Some(m) = state.matches.by_id(id).await.map_err(|e| ServerFnError::new(e.to_string()))? else {
        return Ok(None);
    };
    let posts = state
        .feed
        .by_match(viewer, id, 100)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(to_dto)
        .collect();
    Ok(Some(MatchPageDto { game: match_to_dto(m), posts }))
}

/// Server function : poste à propos d'un match (live-posting, auth par cookie).
/// Le sport est dérivé du match par le use case `PostAboutMatch`.
#[server(endpoint = "post_about_match")]
pub async fn post_about_match(match_id: i64, content: String) -> Result<(), ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    grind_application::PostAboutMatch::new(&*state.posts, &*state.matches)
        .execute(grind_domain::entities::UserId(claims.sub), &content, match_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Point d'entrée d'hydratation côté client (WASM).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
