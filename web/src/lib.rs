//! # grind-web
//!
//! App Leptos **full-stack hydratée** (SSR + WASM). Les server functions sont
//! câblées sur les use cases/repos réels via le context Leptos (`DomainState`),
//! injecté côté serveur. Construite avec `cargo leptos build`.
//!
//! UI : réplique fidèle du design Django d'origine (thème clair « X/Twitter
//! sport », layout 3 colonnes, accent rouge #dc2626). Icônes en **SVG inline**
//! (auto-hébergées, éco) — Font Awesome (CDN) a été retiré.

use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::hooks::use_params_map;
use leptos_router::path;

use grind_shared::{
    BookmarkStateDto, CatalogDto, ConversationDto, FeedItemDto, FollowStateDto, HashtagDto,
    LikeStateDto, LoginDto, MatchPageDto, MessageDto, NotificationDto, ProfileDto, RepostStateDto,
    TeamFollowStateDto, TeamPageDto,
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

// ════════════════════════════════════════════════════════════════════════
//  Icônes SVG inline (remplacent Font Awesome). Style porté par `.ic` en CSS.
// ════════════════════════════════════════════════════════════════════════

fn ic_dumbbell() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <rect x="1" y="8" width="3" height="8" rx="1" />
            <rect x="20" y="8" width="3" height="8" rx="1" />
            <rect x="4" y="10" width="2" height="4" />
            <rect x="18" y="10" width="2" height="4" />
            <line x1="6" y1="12" x2="18" y2="12" />
        </svg>
    }
}
fn ic_home() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
            <path d="M9 22V12h6v10" />
        </svg>
    }
}
fn ic_fire() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" />
        </svg>
    }
}
fn ic_bell() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
    }
}
fn ic_mail() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <path d="M22 6l-10 7L2 6" />
        </svg>
    }
}
fn ic_bookmark() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
        </svg>
    }
}
fn ic_user() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
        </svg>
    }
}
fn ic_pen() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M12 20h9" />
            <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4z" />
        </svg>
    }
}
fn ic_heart() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
        </svg>
    }
}
fn ic_repeat() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M17 1l4 4-4 4" />
            <path d="M3 11V9a4 4 0 0 1 4-4h14" />
            <path d="M7 23l-4-4 4-4" />
            <path d="M21 13v2a4 4 0 0 1-4 4H3" />
        </svg>
    }
}
fn ic_reply() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M9 14L4 9l5-5" />
            <path d="M20 20v-7a4 4 0 0 0-4-4H4" />
        </svg>
    }
}
fn ic_share() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <circle cx="18" cy="5" r="3" />
            <circle cx="6" cy="12" r="3" />
            <circle cx="18" cy="19" r="3" />
            <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
            <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
        </svg>
    }
}
fn ic_arrow_left() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <line x1="19" y1="12" x2="5" y2="12" />
            <path d="M12 19l-7-7 7-7" />
        </svg>
    }
}
fn ic_info() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
    }
}
fn ic_search() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
    }
}
fn ic_trophy() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
            <path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
            <path d="M4 22h16" />
            <path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
            <path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
            <path d="M18 2H6v7a6 6 0 0 0 12 0V2z" />
        </svg>
    }
}
fn ic_shield() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        </svg>
    }
}
fn ic_feather() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M20.24 12.24a6 6 0 0 0-8.49-8.49L5 10.5V19h8.5z" />
            <line x1="16" y1="8" x2="2" y2="22" />
            <line x1="17.5" y1="15" x2="9" y2="15" />
        </svg>
    }
}
fn ic_logout() -> impl IntoView {
    view! {
        <svg class="ic" viewBox="0 0 24 24">
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
            <path d="M16 17l5-5-5-5" />
            <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
    }
}

/// Initiale majuscule d'un pseudo (pour l'avatar).
fn initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
}

/// Formate une date ISO (`2026-07-08T…`) en date courte FR (`8 juil.`), sans
/// dépendance runtime. Chaîne vide ou non-ISO → renvoyée telle quelle.
fn short_date(iso: &str) -> String {
    let b = iso.as_bytes();
    if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
        let mois = [
            "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
            "nov.", "déc.",
        ];
        let m: usize = iso[5..7].parse().unwrap_or(0);
        let day = iso[8..10].trim_start_matches('0');
        if (1..=12).contains(&m) {
            return format!("{day} {}", mois[m - 1]);
        }
    }
    iso.to_string()
}

