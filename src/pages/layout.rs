//! The shared page frame: navigation with the login state, the routed page
//! content, and the footer.

use leptos::prelude::*;
use leptos_router::components::Outlet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
    pub active_character_id: Option<i64>,
}

/// The logged-in user of the request's session cookie, if any.
#[server]
pub async fn get_current_user() -> Result<Option<CurrentUser>, ServerFnError> {
    use crate::auth::session::session_from_headers;
    use crate::server::AppState;

    let state = expect_context::<AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;

    let Some(session) = session_from_headers(&state.pool, &headers)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?
    else {
        return Ok(None);
    };

    let name: Option<String> = sqlx::query_scalar("select name from users where id = $1")
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;

    Ok(name.map(|name| CurrentUser {
        name,
        active_character_id: session.active_character_id,
    }))
}

#[component]
pub fn Layout() -> impl IntoView {
    let user = OnceResource::new(get_current_user());

    view! {
        <header class="site-header">
            <nav>
                <a href="/" class="brand">"MutaMarket"</a>
                <a href="/modules">"Modules"</a>
                <a href="/all-modules">"All Modules"</a>
                <a href="/characters">"Characters"</a>
                <a href="/collections">"Collections"</a>
                <a href="/calculator">"Calculator"</a>
                <a href="/statistics">"Statistics"</a>
                <Suspense fallback=|| ()>
                    {move || Suspend::new(async move {
                        match user.await {
                            Ok(Some(user)) => view! {
                                <span class="auth">
                                    <span class="user-name">{user.name}</span>
                                    <form method="post" action="/logout" class="logout">
                                        <button type="submit">"Log out"</button>
                                    </form>
                                </span>
                            }
                            .into_any(),
                            _ => view! {
                                <span class="auth">
                                    <a href="/login">"Log in"</a>
                                </span>
                            }
                            .into_any(),
                        }
                    })}
                </Suspense>
            </nav>
        </header>
        <main>
            <Outlet/>
        </main>
        <footer class="site-footer">
            <p>"MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online."</p>
        </footer>
    }
}
