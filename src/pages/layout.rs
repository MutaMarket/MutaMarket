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

    let nav_link = "text-sm text-muted-foreground transition-colors hover:text-foreground";

    view! {
        <header class="border-b border-border bg-card-1">
            <nav class="mx-auto flex w-full max-w-7xl flex-wrap items-center gap-x-5 gap-y-2 px-4 py-3">
                <a href="/" class="text-base font-semibold tracking-tight">"MutaMarket"</a>
                <a href="/modules" class=nav_link>"Modules"</a>
                <a href="/all-modules" class=nav_link>"All Modules"</a>
                <a href="/characters" class=nav_link>"Characters"</a>
                <a href="/collections" class=nav_link>"Collections"</a>
                <a href="/calculator" class=nav_link>"Calculator"</a>
                <a href="/statistics" class=nav_link>"Statistics"</a>
                <Suspense fallback=|| ()>
                    {move || Suspend::new(async move {
                        match user.await {
                            Ok(Some(user)) => view! {
                                <span class="ml-auto flex items-center gap-3">
                                    <span class="text-sm">{user.name}</span>
                                    <form method="post" action="/logout">
                                        <button
                                            type="submit"
                                            class="cursor-pointer rounded-md border border-border px-3 py-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
                                        >
                                            "Log out"
                                        </button>
                                    </form>
                                </span>
                            }
                            .into_any(),
                            _ => view! {
                                <span class="ml-auto">
                                    <a
                                        href="/login"
                                        class="rounded-md border border-border px-3 py-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
                                    >
                                        "Log in"
                                    </a>
                                </span>
                            }
                            .into_any(),
                        }
                    })}
                </Suspense>
            </nav>
        </header>
        <main class="mx-auto w-full max-w-7xl flex-1 px-4 py-6">
            <Outlet/>
        </main>
        <footer class="border-t border-border">
            <p class="mx-auto w-full max-w-7xl px-4 py-4 text-xs text-muted-foreground">
                "MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online."
            </p>
        </footer>
    }
}