// ════════════════════════════════════════════════════════════════════════
//  Shell + layout applicatif (3 colonnes)
// ════════════════════════════════════════════════════════════════════════

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
    // Actions d'authentification + état connecté (garde côté client).
    let login = ServerAction::<Login>::new();
    let register = ServerAction::<Register>::new();
    let logout = ServerAction::<Logout>::new();
    let me = Resource::new(
        move || {
            (
                login.version().get(),
                register.version().get(),
                logout.version().get(),
            )
        },
        |_| me(),
    );
    view! {
        <Stylesheet id="leptos" href="/pkg/grind.css" />
        <Title text="GRIND — Réseau social sport" />
        // ── SEO : description, robots, Open Graph, Twitter Card, thème ──
        <Meta name="description" content="GRIND, le réseau social des sportifs : publiez, suivez vos athlètes, likez et repostez. Léger, rapide et accessible." />
        <Meta name="robots" content="index, follow" />
        <Meta name="theme-color" content="#dc2626" />
        <Meta name="author" content="GRIND" />
        <Meta property="og:title" content="GRIND — Réseau social sport" />
        <Meta property="og:description" content="Le réseau social des sportifs : posts, suivis, likes et reposts en temps réel." />
        <Meta property="og:type" content="website" />
        <Meta property="og:locale" content="fr_FR" />
        <Meta property="og:site_name" content="GRIND" />
        <Meta name="twitter:card" content="summary" />
        <Meta name="twitter:title" content="GRIND — Réseau social sport" />
        <Meta name="twitter:description" content="Posts, suivis, likes et reposts sport en temps réel." />
        <Router>
            // ── Barre mobile (haut) ──
            <div class="mobile-header">
                {ic_dumbbell()}
                <span>"GRIND"</span>
            </div>

            <div class="main-container">
                // ── Sidebar gauche : logo + navigation ──
                <aside class="sidebar">
                    <div class="grind-logo">
                        {ic_dumbbell()}
                        <span>"GRIND"</span>
                    </div>
                    <nav style="flex:1;" aria-label="Navigation principale">
                        <A href="/" attr:class="nav-item">{ic_home()}<span>"Accueil"</span></A>
                        <A href="/sports" attr:class="nav-item">{ic_trophy()}<span>"Sports"</span></A>
                        <A href="/trending" attr:class="nav-item">{ic_fire()}<span>"Trending"</span></A>
                        <A href="/notifications" attr:class="nav-item">{ic_bell()}<span>"Notifications"</span></A>
                        <A href="/messages" attr:class="nav-item">{ic_mail()}<span>"Messages"</span></A>
                        {move || me.get().and_then(|r| r.ok()).flatten().map(|u| {
                            let href = format!("/u/{}", u.username);
                            view! { <A href=href attr:class="nav-item">{ic_user()}<span>"Profil"</span></A> }
                        })}
                    </nav>
                    // Carte utilisateur + déconnexion (visible si connecté)
                    <Suspense>
                        {move || me.get().and_then(|r| r.ok()).flatten().map(|u| {
                            let href = format!("/u/{}", u.username);
                            let ini = initial(&u.username);
                            let uname = u.username.clone();
                            view! {
                                <A href=href attr:class="user-card">
                                    <div class="avatar avatar-blue" style="width:36px; height:36px; font-size:14px;">{ini}</div>
                                    <span class="user-card-name">"@"{uname}</span>
                                </A>
                                <ActionForm action=logout>
                                    <button type="submit" class="nav-item" style="width:100%; border:none; background:none;">
                                        {ic_logout()}<span>"Déconnexion"</span>
                                    </button>
                                </ActionForm>
                            }
                        })}
                    </Suspense>
                    <A href="/" attr:class="post-btn">{ic_pen()}<span>"Publier"</span></A>
                </aside>

                // ── Colonne centrale : contenu de la route (toujours monté) ──
                <main class="feed-container">
                    <Routes fallback=|| view! { <div class="empty-state">"Page introuvable."</div> }>
                        <Route path=path!("/") view=Home />
                        <Route path=path!("/post/:id") view=PostDetail />
                        <Route path=path!("/u/:username") view=Profile />
                        <Route path=path!("/sports") view=Sports />
                        <Route path=path!("/trending") view=Trending />
                        <Route path=path!("/team/:slug") view=Team />
                        <Route path=path!("/match/:id") view=MatchPage />
                        <Route path=path!("/messages") view=Messages />
                        <Route path=path!("/messages/:username") view=Thread />
                        <Route path=path!("/notifications") view=Notifications />
                        <Route path=path!("/admin") view=Admin />
                    </Routes>
                </main>

                // ── Sidebar droite : recherche + trending ──
                <aside class="right-sidebar">
                    <div style="margin-bottom:24px; position:relative;">
                        <input type="text" placeholder="Rechercher un sport…" class="search-input" aria-label="Rechercher un sport" />
                    </div>
                    <div class="tag-warning">
                        {ic_info()}
                        <span>
                            <strong>"Hashtags : "</strong>
                            "utilisez #sujet pour taguer vos posts (ex : #Football, #Basketball)"
                        </span>
                    </div>
                    <h3 class="trending-title" style="margin-bottom:16px;">"Tendances sport"</h3>
                    <TrendingAside />
                </aside>
            </div>

            // ── Navigation mobile (bas) ──
            <nav class="mobile-nav" aria-label="Navigation mobile">
                <A href="/" attr:aria-label="Accueil">{ic_home()}</A>
                <A href="/trending" attr:aria-label="Trending">{ic_fire()}</A>
                <A href="/" attr:aria-label="Nouveau post">{ic_pen()}</A>
                <A href="/notifications" attr:aria-label="Notifications">{ic_bell()}</A>
                <A href="/messages" attr:aria-label="Messages">{ic_mail()}</A>
            </nav>

            // ── Garde d'authentification : overlay plein écran si déconnecté ──
            <Suspense>
                {move || {
                    let logged = matches!(me.get(), Some(Ok(Some(_))));
                    (!logged).then(|| view! { <AuthOverlay login=login register=register /> })
                }}
            </Suspense>
        </Router>
    }
}

