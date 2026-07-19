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

/// Everything the navigation needs in one round trip, so the character
/// menu never nests a second resource inside the layout's suspense.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavState {
    pub user: CurrentUser,
    pub characters: Vec<super::character_menu::AccountCharacter>,
}

#[server]
pub async fn fetch_nav_state() -> Result<Option<NavState>, ServerFnError> {
    let Some(user) = get_current_user().await? else {
        return Ok(None);
    };
    let characters = super::character_menu::fetch_account_characters().await?;

    Ok(Some(NavState { user, characters }))
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
    let user = OnceResource::new(fetch_nav_state());

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
                            // The logged-in branch carries the legacy
                            // "My modules" entry and the character menu.
                            Ok(Some(state)) => view! {
                                <span class="ml-auto flex items-center gap-3">
                                    <a
                                        href="/personal/modules"
                                        class="text-sm text-muted-foreground transition-colors hover:text-foreground"
                                    >
                                        "My modules"
                                    </a>
                                    {super::character_menu::CharacterMenu(
                                        super::character_menu::CharacterMenuProps {
                                            characters: state.characters.clone(),
                                        },
                                    )}
                                    <span class="hidden">{state.user.name.clone()}</span>
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
