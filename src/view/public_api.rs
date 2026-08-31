//! Response DTOs of the documented public API.
//!
//! These exist so the OpenAPI description can be *derived* rather than
//! written by hand: every type here carries `ToSchema`, so the spec at
//! `/api/openapi.json` describes what the handlers actually return. A
//! field added here shows up in the spec; a field removed disappears from
//! it. Nothing to keep in sync.
//!
//! The shapes are the legacy API's, key for key — see
//! `content/docs/14-api-modules.md` and `15-api-reference.md` for the
//! prose the annotations reference.

use serde::Serialize;
use utoipa::ToSchema;

use crate::modules::view::ModuleDetail;

/// The single-resource envelope: legacy wraps one resource in `data`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleEnvelope {
    pub data: ModuleDetail,
}

/// What `GET /modules/{query}` answers: one module when the segment is an
/// id or slug, a page when it is a type-scoped query. Untagged, so the
/// generated schema is a `oneOf` of the two real shapes rather than one of
/// them standing in for both.
// Never constructed: the handlers return the two variants directly, and
// this type exists only so the generated schema is a `oneOf` of both. The
// size difference is therefore not a runtime cost.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum ModuleOrPage {
    One(ModuleEnvelope),
    Page(ModulePage),
}

/// A page of modules, with the legacy cursor-paginator's envelope.
#[derive(Debug, Serialize, ToSchema)]
pub struct ModulePage {
    pub data: Vec<ModuleDetail>,
    pub links: PageLinks,
    pub meta: PageMeta,
}

/// Legacy always emits `first` and `last` as null on a cursor paginator.
#[derive(Debug, Serialize, ToSchema)]
pub struct PageLinks {
    pub first: Option<String>,
    pub last: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageMeta {
    pub path: String,
    pub per_page: i64,
    /// Opaque: pass it back as the `cursor` query parameter. Its contents
    /// are not part of the contract.
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

/// Every error response of the public API.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub message: String,
}

/// A rejected request body: `message` repeats the first failing field.
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationError {
    pub message: String,
    /// Field name to the messages for that field.
    #[schema(additional_properties)]
    pub errors: serde_json::Map<String, serde_json::Value>,
}

/// The import request: either `message`, or both ids.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImportModuleRequest {
    /// Any string containing `showinfo:{type_id}//{item_id}`, such as an
    /// in-game item link copied out of chat.
    pub message: Option<String>,
    /// EVE type id of the mutated module.
    pub type_id: Option<i64>,
    /// EVE item id of the module.
    pub item_id: Option<i64>,
}

/// Quality metrics of one per-type price model.
#[derive(Debug, Serialize, ToSchema)]
pub struct EstimatorStatistic {
    pub id: i64,
    pub type_id: i64,
    pub name: String,
    /// Recorded sales the model was trained on.
    pub data_count: i64,
    /// Fit against the training data, 1 being perfect. Null means the
    /// model is untrained, and modules of the type carry no estimate.
    pub r2: Option<f64>,
    /// Mean absolute error, in ISK.
    pub mae: Option<f64>,
    /// Mean absolute error normalized by the mean sale price.
    pub nmae: Option<f64>,
    pub last_trained_at: Option<String>,
    /// Training sales by the source module's meta group.
    pub data_statistics: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// The roll extremes of one attribute of one abyssal type.
#[derive(Debug, Serialize, ToSchema)]
pub struct AbyssalTypeStatistic {
    pub id: i64,
    pub type_id: i64,
    pub attribute_id: i64,
    /// Whether a larger value is better. When false, `best` is smaller.
    pub high_is_good: bool,
    pub is_virtual: bool,
    /// The best possible rolled value.
    pub best: f64,
    /// The worst possible rolled value.
    pub worst: f64,
    /// Computed by MutaMarket rather than rolled by EVE.
    pub is_derived: bool,
    pub attribute: StatisticAttribute,
    pub r#type: StatisticType,
}

/// The dogma attribute of a roll range. `high_is_good` and `is_derived`
/// repeat the parent's values; the duplication is the legacy shape and is
/// kept so existing clients keep working.
#[derive(Debug, Serialize, ToSchema)]
pub struct StatisticAttribute {
    pub id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub high_is_good: bool,
    pub is_derived: bool,
    pub unit: Option<StatisticUnit>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatisticUnit {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatisticType {
    pub id: i64,
    pub name: String,
    pub meta_group: Option<String>,
    pub meta_group_id: Option<i64>,
    pub published: bool,
}