/// Liste « trending » décorative de la sidebar droite (fidèle au design Django).
#[component]
fn TrendingAside() -> impl IntoView {
    let items = [
        ("⚽ Football", "#ChampionsLeague", "892K posts"),
        ("🏀 Basketball", "#NBA", "456K posts"),
        ("🎾 Tennis", "#Wimbledon", "234K posts"),
        ("🏈 Football", "#NFL", "567K posts"),
        ("🏒 Hockey", "#StanleyCup", "178K posts"),
    ];
    view! {
        <div>
            {items
                .into_iter()
                .map(|(cat, tag, count)| {
                    view! {
                        <A href="/trending" attr:class="trending-item">
                            <div style="font-size:13px; color:#6b7280;">{cat}</div>
                            <div class="trending-title">{tag}</div>
                            <div class="trending-subtitle">{count}</div>
                        </A>
                    }
                })
                .collect_view()}
        </div>
    }
}

// ════════════════════════════════════════════════════════════════════════
//  Pages
// ════════════════════════════════════════════════════════════════════════

/// Overlay d'authentification plein écran (affiché tant que non connecté).
/// Bascule Connexion / Créer un compte (un seul formulaire visible à la fois).
#[component]
fn AuthOverlay(
    login: ServerAction<Login>,
    register: ServerAction<Register>,
) -> impl IntoView {
    let (is_register, set_is_register) = signal(false);
    view! {
        <div class="auth-overlay" role="dialog" aria-modal="true" aria-labelledby="auth-title">
            <section class="auth-card">
                <div class="auth-logo">{ic_dumbbell()}<span>"GRIND"</span></div>
                <h1 id="auth-title" class="auth-title">
                    {move || if is_register.get() { "Créer un compte" } else { "Bon retour !" }}
                </h1>
                <p class="auth-sub">"Le réseau social des sportifs — publiez, suivez, likez, repostez."</p>

                <div class="auth-tabs" role="tablist">
                    <button type="button" role="tab" class="auth-tab" class:active=move || !is_register.get()
                        aria-selected=move || (!is_register.get()).to_string()
                        on:click=move |_| set_is_register.set(false)>"Se connecter"</button>
                    <button type="button" role="tab" class="auth-tab" class:active=move || is_register.get()
                        aria-selected=move || is_register.get().to_string()
                        on:click=move |_| set_is_register.set(true)>"Créer un compte"</button>
                </div>

                <Show when=move || !is_register.get()>
                    <ActionForm action=login>
                        <label for="login-username">"Nom d'utilisateur"</label>
                        <input id="login-username" class="field" type="text" name="username"
                            autocomplete="username" placeholder="ex : messi" required=true />
                        <label for="login-password">"Mot de passe"</label>
                        <input id="login-password" class="field" type="password" name="password"
                            autocomplete="current-password" placeholder="Votre mot de passe" required=true />
                        <button type="submit" class="btn-post" style="width:100%; margin-top:12px;">"Se connecter"</button>
                    </ActionForm>
                    {move || match login.value().get() {
                        Some(Err(e)) => view! { <p class="form-msg-err" role="alert">"Échec : "{e.to_string()}</p> }.into_any(),
                        _ => ().into_any(),
                    }}
                    <div class="auth-hint">
                        {ic_info()}
                        <span><strong>"Comptes de test : "</strong>"messi ou ronaldo · "<code>"grind1234"</code></span>
                    </div>
                </Show>

                <Show when=move || is_register.get()>
                    <ActionForm action=register>
                        <label for="reg-username">"Nom d'utilisateur"</label>
                        <input id="reg-username" class="field" type="text" name="username"
                            autocomplete="username" placeholder="lettres, chiffres, _" required=true />
                        <label for="reg-display">"Nom affiché (optionnel)"</label>
                        <input id="reg-display" class="field" type="text" name="display_name"
                            autocomplete="name" placeholder="ex : Lionel Messi" />
                        <label for="reg-password">"Mot de passe (8 caractères min)"</label>
                        <input id="reg-password" class="field" type="password" name="password"
                            autocomplete="new-password" placeholder="Choisissez un mot de passe" required=true />
                        <button type="submit" class="btn-post" style="width:100%; margin-top:12px;">"Créer mon compte"</button>
                    </ActionForm>
                    {move || match register.value().get() {
                        Some(Err(e)) => view! { <p class="form-msg-err" role="alert">"Échec : "{e.to_string()}</p> }.into_any(),
                        _ => ().into_any(),
                    }}
                </Show>
            </section>
        </div>
    }
}

