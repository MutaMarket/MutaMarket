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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionCardData {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub character_name: String,
    pub modules_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionPageData {
    pub collection: CollectionCardData,
    pub modules: Vec<ModuleDetail>,
}
