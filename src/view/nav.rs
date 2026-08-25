//! View DTOs of the navigation and account character menu.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
    pub active_character_id: Option<i64>,
    /// Gates the admin navigation and the `/api/admin` endpoints.
    pub is_admin: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountCharacter {
    pub id: i64,
    pub name: String,
    pub corporation_id: Option<i64>,
    pub has_asset_token: bool,
    pub active: bool,
}

/// Everything the navigation needs in one round trip, so the character
/// menu never needs a second request after the layout's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavState {
    pub user: CurrentUser,
    pub characters: Vec<AccountCharacter>,
}
