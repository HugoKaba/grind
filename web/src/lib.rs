//! # grind-web
//!
//! App Leptos **full-stack hydratée** (SSR + WASM) : démontre le pipeline
//! server functions + hydratation client. Construite avec `cargo leptos build`.
//! (Le câblage des server functions sur les use cases via context = étape suivante.)

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

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
    // Îlot interactif (hydratation client) : un compteur réactif.
    let (count, set_count) = signal(0);
    // Appel d'une server function (RPC full-stack).
    let ping = Action::new(|_: &()| async move { ping().await });

    view! {
        <h1>"🏟️ GRIND"</h1>
        <button on:click=move |_| *set_count.write() += 1>
            "Likes locaux : " {count}
        </button>
        <button on:click=move |_| { ping.dispatch(()); }>"Ping serveur"</button>
        <p>
            {move || match ping.value().get() {
                Some(Ok(msg)) => format!("Serveur : {msg}"),
                Some(Err(e)) => format!("Erreur : {e}"),
                None => "—".to_string(),
            }}
        </p>
    }
}

/// Server function : exécutée côté serveur, appelée depuis le client via RPC.
#[server]
pub async fn ping() -> Result<String, ServerFnError> {
    Ok("pong 🦀".to_string())
}

/// Point d'entrée d'hydratation côté client (WASM).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
