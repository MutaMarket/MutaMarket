//! The account character menu, mirroring the legacy
//! `AuthenticatedAsButton.vue` + character dialog: the navbar shows the
//! active character (portrait, name, switch glyph, and a warning ping when
//! any character lacks the asset scope); the dialog lists the account's
//! characters to act as, plus the add-character, corporation-scope and
//! remove actions. Built on the Rust/UI dialog and button components.

use leptos::prelude::*;

use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::dialog::{Dialog, DialogContent, DialogTrigger};

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
    let switch = Action::new(|character_id: &i64| switch_active_character(*character_id));
    let remove = Action::new(|character_id: &i64| remove_account_character(*character_id));
    Effect::new(move |_| {
        let switched = matches!(switch.value().get(), Some(Ok(())));
        let removed = matches!(remove.value().get(), Some(Ok(())));
        if switched || removed {
            let _ = window().location().reload();
        }
    });

    let Some(active) = characters.iter().find(|character| character.active).cloned() else {
        return ().into_any();
    };
    {
        {
            {
                let missing_scopes =
                    characters.iter().any(|character| !character.has_asset_token);
                let removable = characters.len() > 1;
                let active_name = active.name.clone();
                let corporation_hint =
                    format!("Add a corporation assets token for {active_name}");

                view! {
                    <Dialog>
                        <DialogTrigger class="relative flex items-center gap-2 rounded-md bg-white/[0.04] px-2 py-1.5 text-sm text-white transition hover:bg-white/[0.07]">
                            <img alt="" class="size-7 rounded" src=portrait(active.id)/>
                            <span class="max-w-32 truncate">{active_name.clone()}</span>
                            <span aria-hidden="true" class="text-white/55">{"\u{21C4}"}</span>
                            {missing_scopes.then(|| view! {
                                <span class="absolute -top-1 -right-1 size-2 animate-ping rounded-full bg-red-500"></span>
                            })}
                        </DialogTrigger>
                        <DialogContent class="max-w-md bg-card">
                            <h3 class="mb-3 text-lg font-semibold">"Characters"</h3>
                            <div class="grid gap-2">
                                {characters
                                    .clone()
                                    .into_iter()
                                    .map(|character| {
                                        let id = character.id;
                                        let active_row = character.active;
                                        let row_class = if active_row {
                                            "grid grid-cols-[auto_1fr_auto] items-center gap-3 rounded-lg border border-primary bg-card-1 p-2 text-left"
                                        } else {
                                            "grid grid-cols-[auto_1fr_auto] items-center gap-3 rounded-lg border border-border bg-card-1 p-2 text-left hover:bg-card-2"
                                        };

                                        view! {
                                            <button
                                                class=row_class
                                                on:click=move |_| {
                                                    if !active_row {
                                                        switch.dispatch(id);
                                                    }
                                                }
                                            >
                                                <img alt="" class="size-9 rounded" src=portrait(id)/>
                                                <span class="overflow-hidden">
                                                    <span class="block truncate text-sm text-white">
                                                        {character.name.clone()}
                                                    </span>
                                                    <span class="block text-xs text-muted-foreground">
                                                        {if active_row { "Acting as" } else { "Switch to" }}
                                                        {(!character.has_asset_token)
                                                            .then_some(" \u{00B7} missing asset scope")}
                                                    </span>
                                                </span>
                                                {(removable && !active_row)
                                                    .then(|| {
                                                        view! {
                                                            <Button
                                                                variant=ButtonVariant::Destructive
                                                                class="h-7 px-2 text-xs"
                                                                on:click=move |event: leptos::ev::MouseEvent| {
                                                                    event.stop_propagation();
                                                                    remove.dispatch(id);
                                                                }
                                                            >
                                                                "Remove"
                                                            </Button>
                                                        }
                                                    })}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="mt-4 grid gap-2">
                                <a
                                    class="rounded-lg border border-border bg-card-1 p-3 text-sm text-white hover:bg-card-2"
                                    href="/eve?add_to_account=true"
                                    rel="external"
                                >
                                    "Add Character"
                                    <span class="block text-xs text-muted-foreground">
                                        "Add a new character to your account"
                                    </span>
                                </a>
                                <a
                                    class="rounded-lg border border-border bg-card-1 p-3 text-sm text-white hover:bg-card-2"
                                    href="/eve/corporation"
                                    rel="external"
                                >
                                    "Add Corporation Scopes"
                                    <span class="block text-xs text-muted-foreground">
                                        {corporation_hint}
                                    </span>
                                </a>
                            </div>
                        </DialogContent>
                    </Dialog>
                }
                .into_any()
            }
        }
    }
}
