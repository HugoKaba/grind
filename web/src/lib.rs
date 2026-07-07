//! # grind-web
//!
//! App Leptos **full-stack hydratée** (SSR + WASM). Les server functions sont
//! câblées sur les use cases/repos réels via le context Leptos (`DomainState`),
//! injecté côté serveur. Construite avec `cargo leptos build`.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

use grind_shared::FeedItemDto;

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
            <main>
                <Routes fallback=|| "Page introuvable.".into_view()>
                    <Route path=StaticSegment("") view=Home />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn Home() -> impl IntoView {
    // Îlot interactif (hydratation client).
    let (count, set_count) = signal(0);
    // Données réelles chargées via server function (SSR puis hydratées).
    let timeline = Resource::new(|| (), |_| get_timeline());

    view! {
        <h1>"🏟️ GRIND"</h1>
        <button on:click=move |_| *set_count.write() += 1>"Likes locaux : " {count}</button>

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

    Ok(items
        .into_iter()
        .map(|i| FeedItemDto {
            id: i.id,
            author_username: i.author_username,
            author_display: i.author_display,
            content: i.content,
            likes_count: i.likes_count,
            reposts_count: i.reposts_count,
            replies_count: i.replies_count,
            created_at: i.created_at,
        })
        .collect())
}

/// Point d'entrée d'hydratation côté client (WASM).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
