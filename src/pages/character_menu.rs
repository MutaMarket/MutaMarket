//! The account character menu, mirroring the legacy
//! `AuthenticatedAsButton.vue` + character dialog: the navbar shows the
//! active character (portrait, name, switch glyph, and a warning ping when
//! any character lacks the asset scope); the dialog lists the account's
//! characters to act as, plus the add-character, corporation-scope and
//! remove actions. Built on the Rust/UI dialog and button components.

use leptos::prelude::*;

use crate::components::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuAction, DropdownMenuActionVariant, DropdownMenuAlign,
    DropdownMenuContent, DropdownMenuTrigger,
};
use crate::components::ui::separator::Separator;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AccountCharacter {
    pub id: i64,
    pub name: String,
    pub corporation_id: Option<i64>,
    pub has_asset_token: bool,
    pub active: bool,
}

/// The session user's characters with the active flag and asset-scope
/// state, like the legacy `auth.user.characters` page prop.
#[server]
pub async fn fetch_account_characters() -> Result<Vec<AccountCharacter>, ServerFnError> {
    use crate::auth::scopes;
    use crate::auth::session::session_from_headers;

    let state = expect_context::<crate::server::AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let Some(session) = session_from_headers(&state.pool, &headers).await.map_err(fail)? else {
        return Ok(Vec::new());
    };

    let rows: Vec<(i64, String, Option<i64>, bool)> = sqlx::query_as(
        "select c.id, c.name, c.corporation_id,
                exists (select 1 from esi_tokens t
                        where t.character_id = c.id and $1 = any(t.scopes)) as has_asset_token
         from characters c where c.user_id = $2 order by c.id",
    )
    .bind(scopes::READ_ASSETS)
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(fail)?;

    // The active character falls back to the first one, like the legacy
    // getActiveCharacter.
    let active_id = session
        .active_character_id
        .filter(|id| rows.iter().any(|(row_id, ..)| row_id == id))
        .or_else(|| rows.first().map(|(id, ..)| *id));

    Ok(rows
        .into_iter()
        .map(|(id, name, corporation_id, has_asset_token)| AccountCharacter {
            id,
            name,
            corporation_id,
            has_asset_token,
            active: Some(id) == active_id,
        })
        .collect())
}

