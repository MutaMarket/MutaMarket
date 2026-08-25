//! The personal modules page, ported from the legacy
//! `PersonalModules/ShowAllPersonalModulesPage.vue` with the
//! `AssetImportStatus` component family: the user's owned modules next to
//! the asset import panel that starts an import and shows its progress.
//!
//! Deliberate divergences, until their milestones land:
//! - the legacy panel polls the import state over Inertia every two
//!   seconds; here the state is pushed over the `/ws` socket (the
//!   `AssetImportUpdated` event, see `server::ws`);
//! - the legacy full filter sidebar (search options, stats, type list) and
//!   pagination are not ported yet — the page shows the newest page of
//!   owned modules;
//! - `asset_imports.expires_at` (the "updated data will be available in…"
//!   line of the completed panel) is not tracked yet;
//! - the missing-scope error arrives as a flash notification with a CTA in
//!   legacy; flash notifications are unported, so the grant link renders
//!   inline in the panel;
//! - the legacy Start Import button posts over Inertia XHR; here it is a
//!   plain form post that redirects back, with the socket taking over the
//!   live updates.

use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use serde::Deserialize;

use super::modules_page::{ModuleCard, fetch_display_settings_or_default};
use crate::modules::view::{AssetLocationView, ModuleDetail};
pub use crate::view::personal::{AssetImportView, PersonalPageData};

/// The `/ws` event envelope, mirroring the legacy Echo delivery shape.
#[cfg(feature = "hydrate")]
#[derive(Debug, Clone, Deserialize)]
struct WsEnvelope {
    channel: String,
    event: String,
    data: Option<AssetImportView>,
}

/// The page data for the logged-in user; a guest gets `None` after the
/// response was turned into the login redirect, like the legacy auth
/// middleware.
#[server]
pub async fn fetch_personal_page() -> Result<Option<PersonalPageData>, ServerFnError> {
    use crate::auth::session::session_from_headers;
    use crate::server::AppState;

    let state = expect_context::<AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;

    let Some(session) = session_from_headers(&state.pool, &headers)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?
    else {
        leptos_axum::redirect("/login");
        return Ok(None);
    };

    crate::server::personal::personal_page_data(&state, &session)
        .await
        .map(Some)
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The user's owned modules, newest first — the legacy `whereOwnedByUser`
/// scope. Legacy reads the trigger-maintained `module_ownerships` table
/// (assets plus contract items); the same union is computed directly here
/// since the trigger table is not ported.
#[server]
pub async fn fetch_personal_modules()
-> Result<Vec<(ModuleDetail, Option<AssetLocationView>)>, ServerFnError> {
    use crate::auth::session::session_from_headers;
    use crate::server::AppState;

    let state = expect_context::<AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;

    let Some(session) = session_from_headers(&state.pool, &headers)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?
    else {
        return Ok(Vec::new());
    };

    crate::server::personal::personal_module_entries(&state, &session)
        .await
        .map(|entries| entries.into_iter().map(|entry| (entry.module, entry.location)).collect())
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[component]
pub fn PersonalModulesPage() -> impl IntoView {
    // Keyed on the active-character refresh generation so switching
    // characters refetches this page's data client-side, with no reload.
    let refresh = use_context::<super::layout::ActiveCharacterRefresh>().map(|r| r.0);
    let generation = move || refresh.map(|signal| signal.get()).unwrap_or(0);

    let page = Resource::new(generation, |_| fetch_personal_page());
    let modules = Resource::new(generation, |_| fetch_personal_modules());
    let settings = OnceResource::new(fetch_display_settings_or_default());

    view! {
        <Suspense fallback=|| {
            view! { <p class="text-muted-foreground">"Loading your modules..."</p> }
        }>
            {move || Suspend::new(async move {
                match page.await {
                    Ok(Some(data)) => {
                        let settings = settings.await;
                        let modules = modules.await.unwrap_or_default();
                        view! {
                            // The legacy title; the type-filtered variant
                            // ("Your {type}") arrives with the filters.
                            <h1 class="mb-4 text-xl font-semibold">"Your Modules"</h1>
                            <div class="my-4 flex flex-col items-start gap-4 lg:grid lg:grid-cols-[280px_1fr]">
                                <div class="w-full rounded-lg border border-border bg-card-1">
                                    <AssetImportPanel data/>
                                </div>
                                <div class="w-full">
                                    <ModuleGrid modules settings/>
                                </div>
                            </div>
                        }
                        .into_any()
                    }
                    // Guests get the login redirect, like the legacy auth
                    // middleware (302; set on the SSR response like the
                    // modules page sets its 404).
                    Ok(None) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                            response.set_status(axum::http::StatusCode::FOUND);
                            response.insert_header(
                                axum::http::header::LOCATION,
                                axum::http::HeaderValue::from_static("/login"),
                            );
                        }
                        ().into_any()
                    }
                    Err(_) => {
                        #[cfg(feature = "ssr")]
                        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
                            response.set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        }
                        view! { <p>"Your modules are unavailable right now."</p> }.into_any()
                    }
                }
            })}
        </Suspense>
    }
}

