//! Characters and collections pages. Backend-first ports of the legacy
//! Character/Collection controllers: real data, real 404/403 semantics;
//! the visual one-to-one mirror follows with the frontend milestone.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use super::modules_page::{ModuleCard, fetch_display_settings_or_default};
pub use crate::view::social::{
    CharacterCardData, CharacterPageData, CollectionCardData, CollectionPageData,
};

/// Modules shown on a character or collection page, like the legacy
/// simplePaginate(40) page size.
const SOCIAL_MODULES_PAGE_SIZE: i64 = 40;

#[cfg(feature = "ssr")]
fn character_card(view: crate::characters::CharacterView) -> CharacterCardData {
    CharacterCardData {
        id: view.id,
        slug: view.slug,
        name: view.name,
        description: view.description,
        has_premium: view.has_premium,
        corporation_id: view.corporation_id,
        modules_count: view.modules_count,
    }
}

#[server]
pub async fn fetch_characters(search: Option<String>) -> Result<Vec<CharacterCardData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::characters::characters_index(&state.pool, search.as_deref(), 1)
        .await
        .map(|characters| characters.into_iter().map(character_card).collect())
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[server]
pub async fn fetch_character_page(slug: String) -> Result<Option<CharacterPageData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let Some(id) = crate::characters::character_id_from_slug(&slug) else {
        return Ok(None);
    };
    let Some(character) = crate::characters::character_by_id(&state.pool, id).await.map_err(fail)?
    else {
        return Ok(None);
    };

    let ids =
        crate::characters::publicly_owned_module_ids(&state.pool, id, SOCIAL_MODULES_PAGE_SIZE)
            .await
            .map_err(fail)?;
    let modules = crate::modules::queries::details_for(&state.pool, &state.reference, ids)
        .await
        .map_err(fail)?;

    Ok(Some(CharacterPageData { character: character_card(character), modules }))
}

#[server]
pub async fn fetch_collections(
    search: Option<String>,
) -> Result<Vec<CollectionCardData>, ServerFnError> {
    let state = expect_context::<crate::server::AppState>();

    crate::collections::collections_index(&state.pool, search.as_deref(), 1)
        .await
        .map(|listings| {
            listings
                .into_iter()
                .map(|listing| CollectionCardData {
                    id: listing.collection.id,
                    slug: listing.collection.slug(),
                    name: listing.collection.name.clone(),
                    description: listing.collection.description.clone(),
                    visibility: listing.collection.visibility.clone(),
                    character_name: listing.character_name,
                    modules_count: listing.modules_count,
                })
                .collect()
        })
        .map_err(|error| ServerFnError::new(error.to_string()))
}

/// The collection page data; `Err(true)` inside the payload marks a known
/// but forbidden (private, not owner) collection, like the legacy 403.
#[server]
pub async fn fetch_collection_page(
    slug: String,
) -> Result<Result<Option<CollectionPageData>, bool>, ServerFnError> {
    use crate::auth::session::session_from_headers;

    let state = expect_context::<crate::server::AppState>();
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let Some(collection) =
        crate::collections::collection_by_slug(&state.pool, &slug).await.map_err(fail)?
    else {
        return Ok(Ok(None));
    };

    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
    let user_id = session_from_headers(&state.pool, &headers)
        .await
        .map_err(fail)?
        .map(|session| session.user_id);
    if !collection.viewable_by(user_id) {
        return Ok(Err(true));
    }

    let mut ids =
        crate::collections::collection_module_ids(&state.pool, collection.id).await.map_err(fail)?;
    ids.truncate(SOCIAL_MODULES_PAGE_SIZE as usize);
    let modules = crate::modules::queries::details_for(&state.pool, &state.reference, ids)
        .await
        .map_err(fail)?;

    let character_name: String =
        sqlx::query_scalar("select name from characters where id = $1")
            .bind(collection.character_id)
            .fetch_one(&state.pool)
            .await
            .map_err(fail)?;

    Ok(Ok(Some(CollectionPageData {
        collection: CollectionCardData {
            id: collection.id,
            slug: collection.slug(),
            name: collection.name.clone(),
            description: collection.description.clone(),
            visibility: collection.visibility.clone(),
            character_name,
            modules_count: modules.len() as i64,
        },
        modules,
    })))
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
