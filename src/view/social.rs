//! View DTOs of the characters and collections pages.

use serde::{Deserialize, Serialize};

use crate::modules::view::{CharacterLocationView, ModuleDetail, ScopedModuleStats};

/// One index page, the legacy `paginate(n)` resource collection reduced
/// to the members the pages read: the cards plus the meta the pagination
/// buttons need.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexPage<T> {
    pub data: Vec<T>,
    pub meta: IndexMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMeta {
    pub current_page: i64,
    pub per_page: i64,
    pub total: i64,
    pub last_page: i64,
}

impl IndexMeta {
    pub fn new(current_page: i64, per_page: i64, total: i64) -> Self {
        Self {
            current_page,
            per_page,
            total,
            // Laravel reports one page for an empty set.
            last_page: ((total + per_page - 1) / per_page).max(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCardData {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub has_premium: bool,
    pub corporation_id: Option<i64>,
    pub modules_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterPageData {
    pub character: CharacterCardData,
    pub modules: Vec<ModuleDetail>,
    /// Header counts over both of the character's sets, whichever is
    /// listed (no legacy counterpart: the page-header redesign).
    pub for_sale_count: i64,
    pub created_count: i64,
    /// Totals over the listed set (listings, or creations under the
    /// `created` option), the legacy CharacterModuleStats.
    pub stats: ScopedModuleStats,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionCardData {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub character_id: i64,
    pub character_name: String,
    pub character_has_premium: bool,
    pub modules_count: i64,
    /// Distinct module types of the collection (most frequent first,
    /// capped) for the card's icon strip.
    pub type_ids: Vec<i64>,
    pub types_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionPageData {
    pub collection: CollectionCardData,
    pub modules: Vec<ModuleDetail>,
    /// Totals over the whole collection, not just the filtered page
    /// (the legacy CollectionStats).
    pub stats: ScopedModuleStats,
    /// The legacy CollectionResource auto_sync/last_synced_at pair
    /// (carried on the page payload instead of every card).
    pub auto_sync: bool,
    pub last_synced_at: Option<String>,
    /// Owner-only (None for other viewers): the auto-sync tracked
    /// locations, the legacy whenLoaded('collectionLocations').
    pub tracked_locations: Option<Vec<CharacterLocationView>>,
    /// Owner-only: the collection character's asset locations holding
    /// abyssal modules (the legacy getLocationsIfAuthorized).
    pub locations: Option<Vec<CharacterLocationView>>,
}
