//! The login page: EVE SSO is the only way in.

use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <Title text="Log in - MutaMarket"/>
        <section class="login">
            <h1>"Log in"</h1>
            <p>"MutaMarket uses EVE Online's single sign-on. No separate account needed."</p>
            <a href="/eve" class="login-button">"Log in with EVE Online"</a>
            <p class="login-alternative">
                <a href="/eve?without_scopes=true">"Log in without granting any scopes"</a>
            </p>
        </section>
    }
}