/// Acts as another owned character (the PUT endpoint's logic for the
/// hydrated dialog).
#[server]
pub async fn switch_active_character(character_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session::session_from_headers;

    let state = expect_context::<crate::server::AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let Some(session) = session_from_headers(&state.pool, &headers).await.map_err(fail)? else {
        return Err(ServerFnError::new("not logged in"));
    };
    let owned: bool = sqlx::query_scalar(
        "select exists (select 1 from characters where id = $1 and user_id = $2)",
    )
    .bind(character_id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(fail)?;
    if !owned {
        return Err(ServerFnError::new("You do not own this character."));
    }

    sqlx::query("update sessions set active_character_id = $1 where token = $2")
        .bind(character_id)
        .bind(&session.token)
        .execute(&state.pool)
        .await
        .map_err(fail)?;

    Ok(())
}

/// Unlinks an owned character (the DELETE endpoint's logic), with the
/// legacy guards: never the last character, active falls back to the first
/// remaining one.
#[server]
pub async fn remove_account_character(character_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session::session_from_headers;

    let state = expect_context::<crate::server::AppState>();
    let headers: axum::http::HeaderMap = leptos_axum::extract().await?;
    let fail = |error: sqlx::Error| ServerFnError::new(error.to_string());

    let Some(session) = session_from_headers(&state.pool, &headers).await.map_err(fail)? else {
        return Err(ServerFnError::new("not logged in"));
    };
    let owned: bool = sqlx::query_scalar(
        "select exists (select 1 from characters where id = $1 and user_id = $2)",
    )
    .bind(character_id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(fail)?;
    let count: i64 = sqlx::query_scalar("select count(*) from characters where user_id = $1")
        .bind(session.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(fail)?;
    if !owned {
        return Err(ServerFnError::new("You do not have permission to remove this character."));
    }
    if count <= 1 {
        return Err(ServerFnError::new("You cannot remove your last character."));
    }

    sqlx::query("update characters set user_id = null where id = $1")
        .bind(character_id)
        .execute(&state.pool)
        .await
        .map_err(fail)?;
    if session.active_character_id == Some(character_id) {
        sqlx::query(
            "update sessions set active_character_id =
                 (select id from characters where user_id = $1 order by id limit 1)
             where token = $2",
        )
        .bind(session.user_id)
        .bind(&session.token)
        .execute(&state.pool)
        .await
        .map_err(fail)?;
    }

    Ok(())
}

fn portrait(character_id: i64) -> String {
    format!("https://images.evetech.net/characters/{character_id}/portrait?size=64")
}

/// The navbar trigger plus the character dialog. Purely prop-driven: the
/// layout fetches the characters alongside the user in one resource, so
/// hydration never races a nested suspense.
#[component]
pub fn CharacterMenu(characters: Vec<AccountCharacter>) -> AnyView {
    let Some(initial_active) = characters.iter().find(|character| character.active).map(|c| c.id)
    else {
        return ().into_any();
    };

    // Local reactive state: switching updates these directly (instant, no
    // server round-trip for the display), while the shared refresh signal
    // tells dependent pages to refetch. See the leptos-async-data skill.
    let list = RwSignal::new(characters);
    let active_id = RwSignal::new(initial_active);
    let refresh = use_context::<super::layout::ActiveCharacterRefresh>();
    let bump = move || {
        if let Some(refresh) = refresh {
            refresh.0.update(|generation| *generation += 1);
        }
    };

    let switch = Action::new(|character_id: &i64| switch_active_character(*character_id));
    let remove = Action::new(|character_id: &i64| remove_account_character(*character_id));
    let pending_switch = RwSignal::new(None::<i64>);
    let pending_remove = RwSignal::new(None::<i64>);

    Effect::new(move |_| {
        if matches!(switch.value().get(), Some(Ok(())))
            && let Some(target) = pending_switch.get_untracked()
        {
            active_id.set(target);
            pending_switch.set(None);
            bump();
        }
    });
    Effect::new(move |_| {
        if matches!(remove.value().get(), Some(Ok(())))
            && let Some(removed) = pending_remove.get_untracked()
        {
            list.update(|characters| characters.retain(|character| character.id != removed));
            // Removing the acting character falls back to the first
            // remaining one, matching the server's selection.
            if active_id.get_untracked() == removed
                && let Some(first) = list.get_untracked().first()
            {
                active_id.set(first.id);
            }
            pending_remove.set(None);
            bump();
        }
    });

    let active_character =
        move || list.get().into_iter().find(|character| character.id == active_id.get());
    let missing_scopes =
        move || list.get().iter().any(|character| !character.has_asset_token);

    view! {
        <DropdownMenu align=DropdownMenuAlign::End>
            <DropdownMenuTrigger class="relative flex h-auto items-center gap-2 border-none bg-white/[0.04] px-2 py-1.5 text-sm text-white hover:bg-white/[0.07]">
                {move || {
                    let active = active_character();
                    let id = active.as_ref().map(|c| c.id).unwrap_or(initial_active);
                    let name = active.map(|c| c.name).unwrap_or_default();
                    view! {
                        <img alt="" class="size-7 rounded" src=portrait(id)/>
                        <span class="max-w-32 truncate">{name}</span>
                    }
                }}
                <span aria-hidden="true" class="text-white/55">{"\u{21C4}"}</span>
                {move || missing_scopes().then(|| view! {
                    <span class="absolute -top-1 -right-1 size-2 animate-ping rounded-full bg-red-500"></span>
                })}
            </DropdownMenuTrigger>
            <DropdownMenuContent class="min-w-64">
                <span class="block px-2 py-1.5 text-xs font-semibold text-muted-foreground">
                    "Characters"
                </span>
                {move || {
                    let characters = list.get();
                    let removable = characters.len() > 1;
                    characters
                        .into_iter()
                        .map(|character| {
                            let id = character.id;
                            let is_active = move || active_id.get() == id;
                            let acting = is_active();

                            view! {
                                <div class="flex items-center gap-1">
                                    <DropdownMenuAction
                                        class="grow px-2 py-1.5"
                                        on:click=move |_| {
                                            if !is_active() {
                                                pending_switch.set(Some(id));
                                                switch.dispatch(id);
                                            }
                                        }
                                    >
                                        <img alt="" class="size-6 rounded" src=portrait(id)/>
                                        <span class="grow truncate">{character.name.clone()}</span>
                                        {acting.then(|| view! {
                                            <span class="text-xs text-muted-foreground">"acting"</span>
                                        })}
                                        {(!character.has_asset_token).then(|| view! {
                                            <span class="size-1.5 rounded-full bg-red-500" title="missing asset scope"></span>
                                        })}
                                    </DropdownMenuAction>
                                    {(removable && !acting)
                                        .then(|| view! {
                                            <DropdownMenuAction
                                                class="w-auto shrink-0 px-2 py-1.5 text-xs"
                                                variant=DropdownMenuActionVariant::Destructive
                                                on:click=move |_| {
                                                    pending_remove.set(Some(id));
                                                    remove.dispatch(id);
                                                }
                                            >
                                                "Remove"
                                            </DropdownMenuAction>
                                        })}
                                </div>
                            }
                        })
                        .collect_view()
                }}
                <Separator class="my-1"/>
                <DropdownMenuAction class="px-2 py-1.5" href="/eve?add_to_account=true" attr:rel="external">
                    "Add Character"
                </DropdownMenuAction>
                <DropdownMenuAction class="px-2 py-1.5" href="/eve/corporation" attr:rel="external">
                    "Add Corporation Scopes"
                </DropdownMenuAction>
                <Separator class="my-1"/>
                <form method="post" action="/logout">
                    <button
                        type="submit"
                        class="inline-flex w-full items-center gap-2 px-2 py-1.5 text-left text-sm text-destructive transition-colors hover:bg-destructive/10"
                    >
                        "Log out"
                    </button>
                </form>
            </DropdownMenuContent>
        </DropdownMenu>
    }
    .into_any()
}