#[component]
fn ModuleGrid(
    modules: Vec<(ModuleDetail, Option<AssetLocationView>)>,
    settings: crate::modules::view::DisplaySettings,
) -> impl IntoView {
    if modules.is_empty() {
        return view! {
            <p class="text-muted-foreground">"No owned modules yet - import your assets to see them here."</p>
        }
        .into_any();
    }

    view! {
        <div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
            {modules
                .into_iter()
                .map(|(module, asset)| {
                    match asset {
                        Some(asset) => view! {
                            <ModuleCard module settings=settings.clone() asset/>
                        }
                        .into_any(),
                        None => view! { <ModuleCard module settings=settings.clone()/> }.into_any(),
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// The asset import panel, the legacy `AssetImportStatus.vue`: current
/// import state, live progress, and the Start Import button.
#[component]
fn AssetImportPanel(data: PersonalPageData) -> impl IntoView {
    let import = RwSignal::new(data.asset_import.clone());
    let user_id = data.user_id;

    // Live updates over the user's private event stream.
    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        subscribe_to_import_updates(user_id, import);
        #[cfg(not(feature = "hydrate"))]
        let _ = (user_id, &import);
    });

    let is_active = move || {
        import
            .get()
            .is_some_and(|current| current.status != "completed" && current.status != "failed")
    };

    view! {
        <div class="p-4">
            <h2 class="mb-2">"Asset Import"</h2>
            <div>
                {move || match import.get() {
                    None => view! { <NoAssetsImported/> }.into_any(),
                    Some(current) => view! { <ImportState current/> }.into_any(),
                }}
                <div class="">
                    <Show when=move || !is_active()>
                        {if data.has_assets_scope {
                            view! {
                                <form method="post" action="/personal/modules">
                                    <button
                                        type="submit"
                                        class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                                    >
                                        "Start Import"
                                    </button>
                                </form>
                            }
                                .into_any()
                        } else {
                            // The legacy missing-scope notification, inlined.
                            view! {
                                <div class="text-muted-foreground my-2 text-sm">
                                    <p>"You need to grant the \"Read Assets\" ESI scope to import your personal modules."</p>
                                </div>
                                <a
                                    href=data.grant_scope_url.clone()
                                    rel="external"
                                    class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                                >
                                    "Grant ESI scope"
                                </a>
                            }
                                .into_any()
                        }}
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Opens the `/ws` stream and feeds `AssetImportUpdated` events for this
/// user's channel into the signal.
#[cfg(feature = "hydrate")]
fn subscribe_to_import_updates(user_id: i64, import: RwSignal<Option<AssetImportView>>) {
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::wasm_bindgen::closure::Closure;

    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let (Ok(protocol), Ok(host)) = (location.protocol(), location.host()) else {
        return;
    };
    let scheme = if protocol == "https:" { "wss" } else { "ws" };

    let Ok(socket) = web_sys::WebSocket::new(&format!("{scheme}://{host}/ws")) else {
        return;
    };

    let channel = format!("Users.{user_id}");
    let onmessage = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
        let Some(text) = event.data().as_string() else {
            return;
        };
        let Ok(envelope) = serde_json::from_str::<WsEnvelope>(&text) else {
            return;
        };
        if envelope.channel == channel && envelope.event == "AssetImportUpdated" {
            import.set(envelope.data);
        }
    });
    socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    // The closure lives as long as the page; the socket closes with it.
    onmessage.forget();
}

/// Legacy `StepDescription.vue`.
#[component]
fn StepDescription(children: Children) -> impl IntoView {
    view! { <div class="text-muted-foreground my-2 text-sm">{children()}</div> }
}

/// Legacy `NoAssetsImported.vue` — including its "Your have" typo.
#[component]
fn NoAssetsImported() -> impl IntoView {
    view! {
        <StepDescription>
            <p>"Your have not imported any assets yet. Click the button below to start your first import."</p>
        </StepDescription>
    }
}

#[component]
fn ImportState(current: AssetImportView) -> impl IntoView {
    match current.status.as_str() {
        // Legacy PendingAssetImport.vue.
        "pending" => view! {
            <StepDescription>
                <p class="animate-pulse">"Your asset import has been queued. This may take a few minutes."</p>
            </StepDescription>
        }
        .into_any(),
        "processing" => view! { <ProcessingImport current/> }.into_any(),
        // Legacy CompletedAssetImport.vue (the expires_at line is not
        // ported: the column is not tracked yet).
        "completed" => {
            let count = current.abyssal_modules_imported_count;
            let ago = distance_strict(current.updated_seconds_ago);
            view! {
                <StepDescription>
                    <p>{format!("We successfully imported {count} modules from your assets {ago} ago.")}</p>
                </StepDescription>
            }
            .into_any()
        }
        // Legacy FailedAssetImport.vue.
        "failed" => {
            let action = failed_action(&current.step);
            view! {
                <StepDescription>
                    <p>{format!("Your import failed while we were trying to {action}.")}</p>
                </StepDescription>
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

/// Legacy ProcessingAssetImport.vue: spinner, the step description, and
/// the module import progress bar (without the tweened count animation).
#[component]
fn ProcessingImport(current: AssetImportView) -> impl IntoView {
    let step_text = match current.step.as_str() {
        "fetching_assets" => Some("Fetching assets from ESI"),
        "fetching_asset_names" => Some("Fetching asset names from ESI"),
        "fetching_corporation_assets" => Some("Fetching corporation assets from ESI"),
        "fetching_corporation_asset_names" => Some("Fetching corporation asset names from ESI"),
        "searching_abyssal_modules" => Some("Searching for abyssal modules"),
        _ => None,
    };

    view! {
        <div class="grid grid-cols-[auto_1fr] items-center gap-2 text-sm">
            <svg class="size-3 animate-spin" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"></path>
            </svg>
            {match step_text {
                Some(text) => view! { <p class="col-start-2 row-start-1">{text}</p> }.into_any(),
                None => {
                    let imported = current.abyssal_modules_imported_count;
                    let total = current.abyssal_modules_count;
                    let percent = if total > 0 {
                        (imported as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    view! {
                        <div class="col-start-2 row-start-1">
                            <p>{format!("Importing abyssal modules {imported}/{total}")}</p>
                            <div class="bg-card mt-2 h-1 rounded-full">
                                <div
                                    class="bg-primary h-1 rounded-full transition-[width] duration-1000"
                                    style=format!("width: {percent}%")
                                ></div>
                            </div>
                        </div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

/// The failed-step wording of the legacy FailedAssetImport.vue.
fn failed_action(step: &str) -> &'static str {
    match step {
        "fetching_assets" => "fetch your assets from ESI",
        "fetching_asset_names" => "fetch your asset names from ESI",
        "fetching_corporation_assets" => "fetch your corporation assets from ESI",
        "fetching_corporation_asset_names" => "fetch your corporation asset names from ESI",
        "searching_abyssal_modules" => "search for abyssal modules",
        "importing_abyssal_modules" => "import abyssal modules",
        _ => "import your assets",
    }
}

/// An approximation of date-fns `formatDistanceToNowStrict`, as used by
/// the completed panel.
fn distance_strict(seconds: i64) -> String {
    let seconds = seconds.max(0);

    let (amount, unit) = if seconds < 60 {
        (seconds, "second")
    } else if seconds < 3600 {
        (seconds / 60, "minute")
    } else if seconds < 86_400 {
        (seconds / 3600, "hour")
    } else {
        (seconds / 86_400, "day")
    };

    if amount == 1 {
        format!("1 {unit}")
    } else {
        format!("{amount} {unit}s")
    }
}

#[cfg(test)]
mod tests {
    use super::{distance_strict, failed_action};

    #[test]
    fn distances_match_the_date_fns_strict_format() {
        assert_eq!(distance_strict(0), "0 seconds");
        assert_eq!(distance_strict(1), "1 second");
        assert_eq!(distance_strict(59), "59 seconds");
        assert_eq!(distance_strict(60), "1 minute");
        assert_eq!(distance_strict(3 * 60), "3 minutes");
        assert_eq!(distance_strict(3600), "1 hour");
        assert_eq!(distance_strict(2 * 86_400), "2 days");
    }

    #[test]
    fn failed_actions_carry_the_legacy_wording() {
        assert_eq!(failed_action("fetching_assets"), "fetch your assets from ESI");
        assert_eq!(failed_action("importing_abyssal_modules"), "import abyssal modules");
        assert_eq!(failed_action("anything else"), "import your assets");
    }
}
