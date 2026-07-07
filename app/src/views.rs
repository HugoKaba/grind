//! Vues Leptos rendues côté serveur (SSR → string via `RenderHtml::to_html`).
//! L'hydratation client full-WASM via `cargo-leptos` + `#[server]` = incrément final.

use leptos::prelude::*;
use leptos::reactive::owner::Owner;

use grind_application::FeedItem;

/// Rend la page timeline en HTML (SSR), dans un scope réactif `Owner`.
pub fn render_timeline(items: &[FeedItem]) -> String {
    let list = items.to_vec();
    let owner = Owner::new();
    let body = owner.with(|| {
        view! {
            <html lang="fr">
                <head>
                    <meta charset="utf-8"/>
                    <title>"GRIND — Timeline"</title>
                </head>
                <body>
                    <h1>"🏟️ GRIND"</h1>
                    <ul class="feed">
                        {list
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
                </body>
            </html>
        }
        .to_html()
    });
    format!("<!DOCTYPE html>{body}")
}
