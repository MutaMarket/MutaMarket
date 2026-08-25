//! Characters and collections pages. Backend-first ports of the legacy
//! Character/Collection controllers: real data, real 404/403 semantics;
//! the visual one-to-one mirror follows with the frontend milestone.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use super::modules_page::{ModuleCard, fetch_display_settings_or_default};
pub use crate::view::social::{
    CharacterCardData, CharacterPageData, CollectionCardData, CollectionPageData,
};

#[server]
pub async fn fetch_characters(search: Option<String>) -> Result<Vec<CharacterCardData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::server::social::character_cards(&state, search.as_deref())
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[server]
pub async fn fetch_character_page(slug: String) -> Result<Option<CharacterPageData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::server::social::character_page_data(&state, &slug)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[server]
pub async fn fetch_collections(
    search: Option<String>,
) -> Result<Vec<CollectionCardData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::server::social::collection_cards(&state, search.as_deref())
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The collection page data; `Err(true)` inside the payload marks a known
/// but forbidden (private, not owner) collection, like the legacy 403.
#[server]
pub async fn fetch_collection_page(
    slug: String,
) -> Result<Result<Option<CollectionPageData>, bool>, ServerFnError> {
    use crate::auth::session::session_from_headers;
    use crate::server::social::CollectionPageOutcome;

    let state = expect_context::<crate::server::AppState>();
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
    let user_id = session_from_headers(&state.pool, &headers)
        .await
        .map_err(fail)?
        .map(|session| session.user_id);

    match crate::server::social::collection_page_data(&state, &slug, user_id).await.map_err(fail)? {
        CollectionPageOutcome::Page(page) => Ok(Ok(Some(*page))),
        CollectionPageOutcome::Forbidden => Ok(Err(true)),
        CollectionPageOutcome::NotFound => Ok(Ok(None)),
    }
}

#[cfg(feature = "ssr")]
fn set_status(status: axum::http::StatusCode) {
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(status);
    }
}

#[component]
pub fn CharactersPage() -> impl IntoView {
    let characters = OnceResource::new(fetch_characters(None));

    view! {
        <h1 class="mb-4 text-xl font-semibold">"Characters"</h1>
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading..."</p> }>
            {move || Suspend::new(async move {
                match characters.await {
                    Ok(characters) if !characters.is_empty() => view! {
                        <div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                            {characters
                                .into_iter()
                                .map(|character| {
                                    let href = format!("/characters/{}", character.slug);
                                    let portrait = format!(
                                        "https://images.evetech.net/characters/{}/portrait?size=64",
                                        character.id,
                                    );
                                    view! {
                                        <a
                                            class="flex items-center gap-3 rounded-lg border border-border bg-card-1 p-3 hover:brightness-110"
                                            href=href
                                        >
                                            <img alt="" class="size-12 rounded-lg" src=portrait/>
                                            <span>
                                                <span class="block text-sm text-white">{character.name}</span>
                                                <span class="block text-xs text-muted-foreground">
                                                    {character.modules_count.unwrap_or(0)}
                                                    " public modules"
                                                </span>
                                            </span>
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                    Ok(_) => view! {
                        <p class="text-muted-foreground">"No characters with public modules yet."</p>
                    }
                    .into_any(),
                    Err(_) => {
                        #[cfg(feature = "ssr")]
                        set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        view! { <p>"Characters are unavailable right now."</p> }.into_any()
                    }
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn CharacterPage() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.read().get("character").unwrap_or_default());

    view! {
        {move || {
            let data = OnceResource::new(fetch_character_page(slug.get()));
            view! {
                <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading..."</p> }>
                    {move || Suspend::new(async move {
                        let settings = fetch_display_settings_or_default().await;

                        match data.await {
                            Ok(Some(page)) => view! {
                                <h1 class="mb-1 text-xl font-semibold">{page.character.name.clone()}</h1>
                                <p class="mb-4 text-sm text-muted-foreground">
                                    {page.character.description.clone().unwrap_or_default()}
                                </p>
                                <div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                                    {page
                                        .modules
                                        .into_iter()
                                        .map(|module| {
                                            view! { <ModuleCard module settings=settings.clone()/> }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any(),
                            Ok(None) => {
                                #[cfg(feature = "ssr")]
                                set_status(axum::http::StatusCode::NOT_FOUND);
                                view! {
                                    <h1 class="text-xl font-semibold">"Character not found"</h1>
                                }
                                .into_any()
                            }
                            Err(_) => {
                                #[cfg(feature = "ssr")]
                                set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                                view! { <p>"This character is unavailable right now."</p> }.into_any()
                            }
                        }
                    })}
                </Suspense>
            }
        }}
    }
}

#[component]
pub fn CollectionsPage() -> impl IntoView {
    let collections = OnceResource::new(fetch_collections(None));

    view! {
        <h1 class="mb-4 text-xl font-semibold">"Collections"</h1>
        <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading..."</p> }>
            {move || Suspend::new(async move {
                match collections.await {
                    Ok(collections) if !collections.is_empty() => view! {
                        <div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                            {collections
                                .into_iter()
                                .map(|collection| {
                                    let href = format!("/collections/{}", collection.slug);
                                    view! {
                                        <a
                                            class="rounded-lg border border-border bg-card-1 p-3 hover:brightness-110"
                                            href=href
                                        >
                                            <span class="block text-sm text-white">{collection.name}</span>
                                            <span class="block text-xs text-muted-foreground">
                                                "by " {collection.character_name} " · "
                                                {collection.modules_count} " modules"
                                            </span>
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                    Ok(_) => view! {
                        <p class="text-muted-foreground">"No public collections yet."</p>
                    }
                    .into_any(),
                    Err(_) => {
                        #[cfg(feature = "ssr")]
                        set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        view! { <p>"Collections are unavailable right now."</p> }.into_any()
                    }
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn CollectionPage() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.read().get("collection").unwrap_or_default());

    view! {
        {move || {
            let data = OnceResource::new(fetch_collection_page(slug.get()));
            view! {
                <Suspense fallback=|| view! { <p class="text-muted-foreground">"Loading..."</p> }>
                    {move || Suspend::new(async move {
                        let settings = fetch_display_settings_or_default().await;

                        match data.await {
                            Ok(Ok(Some(page))) => view! {
                                <h1 class="mb-1 text-xl font-semibold">{page.collection.name.clone()}</h1>
                                <p class="mb-4 text-sm text-muted-foreground">
                                    "by " {page.collection.character_name.clone()}
                                    {page
                                        .collection
                                        .description
                                        .clone()
                                        .map(|description| format!(" · {description}"))}
                                </p>
                                <div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                                    {page
                                        .modules
                                        .into_iter()
                                        .map(|module| {
                                            view! { <ModuleCard module settings=settings.clone()/> }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any(),
                            Ok(Err(_)) => {
                                #[cfg(feature = "ssr")]
                                set_status(axum::http::StatusCode::FORBIDDEN);
                                view! {
                                    <h1 class="text-xl font-semibold">"This collection is private."</h1>
                                }
                                .into_any()
                            }
                            Ok(Ok(None)) => {
                                #[cfg(feature = "ssr")]
                                set_status(axum::http::StatusCode::NOT_FOUND);
                                view! {
                                    <h1 class="text-xl font-semibold">"Collection not found"</h1>
                                }
                                .into_any()
                            }
                            Err(_) => {
                                #[cfg(feature = "ssr")]
                                set_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                                view! { <p>"This collection is unavailable right now."</p> }.into_any()
                            }
                        }
                    })}
                </Suspense>
            }
        }}
    }
}
