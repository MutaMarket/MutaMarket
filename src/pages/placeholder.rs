//! Placeholder for pages whose feature milestone has not landed yet in the
//! rewrite: the route exists and renders, the content is on its way.

use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn PlaceholderPage(title: &'static str) -> impl IntoView {
    view! {
        <Title text=format!("{title} - MutaMarket")/>
        <section class="placeholder">
            <h1>{title}</h1>
            <p>"This part of MutaMarket is being rebuilt and will be back shortly."</p>
        </section>
    }
}
