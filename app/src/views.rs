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

fn post_line(i: FeedItem) -> impl IntoView {
    view! {
        <li class="post">
            <strong>"@"{i.author_username}</strong>
            " · "
            <span>{i.content}</span>
            " — "
            <em>{i.likes_count}" ❤ · "{i.replies_count}" 💬"</em>
        </li>
    }
}

/// Page détail d'un post + son thread de réponses (SSR).
pub fn render_post_detail(post: &FeedItem, replies: &[FeedItem]) -> String {
    let post = post.clone();
    let replies = replies.to_vec();
    let owner = Owner::new();
    let body = owner.with(|| {
        view! {
            <html lang="fr">
                <head>
                    <meta charset="utf-8"/>
                    <title>"GRIND — Post"</title>
                </head>
                <body>
                    <h1>"🏟️ GRIND"</h1>
                    <article class="post-detail">
                        <h2>"@"{post.author_display.clone()}</h2>
                        <p>{post.content.clone()}</p>
                        <small>{post.likes_count}" ❤ · "{post.replies_count}" 💬"</small>
                    </article>
                    <h3>"Réponses"</h3>
                    <ul class="thread">
                        {replies.into_iter().map(post_line).collect_view()}
                    </ul>
                </body>
            </html>
        }
        .to_html()
    });
    format!("<!DOCTYPE html>{body}")
}

/// Page profil d'un athlète + ses posts (SSR).
pub fn render_profile(username: &str, posts: &[FeedItem]) -> String {
    let username = username.to_owned();
    let posts = posts.to_vec();
    let owner = Owner::new();
    let body = owner.with(|| {
        view! {
            <html lang="fr">
                <head>
                    <meta charset="utf-8"/>
                    <title>"GRIND — @"{username.clone()}</title>
                </head>
                <body>
                    <h1>"👤 @"{username.clone()}</h1>
                    <ul class="feed">
                        {posts.into_iter().map(post_line).collect_view()}
                    </ul>
                </body>
            </html>
        }
        .to_html()
    });
    format!("<!DOCTYPE html>{body}")
}
