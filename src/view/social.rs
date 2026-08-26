//! View DTOs of the characters and collections pages.

use serde::{Deserialize, Serialize};

use crate::modules::view::ModuleDetail;

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
    /// Header stats (no legacy counterpart: the page-header redesign).
    pub for_sale_count: i64,
    pub created_count: i64,
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
    /// Header stat (no legacy counterpart: the page-header redesign);
    /// sums the estimates of every module in the collection, not just
    /// the filtered page.
    pub estimated_value_total: f64,
}