#[component]
fn Home() -> impl IntoView {
    // Server actions (formulaires → server functions).
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
        <Title text="Accueil / GRIND" />
        <Meta name="description" content="GRIND — le fil d'actualité sport : posts, likes, reposts et suivis d'athlètes en temps réel." />
        // En-tête « Home »
        <div class="header-bar">
            <h1 class="header-title" style="margin-bottom:12px;">"Accueil"</h1>
            <div class="tabs" role="tablist">
                <span class="tab active" role="tab" aria-selected="true">"Tout le monde"</span>
            </div>
        </div>

        // Composer
        <div class="composer">
            <div style="display:flex; gap:12px;">
                <div class="avatar avatar-blue">{ic_pen()}</div>
                <div style="flex:1;">
                    <ActionForm action=create>
                        <input type="text" name="content" placeholder="Partagez votre actu sport…" />
                        <div style="display:flex; justify-content:flex-end; margin-top:8px;">
                            <button type="submit" class="btn-post">"Post"</button>
                        </div>
                    </ActionForm>
                </div>
            </div>
        </div>

        // Fil
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement du fil…"</div> }>
            {move || {
                timeline
                    .get()
                    .map(|res| match res {
                        Ok(items) if items.is_empty() => {
                            view! {
                                <div class="empty-state">
                                    {ic_feather()}
                                    <h3 style="font-size:22px; font-weight:700; color:#111827; margin-bottom:6px;">"No posts yet"</h3>
                                    <p>"Soyez le premier à partager quelque chose !"</p>
                                </div>
                            }
                                .into_any()
                        }
                        Ok(items) => {
                            view! {
                                <div>
                                    {items
                                        .into_iter()
                                        .map(|i| tweet_card(i, like, repost, bookmark))
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

/// Carte de post (tweet) — structure & actions fidèles au design Django.
fn tweet_card(
    i: FeedItemDto,
    like: ServerAction<ToggleLike>,
    repost: ServerAction<ToggleRepost>,
    bookmark: ServerAction<ToggleBookmark>,
) -> impl IntoView {
    let id = i.id;
    let href = format!("/post/{}", i.id);
    let ini = initial(&i.author_username);
    let like_class = if i.liked_by_me { "tweet-action action-btn liked" } else { "tweet-action action-btn" };
    let repost_class = if i.reposted_by_me { "tweet-action action-btn retweeted" } else { "tweet-action action-btn" };
    let bm_class = if i.bookmarked_by_me { "tweet-action action-btn bookmarked" } else { "tweet-action action-btn" };
    let display = if i.author_display.is_empty() { i.author_username.clone() } else { i.author_display.clone() };

    view! {
        <article class="tweet-hover">
            <div style="display:flex; gap:12px;">
                <div class="avatar">{ini}</div>
                <div style="flex:1; min-width:0;">
                    <div style="display:flex; align-items:center; gap:6px; flex-wrap:wrap;">
                        <A href=href.clone() attr:style="font-weight:700; color:#111827; text-decoration:none;">
                            {display}
                        </A>
                        <span style="color:#6b7280;">"@"{i.author_username}</span>
                        <span style="color:#6b7280;">"·"</span>
                        <span style="color:#6b7280; font-size:13px;">{short_date(&i.created_at)}</span>
                    </div>
                    <A href=href attr:style="display:block; text-decoration:none;">
                        <p style="color:#111827; margin-top:8px; line-height:1.5; word-break:break-word;">
                            {i.content}
                        </p>
                    </A>
                    <div style="display:flex; justify-content:space-between; max-width:28rem; margin-top:12px;">
                        <A href=format!("/post/{id}") attr:class="tweet-action" attr:aria-label="Répondre">
                            {ic_reply()}<span class="action-label">{i.replies_count}</span>
                        </A>
                        <ActionForm action=repost>
                            <input type="hidden" name="id" value=id />
                            <button type="submit" class=repost_class aria-label="Reposter">
                                {ic_repeat()}<span class="action-label">{i.reposts_count}</span>
                            </button>
                        </ActionForm>
                        <ActionForm action=like>
                            <input type="hidden" name="id" value=id />
                            <button type="submit" class=like_class aria-label="Aimer">
                                {ic_heart()}<span class="action-label">{i.likes_count}</span>
                            </button>
                        </ActionForm>
                        <ActionForm action=bookmark>
                            <input type="hidden" name="id" value=id />
                            <button type="submit" class=bm_class aria-label="Enregistrer">{ic_bookmark()}</button>
                        </ActionForm>
                        <span class="tweet-action" role="img" aria-label="Partager">{ic_share()}</span>
                    </div>
                </div>
            </div>
        </article>
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
        <div class="header-bar">
            <A href="/" attr:class="tweet-action" attr:style="color:#dc2626;">
                {ic_arrow_left()}<span>"Retour"</span>
            </A>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                post.get()
                    .map(|res| match res {
                        Ok((Some(p), replies)) => {
                            let parent_id = p.id;
                            let ini = initial(&p.author_username);
                            let display = if p.author_display.is_empty() { p.author_username.clone() } else { p.author_display.clone() };
                            view! {
                                <article style="padding:16px; border-bottom:1px solid #e5e7eb;">
                                    <div style="display:flex; gap:12px; margin-bottom:12px;">
                                        <div class="avatar">{ini}</div>
                                        <div>
                                            <div style="font-weight:700; color:#111827;">{display}</div>
                                            <div style="color:#6b7280;">"@"{p.author_username.clone()}</div>
                                        </div>
                                    </div>
                                    <p style="font-size:22px; font-weight:300; color:#111827; margin-bottom:16px;">
                                        {p.content.clone()}
                                    </p>
                                    <div style="display:flex; gap:24px; border-top:1px solid #e5e7eb; border-bottom:1px solid #e5e7eb; padding:16px 0; color:#6b7280;">
                                        <div><strong style="color:#111827;">{p.replies_count}</strong>" Réponses"</div>
                                        <div><strong style="color:#111827;">{p.reposts_count}</strong>" Reposts"</div>
                                        <div><strong style="color:#111827;">{p.likes_count}</strong>" Likes"</div>
                                    </div>
                                </article>
                                <div class="composer">
                                    <ActionForm action=reply>
                                        <input type="hidden" name="parent_id" value=parent_id />
                                        <input type="text" name="content" placeholder="Postez votre réponse…" />
                                        <div style="display:flex; justify-content:flex-end; margin-top:8px;">
                                            <button type="submit" class="btn-post">"Répondre"</button>
                                        </div>
                                    </ActionForm>
                                    {move || match reply.value().get() {
                                        Some(Ok(_)) => view! { <p class="form-msg-ok">"Réponse publiée."</p> }.into_any(),
                                        Some(Err(e)) => view! { <p class="form-msg-err">"Échec : "{e.to_string()}</p> }.into_any(),
                                        None => ().into_any(),
                                    }}
                                </div>
                                <div>
                                    {if replies.is_empty() {
                                        view! { <div class="empty-state">"Aucune réponse pour l'instant. Soyez le premier !"</div> }.into_any()
                                    } else {
                                        replies
                                            .into_iter()
                                            .map(|r| {
                                                let ri = initial(&r.author_username);
                                                let rd = if r.author_display.is_empty() { r.author_username.clone() } else { r.author_display.clone() };
                                                view! {
                                                    <article class="tweet-hover">
                                                        <div style="display:flex; gap:12px;">
                                                            <div class="avatar avatar-amber">{ri}</div>
                                                            <div style="flex:1; min-width:0;">
                                                                <div style="display:flex; align-items:center; gap:6px;">
                                                                    <span style="font-weight:700; color:#111827;">{rd}</span>
                                                                    <span style="color:#6b7280;">"@"{r.author_username}</span>
                                                                </div>
                                                                <p style="color:#111827; margin-top:6px;">{r.content}</p>
                                                            </div>
                                                        </div>
                                                    </article>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }
                        Ok((None, _)) => view! { <div class="empty-state">"Post introuvable."</div> }.into_any(),
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
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
    let profile = Resource::new(
        move || (username(), follow.version().get()),
        |(name, _)| async move { get_profile(name).await },
    );

    view! {
        <div class="header-bar">
            <h2 class="header-title">"@"{username}</h2>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                profile
                    .get()
                    .map(|res| match res {
                        Ok(p) => {
                            let ini = initial(&p.username);
                            let uname = p.username.clone();
                            let follow_btn = p.can_follow.then(|| {
                                let label = if p.is_following { "Ne plus suivre" } else { "Suivre" };
                                let u = p.username.clone();
                                view! {
                                    <ActionForm action=follow>
                                        <input type="hidden" name="username" value=u />
                                        <button type="submit" class="btn-post">{label}</button>
                                    </ActionForm>
                                }
                            });
                            view! {
                                <div style="padding:16px; border-bottom:1px solid #e5e7eb; display:flex; gap:16px; align-items:center;">
                                    <div class="avatar" style="width:64px; height:64px; font-size:24px;">{ini}</div>
                                    <div style="flex:1;">
                                        <div style="font-weight:800; font-size:20px; color:#111827;">"@"{uname}</div>
                                    </div>
                                    {follow_btn}
                                </div>
                                <div>
                                    {if p.posts.is_empty() {
                                        view! { <div class="empty-state">"Aucun post."</div> }.into_any()
                                    } else {
                                        p.posts
                                            .into_iter()
                                            .map(|i| {
                                                let href = format!("/post/{}", i.id);
                                                view! {
                                                    <A href=href attr:class="tweet-hover" attr:style="display:block; text-decoration:none;">
                                                        <p style="color:#111827;">{i.content}</p>
                                                        <div style="color:#6b7280; font-size:13px; margin-top:6px;">
                                                            {i.likes_count}" ❤ · "{i.replies_count}" 💬"
                                                        </div>
                                                    </A>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
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
        <div class="header-bar">
            <h2 class="header-title">{ic_shield()}" Admin — modération"</h2>
        </div>
        {move || match del.value().get() {
            Some(Ok(())) => view! { <p class="form-msg-ok" style="padding:8px 16px;">"Post supprimé."</p> }.into_any(),
            Some(Err(e)) => view! { <p class="form-msg-err" style="padding:8px 16px;">"Erreur : "{e.to_string()}</p> }.into_any(),
            None => ().into_any(),
        }}
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                list.get()
                    .map(|res| match res {
                        Ok(items) => {
                            view! {
                                <div>
                                    {items
                                        .into_iter()
                                        .map(|i| {
                                            let id = i.id;
                                            view! {
                                                <div class="tweet-hover" style="display:flex; justify-content:space-between; align-items:center; gap:12px;">
                                                    <div>
                                                        <span style="color:#6b7280;">"#"{i.id}" @"{i.author_username}" : "</span>
                                                        <span style="color:#111827;">{i.content}</span>
                                                    </div>
                                                    <ActionForm action=del>
                                                        <input type="hidden" name="id" value=id />
                                                        <button type="submit" class="tweet-action" style="color:#dc2626;">"Supprimer"</button>
                                                    </ActionForm>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Erreur (staff requis) : "{e.to_string()}</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Sports() -> impl IntoView {
    let catalog = Resource::new(|| (), |_| get_catalog());
    view! {
        <div class="header-bar">
            <h2 class="header-title">{ic_trophy()}" Sports"</h2>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                catalog
                    .get()
                    .map(|res| match res {
                        Ok(c) => {
                            view! {
                                <div style="padding:16px;">
                                    <h3 style="font-weight:700; color:#111827; margin-bottom:8px;">"Équipes"</h3>
                                    <div>
                                        {c.teams
                                            .into_iter()
                                            .map(|t| {
                                                let href = format!("/team/{}", t.slug);
                                                view! {
                                                    <A href=href attr:class="trending-item">
                                                        <div class="trending-title">{t.name}</div>
                                                        <div class="trending-subtitle">{t.country}</div>
                                                    </A>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                    <h3 style="font-weight:700; color:#111827; margin:16px 0 8px;">"Matchs"</h3>
                                    <div>
                                        {c.matches
                                            .into_iter()
                                            .map(|m| {
                                                let href = format!("/match/{}", m.id);
                                                view! {
                                                    <A href=href attr:class="trending-item">
                                                        <div class="trending-title">{m.home_team}" vs "{m.away_team}</div>
                                                        <div class="trending-subtitle">{m.status}</div>
                                                    </A>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
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
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
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
                                        <button type="submit" class="btn-post">{label}</button>
                                    </ActionForm>
                                }
                            });
                            view! {
                                <div class="header-bar">
                                    <h2 class="header-title">{ic_trophy()}" "{p.team.name.clone()}</h2>
                                </div>
                                <div style="padding:16px;">
                                    <p style="color:#6b7280; margin-bottom:12px;">"Pays : "{p.team.country.clone()}</p>
                                    {follow_btn}
                                </div>
                            }
                                .into_any()
                        }
                        Ok(None) => view! { <div class="empty-state">"Équipe introuvable."</div> }.into_any(),
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
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
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
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
                                <div class="header-bar">
                                    <h2 class="header-title">{p.game.home_team.clone()}" vs "{p.game.away_team.clone()}</h2>
                                </div>
                                <div style="padding:16px;">
                                    <p style="color:#6b7280; margin-bottom:12px;">"Score : "{score}" · "{p.game.status.clone()}</p>
                                    <div class="composer" style="padding:0 0 16px; border:none;">
                                        <ActionForm action=post>
                                            <input type="hidden" name="match_id" value=id />
                                            <input type="text" name="content" placeholder="Votre réaction live…" />
                                            <div style="display:flex; justify-content:flex-end; margin-top:8px;">
                                                <button type="submit" class="btn-post">"Publier"</button>
                                            </div>
                                        </ActionForm>
                                    </div>
                                </div>
                                <div>
                                    {p.posts
                                        .into_iter()
                                        .map(|i| {
                                            let ini = initial(&i.author_username);
                                            view! {
                                                <article class="tweet-hover">
                                                    <div style="display:flex; gap:12px;">
                                                        <div class="avatar">{ini}</div>
                                                        <div>
                                                            <span style="font-weight:700; color:#111827;">"@"{i.author_username}</span>
                                                            <p style="color:#111827; margin-top:4px;">{i.content}</p>
                                                        </div>
                                                    </div>
                                                </article>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Ok(None) => view! { <div class="empty-state">"Match introuvable."</div> }.into_any(),
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Trending() -> impl IntoView {
    let tags = Resource::new(|| (), |_| get_trending());
    view! {
        <div class="header-bar">
            <h2 class="header-title">{ic_fire()}" Trending"</h2>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                tags.get()
                    .map(|res| match res {
                        Ok(list) if list.is_empty() => {
                            view! { <div class="empty-state">"Aucun hashtag tendance pour l'instant."</div> }.into_any()
                        }
                        Ok(list) => {
                            view! {
                                <div style="padding:16px;">
                                    {list
                                        .into_iter()
                                        .map(|h| {
                                            view! {
                                                <div class="trending-item">
                                                    <div class="trending-title" style="color:#dc2626;">"#"{h.slug}</div>
                                                    <div class="trending-subtitle">{h.posts_count}" posts"</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Erreur : "{e.to_string()}</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Messages() -> impl IntoView {
    let convs = Resource::new(|| (), |_| get_conversations());
    view! {
        <div class="header-bar">
            <h2 class="header-title">{ic_mail()}" Messages"</h2>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                convs
                    .get()
                    .map(|res| match res {
                        Ok(list) if list.is_empty() => {
                            view! { <div class="empty-state">"Aucune conversation."</div> }.into_any()
                        }
                        Ok(list) => {
                            view! {
                                <div>
                                    {list
                                        .into_iter()
                                        .map(|c| {
                                            let href = format!("/messages/{}", c.other_username);
                                            let ini = initial(&c.other_username);
                                            view! {
                                                <A href=href attr:class="tweet-hover" attr:style="display:flex; gap:12px; text-decoration:none;">
                                                    <div class="avatar">{ini}</div>
                                                    <div style="flex:1; min-width:0;">
                                                        <div style="font-weight:700; color:#111827;">"@"{c.other_username}</div>
                                                        <div style="color:#6b7280; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{c.last_body}</div>
                                                    </div>
                                                </A>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Connectez-vous pour voir vos messages. ("{e.to_string()}")"</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Thread() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.read().get("username").unwrap_or_default();
    let send = ServerAction::<SendMessage>::new();
    let thread = Resource::new(
        move || (username(), send.version().get()),
        |(name, _)| async move { get_thread(name).await },
    );
    view! {
        <div class="header-bar">
            <h2 class="header-title">"Conversation avec @"{username}</h2>
        </div>
        <div class="composer">
            <ActionForm action=send>
                <input type="hidden" name="recipient" value=username />
                <input type="text" name="body" placeholder="Votre message…" />
                <div style="display:flex; justify-content:flex-end; margin-top:8px;">
                    <button type="submit" class="btn-post">"Envoyer"</button>
                </div>
            </ActionForm>
            {move || match send.value().get() {
                Some(Err(e)) => view! { <p class="form-msg-err">"Échec : "{e.to_string()}</p> }.into_any(),
                _ => ().into_any(),
            }}
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                thread
                    .get()
                    .map(|res| match res {
                        Ok(msgs) => {
                            view! {
                                <div>
                                    {msgs
                                        .into_iter()
                                        .map(|m| {
                                            let ini = initial(&m.sender_username);
                                            view! {
                                                <div class="tweet-hover" style="display:flex; gap:12px;">
                                                    <div class="avatar" style="width:36px; height:36px; font-size:14px;">{ini}</div>
                                                    <div>
                                                        <span style="font-weight:700; color:#111827;">"@"{m.sender_username}</span>
                                                        <p style="color:#111827; margin-top:2px;">{m.body}</p>
                                                    </div>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Connectez-vous. ("{e.to_string()}")"</div> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Notifications() -> impl IntoView {
    let mark = ServerAction::<MarkNotificationsRead>::new();
    let list = Resource::new(move || mark.version().get(), |_| get_notifications());
    view! {
        <div class="header-bar" style="display:flex; justify-content:space-between; align-items:center;">
            <h2 class="header-title">{ic_bell()}" Notifications"</h2>
            <ActionForm action=mark>
                <button type="submit" class="tweet-action" style="color:#dc2626;">"Tout marquer comme lu"</button>
            </ActionForm>
        </div>
        <Suspense fallback=|| view! { <div class="empty-state">"Chargement…"</div> }>
            {move || {
                list.get()
                    .map(|res| match res {
                        Ok(items) if items.is_empty() => {
                            view! { <div class="empty-state">{ic_bell()}<p>"Aucune notification."</p></div> }.into_any()
                        }
                        Ok(items) => {
                            view! {
                                <div>
                                    {items
                                        .into_iter()
                                        .map(|n| {
                                            let ini = initial(&n.actor_username);
                                            let bg = if n.is_read { "#ffffff" } else { "#fef2f2" };
                                            view! {
                                                <div class="tweet-hover" style=format!("display:flex; gap:12px; align-items:center; background:{bg};")>
                                                    <div class="avatar avatar-red" style="width:36px; height:36px; font-size:14px;">{ini}</div>
                                                    <div>
                                                        <span style="font-weight:700; color:#111827;">"@"{n.actor_username}</span>
                                                        <span style="color:#6b7280;">" — "{n.kind}</span>
                                                    </div>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <div class="empty-state">"Connectez-vous. ("{e.to_string()}")"</div> }.into_any(),
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

/// Server function : utilisateur connecté (depuis le cookie) ou `None`.
/// Sert de garde d'authentification côté client (gate).
#[server(endpoint = "me")]
pub async fn me() -> Result<Option<LoginDto>, ServerFnError> {
    let state = domain_state()?;
    let Ok(headers) = leptos_axum::extract::<axum::http::HeaderMap>().await else {
        return Ok(None);
    };
    let claims =
        auth::token_from_headers(&headers).and_then(|t| auth::decode_token(&state.jwt_secret, &t));
    Ok(claims.map(|c| LoginDto { username: c.username, is_staff: c.is_staff }))
}

/// Server function : déconnexion (efface le cookie de session).
#[server(endpoint = "logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&auth::clear_session_cookie())
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

// --- Messagerie & notifications ---

/// Server function : envoie un DM (restreint aux suivis, via `SendMessage`).
/// Résout le username destinataire → id (mapping controller).
#[server(endpoint = "send_message")]
pub async fn send_message(recipient: String, body: String) -> Result<(), ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let target = state
        .users
        .by_username(&recipient)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Destinataire introuvable"))?;

    grind_application::SendMessage::new(&*state.messages, &*state.follows, &*state.notifications)
        .execute(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::UserId(target.id),
            &body,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Server function : thread de messages entre l'utilisateur connecté et `username`.
#[server(endpoint = "get_thread")]
pub async fn get_thread(username: String) -> Result<Vec<MessageDto>, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let other = state
        .users
        .by_username(&username)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Utilisateur introuvable"))?;

    let rows = state
        .messages
        .thread(
            grind_domain::entities::UserId(claims.sub),
            grind_domain::entities::UserId(other.id),
            100,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|m| MessageDto {
            id: m.id,
            sender_username: m.sender_username,
            recipient_username: m.recipient_username,
            body: m.body,
            is_read: m.is_read,
            created_at: m.created_at,
        })
        .collect())
}

/// Server function : aperçu des conversations de l'utilisateur connecté.
#[server(endpoint = "get_conversations")]
pub async fn get_conversations() -> Result<Vec<ConversationDto>, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let rows = state
        .messages
        .conversations(grind_domain::entities::UserId(claims.sub))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|c| ConversationDto {
            other_username: c.other_username,
            last_body: c.last_body,
            created_at: c.created_at,
        })
        .collect())
}

/// Server function : notifications de l'utilisateur connecté (récentes d'abord).
#[server(endpoint = "get_notifications")]
pub async fn get_notifications() -> Result<Vec<NotificationDto>, ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    let rows = state
        .notifications
        .list(grind_domain::entities::UserId(claims.sub), 50)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|n| NotificationDto {
            id: n.id,
            kind: n.kind,
            actor_username: n.actor_username,
            is_read: n.is_read,
            created_at: n.created_at,
        })
        .collect())
}

/// Server function : marque toutes les notifications comme lues (`MarkNotificationsRead`).
#[server(endpoint = "mark_notifications_read")]
pub async fn mark_notifications_read() -> Result<(), ServerFnError> {
    let state = domain_state()?;
    let claims = require_claims(&state).await?;
    grind_application::MarkNotificationsRead::new(&*state.notifications)
        .execute(grind_domain::entities::UserId(claims.sub))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Server function : hashtags trending (public). Extraits automatiquement à la
/// création des posts depuis `PostContent::hashtags()`.
/// **Mis en cache (Redis, TTL 30s)** : lecture chaude servie sans toucher la BDD.
#[server(endpoint = "get_trending")]
pub async fn get_trending() -> Result<Vec<HashtagDto>, ServerFnError> {
    const CACHE_KEY: &str = "trending:v1";
    const TTL_SECS: u64 = 30;

    let state = domain_state()?;

    // 1. Tentative de lecture cache (best-effort).
    if let Some(cached) = state.cache.get(CACHE_KEY).await {
        if let Ok(dtos) = serde_json::from_str::<Vec<HashtagDto>>(&cached) {
            set_x_cache("HIT"); // servi depuis Redis, sans toucher la BDD
            return Ok(dtos);
        }
    }

    // 2. Cache manquant/invalide → source de vérité (BDD).
    set_x_cache("MISS"); // calcul complet (agrégation hashtags en BDD)
    let dtos: Vec<HashtagDto> = state
        .hashtags
        .trending(20)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(|h| HashtagDto { slug: h.slug, posts_count: h.posts_count })
        .collect();

    // 3. Réchauffe le cache pour les prochaines lectures (silencieux si échec).
    if let Ok(json) = serde_json::to_string(&dtos) {
        state.cache.set(CACHE_KEY, &json, TTL_SECS).await;
    }
    Ok(dtos)
}

/// Pose l'en-tête `X-Cache: HIT|MISS` sur la réponse (preuve du cache applicatif).
#[cfg(feature = "ssr")]
fn set_x_cache(status: &'static str) {
    if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
        resp.insert_header(
            axum::http::HeaderName::from_static("x-cache"),
            axum::http::HeaderValue::from_static(status),
        );
    }
}

/// Point d'entrée d'hydratation côté client (WASM).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
