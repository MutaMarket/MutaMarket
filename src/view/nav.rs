//! View DTOs of the navigation and account character menu.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
    pub active_character_id: Option<i64>,
    /// Gates the admin navigation and the `/api/admin` endpoints.
    pub is_admin: bool,
    /// Any of the account's characters has active premium (the legacy
    /// `User::hasPremium`); gates the similar-sold tab.
    pub has_premium: bool,
    /// A premium account's custom accent color (`#rrggbb`), retinting the
    /// theme; `None` for non-premium accounts and the default lime.
    pub accent_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountCharacter {
    pub id: i64,
    pub name: String,
    pub corporation_id: Option<i64>,
    pub has_asset_token: bool,
    pub active: bool,
    /// Every scope the character's tokens carry together, so the menu
    /// and the settings summary can name what is missing.
    pub granted_scopes: Vec<String>,
    /// The user silenced this character's missing-scope warnings.
    pub scope_warnings_muted: bool,
}

/// One requestable ESI scope with its user-facing wording, the
/// vocabulary of the settings access summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub optional: bool,
}

/// A prize drawn for the session user and not yet claimed or declined,
/// the legacy `RaffleData` shared prop feeding the site-wide dialog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RafflePrize {
    pub id: i64,
    pub status: i32,
    pub expires_at: Option<String>,
    pub r#type: Option<crate::modules::view::TypeRef>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

/// Everything the navigation needs in one round trip, so the character
/// menu never needs a second request after the layout's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavState {
    pub user: CurrentUser,
    pub characters: Vec<AccountCharacter>,
    /// The legacy `raffle` shared prop; null unless a prize awaits.
    pub raffle: Option<RafflePrize>,
    /// Static scope vocabulary, so the character menu can name what a
    /// character is missing without a second request.
    pub scope_catalogue: Vec<ScopeInfo>,
}
