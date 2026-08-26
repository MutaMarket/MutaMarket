//! View DTOs of the personal modules page.

use serde::{Deserialize, Serialize};

use crate::modules::view::{AssetLocationView, ModuleDetail};

/// One `asset_imports` row as shown to the user — the shape the legacy
/// page receives as its `asset_import` Inertia prop (minus the timestamps,
/// replaced by the age the completed panel needs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetImportView {
    pub id: i64,
    pub character_id: i64,
    pub status: String,
    pub step: String,
    pub assets_count: i64,
    pub assets_corporation_count: i64,
    pub abyssal_modules_count: i64,
    pub abyssal_modules_imported_count: i64,
    pub abyssal_modules_failed_count: i64,
    /// Age of the last update, for "… {timeAgo} ago" (the legacy panel
    /// recomputes this client-side every second; here it refreshes with
    /// every pushed update).
    pub updated_seconds_ago: i64,
}

/// One owned module of the personal grid with its asset location, when
/// the module sits in an imported asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalModuleEntry {
    pub module: ModuleDetail,
    pub location: Option<AssetLocationView>,
}

/// Everything the page needs about the logged-in user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalPageData {
    pub user_id: i64,
    /// Whether the active character holds the Read Assets scope (the
    /// legacy store action's precondition).
    pub has_assets_scope: bool,
    /// The legacy notification CTA target: the EVE login requesting the
    /// missing scope.
    pub grant_scope_url: String,
    pub asset_import: Option<AssetImportView>,
    /// Header stats (no legacy counterpart: the page-header redesign):
    /// the account's whole owned set, unaffected by page filters.
    pub modules_count: i64,
    pub estimated_value_total: f64,
}

/// One container row of the sell page's select-modules dialog: an asset
/// of the active character with abyssal descendants and its published
/// state (the legacy Character::locations()).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SellLocation {
    pub asset_id: i64,
    pub type_id: i64,
    pub name: String,
    /// The container's type name, for the containers-first sort (the
    /// legacy couldBeContainer name check).
    pub type_name: String,
    pub location_flag: String,
    pub abyssal_count: i64,
    /// Set when the container is currently published.
    pub public_asset_id: Option<i64>,
}

/// The sell page header payload.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SellPageData {
    pub character_id: i64,
    pub published_count: i64,
    pub estimated_value_total: f64,
}
