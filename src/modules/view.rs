//! Presentation types and pure helpers for modules, shared by the JSON API
//! and the Leptos pages (and therefore compiled for the browser as well).

use serde::{Deserialize, Serialize};

/// A module as the legacy `ModuleResource` emits it for guests with the
/// default relations loaded: same field names, same nesting. Keys owned by
/// unported features (`contract`, `public_asset`, estimator values) are
/// present and null, like the legacy loaded-but-empty relations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModuleDetail {
    pub id: i64,
    pub r#type: TypeRef,
    pub creator: Option<CharacterRef>,
    pub mutated_attributes: Vec<ModuleAttributeView>,
    pub source_type: Option<SourceTypeRef>,
    pub mutaplasmid: Option<MutaplasmidRef>,
    /// The module's latest public sale contract, if any.
    pub contract: Option<ContractRef>,
    pub estimated_value: Option<f64>,
    pub estimated_value_updated_at: Option<String>,
    /// The MutaMarket sell listing; arrives with the assets milestone.
    pub public_asset: Option<serde_json::Value>,
    pub slug: String,
    pub average_fraction: Option<f64>,
    /// The recorded historic sale (legacy `whenLoaded('trainingModule')`):
    /// present only on the historic-sales cards, absent elsewhere.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub training_module: Option<TrainingRef>,
    /// The signed-in user's note (legacy `withUserNote` in the default
    /// relations): the key exists only with a session, and is null when
    /// the user has no note on the module. Absent for guests.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<Option<NoteRef>>,
    /// The per-collection note (legacy `withCollectionNote`): present for
    /// every viewer of a collection page, absent elsewhere.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub collection_note: Option<Option<CollectionNoteRef>>,
    /// Where the module sits in the signed-in user's assets (the legacy
    /// `withUserAsset` half of `withDefaultRelations`, loaded on every
    /// module list): present-and-null when they do not own it, absent
    /// for guests.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub asset: Option<Option<AssetLocationView>>,
}

/// Legacy `NoteResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NoteRef {
    pub id: i64,
    pub content: String,
}

/// Legacy `CollectionNoteResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CollectionNoteRef {
    pub collection: NoteCollectionRef,
    pub id: i64,
    pub content: String,
}

/// Legacy `CollectionResource` as embedded in a collection note: only the
/// collection's own columns (no counted or loaded relations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NoteCollectionRef {
    pub id: i64,
    pub identifier: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    pub auto_sync: bool,
    pub last_synced_at: Option<String>,
}

/// Legacy `TrainingModuleResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrainingRef {
    pub contract_id: i64,
    pub sold_for: Option<f64>,
    pub sold_at: Option<String>,
}

/// Legacy `ModuleTypeResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TypeRef {
    pub id: i64,
    pub name: String,
}

/// Legacy `MutaplasmidResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MutaplasmidRef {
    pub id: i64,
    pub name: String,
}

/// Legacy `TypeResource` as loaded for the source type (meta group loaded,
/// type attributes and meta level not).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SourceTypeRef {
    pub id: i64,
    pub name: String,
    pub meta_group: Option<String>,
    pub meta_group_id: Option<i64>,
    pub published: bool,
}

/// Legacy `CharacterResource` without the user-conditional fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterRef {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub has_premium: bool,
    pub corporation_id: Option<i64>,
}

/// Legacy `UnitResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UnitRef {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

/// Legacy `ContractResource` as loaded for a module's latest public
/// contract (issuer loaded; modules/types/acceptor/status absent for
/// public contracts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractRef {
    pub id: i64,
    /// `auction` or `item_exchange`.
    pub r#type: String,
    /// The unified price (auction bids and asked-for PLEX included).
    pub price: Option<f64>,
    pub asking_for_items: bool,
    pub plex_count: i64,
    pub non_abyssal_modules_count: i64,
    pub abyssal_modules_count: i64,
    pub issuer: Option<CharacterRef>,
    pub date_issued: Option<String>,
    pub date_expired: Option<String>,
}

/// Legacy `MutatedAttributeResource`, plus the server-computed `type_band`
/// the legacy frontend derives from its bundled static data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModuleAttributeView {
    #[serde(rename = "id")]
    pub attribute_id: i64,
    pub name: String,
    pub display_name: String,
    pub value: f64,
    pub base_value: f64,
    pub fraction: f64,
    pub fraction_type: f64,
    pub fraction_absolute: f64,
    pub bar: i16,
    pub is_derived: bool,
    pub unit: Option<UnitRef>,
    pub is_virtual: bool,
    /// The mutaplasmid's own share of the type-wide roll range, as (min,
    /// max) half-width fractions — the highlight band of the
    /// type-normalized bar. Absent when the mutaplasmid covers the whole
    /// range.
    pub type_band: Option<(f64, f64)>,
}

/// The per-visitor display preferences, from the legacy display cookies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DisplaySettings {
    /// `grid`, `list` or `table`.
    pub display: String,
    /// `default`, `type`, `absolute` or `none`.
    pub attribute_bar_mode: String,
    pub show_attribute_scores: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            display: "grid".to_owned(),
            attribute_bar_mode: "default".to_owned(),
            show_attribute_scores: false,
        }
    }
}

/// Valid values of the `display` setting.
pub const DISPLAY_VALUES: [&str; 3] = ["grid", "list", "table"];

/// Valid values of the `attribute_bar_mode` setting.
pub const ATTRIBUTE_BAR_MODES: [&str; 4] = ["default", "type", "absolute", "none"];

impl ModuleAttributeView {
    fn unit_name(&self) -> Option<&str> {
        self.unit.as_ref().map(|unit| unit.name.as_str())
    }

    fn unit_display_name(&self) -> Option<&str> {
        self.unit.as_ref().map(|unit| unit.display_name.as_str())
    }

    /// Rolled value with its unit, e.g. `12.5HP/s`.
    pub fn formatted_value(&self) -> String {
        format_value(self.value, self.unit_name(), self.unit_display_name())
    }

    /// Signed difference against the base value, e.g. `+1.2s`.
    pub fn formatted_difference(&self) -> String {
        format_difference(
            self.value,
            self.base_value,
            self.unit_name(),
            self.unit_display_name(),
        )
    }

    /// Shown in cards: real attributes with a non-zero rolled value, like
    /// the legacy visual_attributes filter.
    pub fn is_visual(&self) -> bool {
        !self.is_virtual && self.value.abs() > f64::EPSILON
    }

    /// The color/style variant of the difference and the roll bar, like the
    /// legacy difference_type computed property.
    pub fn variant(&self) -> &'static str {
        match self.bar {
            1 => "gold",
            2 => "diamond",
            -1 => "brown",
            _ if self.is_derived && self.fraction >= 0.0 => "positive-derived",
            _ if self.is_derived => "negative-derived",
            _ if self.fraction >= 0.0 => "positive",
            _ => "negative",
        }
    }

    /// The -10..+10 roll score of the absolute fraction, like the legacy
    /// AttributeScore component.
    pub fn score(&self) -> i64 {
        (self.fraction_absolute * 20.0 - 10.0).round() as i64
    }

    pub fn score_label(&self) -> String {
        let score = self.score();
        if score > 0 {
            format!("+{score}")
        } else {
            score.to_string()
        }
    }

    /// Score color thresholds of the legacy component: green from 0.66,
    /// yellow from 0.33, red below.
    pub fn score_class(&self) -> &'static str {
        if self.fraction_absolute >= 0.66 {
            "text-green-500"
        } else if self.fraction_absolute >= 0.33 {
            "text-yellow-500"
        } else {
            "text-red-500"
        }
    }
}

/// The card accent key of a meta group, like the legacy header component.
pub fn meta_group_key(meta_group_id: Option<i64>) -> &'static str {
    match meta_group_id {
        Some(2) => "t2",
        Some(3) => "storyline",
        Some(4) => "faction",
        Some(5) => "officer",
        Some(6) => "deadspace",
        _ => "t1",
    }
}

/// The legacy module route pattern: an all-alphanumeric-and-dashes single
/// segment ending in digits is a module id (slug or bare id).
pub fn module_id_from_slug(query: &str) -> Option<i64> {
    if query.is_empty()
        || query.contains('/')
        || !query.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }

    let digits: String = query
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

/// Lowercase-dashed slug of a display name, like Laravel's `Str::slug`.
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());

    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }

    slug.trim_end_matches('-').to_owned()
}

/// URL slug of a module: the slugified type name plus the item id.
pub fn module_slug(type_name: &str, item_id: i64) -> String {
    format!("{}-{item_id}", slugify(type_name))
}

/// Compact display of an attribute value: two decimals, trailing zeros
/// trimmed.
pub fn format_number(value: f64) -> String {
    to_precision(value, 2)
}

/// A roll-quality fraction as a signed percentage.
pub fn format_fraction(fraction: f64) -> String {
    format!("{:+.1}%", fraction * 100.0)
}

// --- Attribute formatting, port of the legacy AttributeFormatter ----------

/// Converts a raw dogma value into its display value based on the unit:
/// milliseconds become seconds, modifier multipliers become signed percent
/// changes, per-millisecond rates become per-second.
pub fn transform_value(value: f64, unit_name: Option<&str>) -> f64 {
    match unit_name {
        Some("Milliseconds") => value / 1000.0,
        Some("Inversed Modifier Percent") | Some("Inverse Absolute Percent") => {
            (1.0 - value) * 100.0
        }
        Some("Hitpoints/Second") | Some("CubicMetersPerSecond") => value * 1000.0,
        Some("Modifier Percent") => (value - 1.0) * 100.0,
        Some("Absolute Percent") => value * 100.0,
        _ => value,
    }
}

/// The rolled value with its unit suffix, e.g. `12.5HP/s` or `1.234x`.
/// Display-only rounding, the legacy AttributeFormatter.toPrecision: an
/// Intl formatter capped at 3 fraction digits runs before the final
/// rounding, so edge values like x.xx45 round differently than a single
/// pass would. URL building keeps the single-pass precision.
fn display_round(value: f64) -> f64 {
    format!("{value:.3}").parse().unwrap_or(value)
}

pub fn format_value(value: f64, unit_name: Option<&str>, unit_display: Option<&str>) -> String {
    let transformed = display_round(transform_value(value, unit_name));
    let display = unit_display.unwrap_or_default();

    match unit_name {
        // Multipliers and inverted modifiers carry three decimals, like the
        // legacy frontend formatter.
        Some("Multiplier") | Some("Inversed Modifier Percent") => {
            format!("{}{display}", to_precision(transformed, 3))
        }
        Some(_) => format!("{}{display}", to_precision(transformed, 2)),
        None => format!("{}{display}", to_precision(display_round(value), 2)),
    }
}

/// The signed difference between the rolled and the base value, in display
/// units, e.g. `+1.2s` or `-3.5%`.
pub fn format_difference(
    value: f64,
    base_value: f64,
    unit_name: Option<&str>,
    unit_display: Option<&str>,
) -> String {
    let difference =
        display_round(transform_value(value, unit_name) - transform_value(base_value, unit_name));

    let signed = |formatted: String| {
        if difference > 0.0 {
            format!("+{formatted}")
        } else {
            formatted
        }
    };

    match unit_name {
        Some("Milliseconds") => format!("{}s", signed(to_precision(difference, 2))),
        Some("Inversed Modifier Percent")
        | Some("Inverse Absolute Percent")
        | Some("Modifier Percent")
        | Some("Absolute Percent")
        | Some("Percentage") => format!("{}%", signed(to_precision(difference, 2))),
        Some("Hitpoints/Second") => format!("{}HP/s", signed(to_precision(difference, 2))),
        Some("CubicMetersPerSecond") => format!("{}m³/s", signed(to_precision(difference, 2))),
        Some("Multiplier") => signed(to_precision(difference, 3)),
        _ => format!(
            "{}{}",
            signed(to_precision(difference, 2)),
            unit_display.unwrap_or_default(),
        ),
    }
}

/// Rounds to at most `precision` decimals and trims trailing zeros, like
/// PHP's round-and-cast and the frontend's toFixed-and-Number.
pub fn to_precision(value: f64, precision: usize) -> String {
    let formatted = format!("{value:.precision$}");
    // Only fractional zeros are padding; a whole number keeps its zeros
    // (trimming "1000000" to "1" corrupted six-figure filter URLs).
    let trimmed = if formatted.contains('.') {
        formatted.trim_end_matches('0').trim_end_matches('.')
    } else {
        &formatted
    };

    if trimmed == "-0" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

// --- Filter UI helpers, ports of the legacy frontend helpers -------------

/// Linear interpolation between ranges, the legacy
/// `TransformNumber.mapMinMax`.
pub fn map_min_max(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    (value - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}

/// A raw rolled value on the slider's normalized 0..100 scale, where 0 is
/// the type's worst roll and 100 its best (`AttributeMapper.toNormalized`).
pub fn to_normalized(value: f64, best: f64, worst: f64) -> f64 {
    map_min_max(value, worst, best, 0.0, 100.0)
}

/// A normalized slider value back in raw rolled units
/// (`AttributeMapper.toOriginal`).
pub fn to_original(value: f64, best: f64, worst: f64) -> f64 {
    map_min_max(value, 0.0, 100.0, worst, best)
}

/// Compact number for URL segments, like the legacy `FormatNumber.toUrl`
/// (significant-digit limited, no separators).
pub fn format_url_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }

    /// Significant digits kept in filter URLs.
    const URL_SIGNIFICANT_DIGITS: i32 = 6;

    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (URL_SIGNIFICANT_DIGITS - 1 - magnitude).clamp(0, 10) as usize;
    to_precision(value, decimals)
}

/// Market-wide module statistics, the legacy `ModulesStats` DTO shown on
/// the browser header. Query lives in `modules::stats`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModulesStats {
    pub total_count: i64,
    /// Modules currently for sale (a live latest contract) — the page
    /// header's market count (no legacy counterpart).
    pub listed_count: i64,
    pub added_last_hour_count: i64,
    pub added_last_day_count: i64,
    pub added_last_week_count: i64,
    pub contracts_count: i64,
    pub item_exchanges_count: i64,
    pub auctions_count: i64,
    pub goldbars_count: i64,
    pub brownbars_count: i64,
    pub diamondbars_count: i64,
}

/// A mutated attribute of an abyssal type with its extreme roll bounds,
/// backing one filter slider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FilterAttribute {
    pub attribute_id: i64,
    pub name: String,
    pub display_name: String,
    pub unit_name: Option<String>,
    pub unit_display_name: Option<String>,
    pub high_is_good: bool,
    /// Bar-only statistic (legacy is_virtual): shown on module bars but
    /// not offered by the mutation calculator.
    pub is_virtual: bool,
    /// Best reachable rolled value across all source type / mutaplasmid
    /// combinations producing this type.
    pub best: f64,
    /// Worst reachable rolled value across the same combinations.
    pub worst: f64,
}

/// A source type's base value for one filter attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FilterSourceTypeValue {
    pub attribute_id: i64,
    pub value: f64,
}

/// A published source type of the panel's abyssal type with its base
/// values — powers the slider pips, the per-attribute related-type
/// dropdown and the center type/attribute select
/// (specs/browser-filters.md §3.2/§3.4/§3.5). The legacy frontend read
/// this from its client-bundled statics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FilterSourceType {
    pub id: i64,
    pub name: String,
    pub meta_group_id: Option<i64>,
    pub meta_level: Option<i64>,
    pub attributes: Vec<FilterSourceTypeValue>,
}

/// Everything the filter panel needs once a type is selected.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FilterPanelData {
    pub type_id: i64,
    pub type_name: String,
    pub attributes: Vec<FilterAttribute>,
    /// Meta-rank-then-name ordered, like every legacy type list.
    pub source_types: Vec<FilterSourceType>,
}

/// A search failure the browser page shows to the user.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchFailure {
    pub message: String,
    pub not_found: bool,
}

/// Where a user's module physically sits, the legacy `AssetResource`
/// shape: the direct parent (ship/container, or the station itself when the
/// module lies loose in a hangar) plus the resolved station/structure at
/// the top of the ancestor chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AssetLocationView {
    pub parent_name: String,
    pub parent_type_id: Option<i64>,
    pub parent_slug: String,
    /// The station or structure hosting the chain (legacy `station` key).
    pub station: Option<StationRef>,
    pub location_id: i64,
    pub location_type: String,
    pub location_flag: String,
    pub location_index: i64,
    pub corporation_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StationRef {
    pub id: i64,
    pub name: String,
    pub type_id: Option<i64>,
    pub slug: String,
}

/// One asset-location row of the collection page's manage-modules dialog
/// (the legacy `LocationResource`). Deliberate divergence: trimmed to the
/// fields the redesigned dialog reads — the legacy parent_*/location_*
/// extras are dropped and the client derives each row's parent by
/// matching `location_id` against the row set's `item_id`s, exactly like
/// the legacy Vue component did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterLocationView {
    /// The assets.id the collection-location endpoints take (legacy
    /// `asset_id`).
    pub asset_id: i64,
    pub item_id: i64,
    pub name: Option<String>,
    pub type_id: i64,
    pub type_name: Option<String>,
    /// The EVE id of the containing asset, station or structure.
    pub location_id: Option<i64>,
    /// The station or structure rooting the chain (legacy `station` key).
    pub station: Option<StationRef>,
    /// Abyssal modules at or below this location, the legacy
    /// descendants_count (0 where legacy left it unloaded).
    pub modules_count: i64,
    pub public_asset_id: Option<i64>,
    pub corporation_id: Option<i64>,
    pub slug: String,
}

/// The human label of an ESI location flag, the legacy
/// `Static/LocationFlag.ts` map; unknown flags fall back to the raw value.
pub fn location_flag_label(flag: &str) -> String {
    let label = match flag {
        "AssetSafety" => "Asset Safety",
        "AutoFit" => "Auto Fit",
        "BoosterBay" => "Booster Bay",
        "Cargo" => "Cargo",
        "CorporationGoalDeliveries" => "Corporation Goal Deliveries",
        "CorpseBay" => "Corpse Bay",
        "Deliveries" => "Deliveries",
        "DroneBay" => "Drone Bay",
        "FighterBay" => "Fighter Bay",
        "FighterTube0" => "Fighter Tube 0",
        "FighterTube1" => "Fighter Tube 1",
        "FighterTube2" => "Fighter Tube 2",
        "FighterTube3" => "Fighter Tube 3",
        "FighterTube4" => "Fighter Tube 4",
        "FleetHangar" => "Fleet Hangar",
        "FrigateEscapeBay" => "Frigate Escape Bay",
        "Hangar" => "Hangar",
        "HangarAll" => "Hangar All",
        "HiSlot0" => "High Slot 1",
        "HiSlot1" => "High Slot 2",
        "HiSlot2" => "High Slot 3",
        "HiSlot3" => "High Slot 4",
        "HiSlot4" => "High Slot 5",
        "HiSlot5" => "High Slot 6",
        "HiSlot6" => "High Slot 7",
        "HiSlot7" => "High Slot 8",
        "HiddenModifiers" => "Hidden Modifiers",
        "Implant" => "Implant",
        "LoSlot0" => "Low Slot 1",
        "LoSlot1" => "Low Slot 2",
        "LoSlot2" => "Low Slot 3",
        "LoSlot3" => "Low Slot 4",
        "LoSlot4" => "Low Slot 5",
        "LoSlot5" => "Low Slot 6",
        "LoSlot6" => "Low Slot 7",
        "LoSlot7" => "Low Slot 8",
        "Locked" => "Locked",
        "MedSlot0" => "Med Slot 1",
        "MedSlot1" => "Med Slot 2",
        "MedSlot2" => "Med Slot 3",
        "MedSlot3" => "Med Slot 4",
        "MedSlot4" => "Med Slot 5",
        "MedSlot5" => "Med Slot 6",
        "MedSlot6" => "Med Slot 7",
        "MedSlot7" => "Med Slot 8",
        "MobileDepotHold" => "Mobile Depot Hold",
        "QuafeBay" => "Quafe Bay",
        "RigSlot0" => "Rig Slot 0",
        "RigSlot1" => "Rig Slot 1",
        "RigSlot2" => "Rig Slot 2",
        "RigSlot3" => "Rig Slot 3",
        "RigSlot4" => "Rig Slot 4",
        "RigSlot5" => "Rig Slot 5",
        "RigSlot6" => "Rig Slot 6",
        "RigSlot7" => "Rig Slot 7",
        "ShipHangar" => "Ship Hangar",
        "Skill" => "Skill",
        "SpecializedAmmoHold" => "Specialized Ammo Hold",
        "SpecializedAsteroidHold" => "Specialized Asteroid Hold",
        "SpecializedCommandCenterHold" => "Specialized Command Center Hold",
        "SpecializedFuelBay" => "Specialized Fuel Bay",
        "SpecializedGasHold" => "Specialized Gas Hold",
        "SpecializedIceHold" => "Specialized Ice Hold",
        "SpecializedIndustrialShipHold" => "Specialized Industrial Ship Hold",
        "SpecializedLargeShipHold" => "Specialized Large Ship Hold",
        "SpecializedMaterialBay" => "Specialized Material Bay",
        "SpecializedMediumShipHold" => "Specialized Medium Ship Hold",
        "SpecializedMineralHold" => "Specialized Mineral Hold",
        "SpecializedOreHold" => "Specialized Ore Hold",
        "SpecializedPlanetaryCommoditiesHold" => "Specialized Planetary Commodities Hold",
        "SpecializedSalvageHold" => "Specialized Salvage Hold",
        "SpecializedShipHold" => "Specialized Ship Hold",
        "SpecializedSmallShipHold" => "Specialized Small Ship Hold",
        "StructureDeedBay" => "Structure Deed Bay",
        "SubSystemBay" => "Sub System Bay",
        "SubSystemSlot0" => "Sub System Slot 0",
        "SubSystemSlot1" => "Sub System Slot 1",
        "SubSystemSlot2" => "Sub System Slot 2",
        "SubSystemSlot3" => "Sub System Slot 3",
        "SubSystemSlot4" => "Sub System Slot 4",
        "SubSystemSlot5" => "Sub System Slot 5",
        "SubSystemSlot6" => "Sub System Slot 6",
        "SubSystemSlot7" => "Sub System Slot 7",
        "Unlocked" => "Unlocked",
        "Wardrobe" => "Wardrobe",
        "CorpSAG1" => "Corp Hangar 1",
        "CorpSAG2" => "Corp Hangar 2",
        "CorpSAG3" => "Corp Hangar 3",
        "CorpSAG4" => "Corp Hangar 4",
        "CorpSAG5" => "Corp Hangar 5",
        "CorpSAG6" => "Corp Hangar 6",
        "CorpSAG7" => "Corp Hangar 7",
        "InfrastructureHangar" => "Infrastructure Hangar",
        "CorpDeliveries" => "Corp Deliveries",
        "CapsuleerDeliveries" => "Capsuleer Deliveries",
        "ExpeditionHold" => "Expedition Hold",
        other => return other.to_owned(),
    };

    label.to_owned()
}

/// The client-side view of a filter query path: enough to render and edit
/// the filter controls; the server-side `modules::search` stays the
/// authority for resolution and validation.
#[derive(Debug, Clone, PartialEq)]
pub struct UiSearch {
    /// One-based page, the legacy builder's trailing `page/N` segment
    /// (emitted only past page 1).
    pub page: i64,
    pub type_slug: Option<String>,
    pub meta_group: Option<String>,
    pub meta_level: Option<String>,
    pub attributes: Vec<UiAttributeFilter>,
    /// (field, descending) — field is `price`, `value`, `fraction` or an
    /// attribute name.
    pub sort: Option<(String, bool)>,
    pub contract_type: Option<String>,
    pub price: Option<(f64, Option<f64>)>,
    pub value: Option<(f64, Option<f64>)>,
    pub no_multi_item_contracts: bool,
    pub only_contracts: bool,
    pub without_other_items: bool,
    pub goldbar: bool,
    pub brownbar: bool,
    pub diamondbar: bool,
    pub with_personal_modules: bool,
    pub in_jita: bool,
    /// Character pages: the created-by scope instead of public listings.
    pub created: bool,
    /// Personal page: exclude fitted / asset-backed modules.
    pub without_fitted: bool,
    pub without_assets: bool,
}

impl Default for UiSearch {
    fn default() -> Self {
        Self {
            page: 1,
            type_slug: None,
            meta_group: None,
            meta_level: None,
            attributes: Vec::new(),
            sort: None,
            contract_type: None,
            price: None,
            value: None,
            no_multi_item_contracts: false,
            only_contracts: false,
            without_other_items: false,
            goldbar: false,
            brownbar: false,
            diamondbar: false,
            with_personal_modules: false,
            in_jita: false,
            created: false,
            without_fitted: false,
            without_assets: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiAttributeFilter {
    /// The attribute name as it appears in the URL (lowercased by the
    /// legacy builder).
    pub name: String,
    pub lower: f64,
    pub upper: Option<f64>,
}

/// The option keywords of the query path (mirror of the server-side list).
const UI_OPTION_KEYWORDS: [&str; 24] = [
    "page",
    "type",
    "meta-group",
    "meta-level",
    "auction",
    "item-exchange",
    "contracts-only",
    "no-multi-item-contracts",
    "goldbar",
    "brownbar",
    "diamondbar",
    "attributes",
    "contract-price",
    "estimated-value",
    "with-personal-modules",
    "sort",
    "without-contracts",
    "without-fitted",
    "without-other-items",
    "without-assets",
    "created",
    "search",
    "needs-training",
    "in-jita",
];

fn parse_bounds(text: &str) -> Option<(f64, Option<f64>)> {
    let (lower, rest) = take_leading_number(text)?;
    let upper = rest
        .strip_prefix('-')
        .and_then(|rest| take_leading_number(rest))
        .map(|(v, _)| v);
    Some((lower, upper))
}

fn take_leading_number(text: &str) -> Option<(f64, &str)> {
    let negative = text.starts_with('-');
    let digits = &text[usize::from(negative)..];

    let mut end = 0;
    let mut seen_dot = false;
    for (offset, c) in digits.char_indices() {
        if c.is_ascii_digit() {
            end = offset + 1;
        } else if c == '.' && !seen_dot && end > 0 {
            seen_dot = true;
        } else {
            break;
        }
    }

    if end == 0 {
        return None;
    }

    let end = end + usize::from(negative);
    text[..end]
        .parse()
        .ok()
        .map(|number| (number, &text[end..]))
}

/// Parses a filter query path textually for the filter controls.
pub fn parse_query_ui(query: &str) -> UiSearch {
    let segments: Vec<&str> = query
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut search = UiSearch::default();

    let mut index = 0;
    while index < segments.len() {
        let segment = segments[index];
        let args_start = index + 1;
        let args_end = (args_start..segments.len())
            .find(|&i| UI_OPTION_KEYWORDS.contains(&segments[i]))
            .unwrap_or(segments.len());
        let args = &segments[args_start..args_end];

        match segment {
            "page" => {
                search.page = args.first().and_then(|arg| arg.parse().ok()).unwrap_or(1);
            }
            "type" => search.type_slug = args.first().map(|s| (*s).to_owned()),
            "meta-group" => search.meta_group = args.first().map(|s| (*s).to_owned()),
            "meta-level" => search.meta_level = args.first().map(|s| (*s).to_owned()),
            "auction" => search.contract_type = Some("auction".to_owned()),
            "item-exchange" => search.contract_type = Some("item_exchange".to_owned()),
            "contracts-only" => search.only_contracts = true,
            "no-multi-item-contracts" => search.no_multi_item_contracts = true,
            "without-other-items" => search.without_other_items = true,
            "goldbar" => search.goldbar = true,
            "brownbar" => search.brownbar = true,
            "diamondbar" => search.diamondbar = true,
            "with-personal-modules" => search.with_personal_modules = true,
            "in-jita" => search.in_jita = true,
            "created" => search.created = true,
            "without-fitted" => search.without_fitted = true,
            "without-assets" => search.without_assets = true,
            "contract-price" => search.price = args.first().and_then(|arg| parse_bounds(arg)),
            "estimated-value" => search.value = args.first().and_then(|arg| parse_bounds(arg)),
            "sort" => {
                if let Some(field) = args.first() {
                    let descending = args.get(1).copied() == Some("desc");
                    search.sort = Some(((*field).to_owned(), descending));
                }
            }
            "attributes" => {
                for pair in args.chunks(2) {
                    let (Some(name), Some(bounds)) = (pair.first(), pair.get(1)) else {
                        continue;
                    };
                    if let Some((lower, upper)) = parse_bounds(bounds) {
                        search.attributes.push(UiAttributeFilter {
                            name: (*name).to_owned(),
                            lower,
                            upper,
                        });
                    }
                }
            }
            _ => {}
        }

        index = if UI_OPTION_KEYWORDS.contains(&segment) {
            args_end.max(index + 1)
        } else {
            index + 1
        };
    }

    search
}

/// Builds the filter query path, mirroring the legacy `QueryBuilder.make`
/// segment order exactly.
pub fn build_query_path(prefix: &str, search: &UiSearch) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(type_slug) = &search.type_slug {
        parts.push(format!("type/{type_slug}"));
    }
    if let Some(meta_group) = &search.meta_group {
        parts.push(format!("meta-group/{meta_group}"));
    }
    if let Some(meta_level) = &search.meta_level {
        parts.push(format!("meta-level/{meta_level}"));
    }

    if !search.attributes.is_empty() {
        let mut attribute_parts = Vec::new();
        for filter in &search.attributes {
            let name = filter.name.to_lowercase();
            match filter.upper {
                Some(upper) => attribute_parts.push(format!(
                    "{name}/{}-{}",
                    format_url_number(filter.lower),
                    format_url_number(upper),
                )),
                None => {
                    attribute_parts.push(format!("{name}/{}", format_url_number(filter.lower)));
                }
            }
        }
        parts.push(format!("attributes/{}", attribute_parts.join("/")));
    }

    if let Some((field, descending)) = &search.sort {
        let direction = if *descending { "desc" } else { "asc" };
        parts.push(format!("sort/{}/{direction}", field.to_lowercase()));
    }

    match search.contract_type.as_deref() {
        Some("item_exchange") => parts.push("item-exchange".to_owned()),
        Some("auction") => parts.push("auction".to_owned()),
        _ => {}
    }

    if let Some((lower, upper)) = search.price {
        match upper {
            Some(upper) => parts.push(format!("contract-price/{lower:.2}-{upper:.2}")),
            None => parts.push(format!("contract-price/{lower:.2}")),
        }
    }
    if let Some((lower, upper)) = search.value {
        match upper {
            Some(upper) => parts.push(format!("estimated-value/{lower:.2}-{upper:.2}")),
            None => parts.push(format!("estimated-value/{lower:.2}")),
        }
    }

    if search.no_multi_item_contracts {
        parts.push("no-multi-item-contracts".to_owned());
    }
    if search.only_contracts {
        parts.push("contracts-only".to_owned());
    }
    if search.goldbar {
        parts.push("goldbar".to_owned());
    }
    if search.brownbar {
        parts.push("brownbar".to_owned());
    }
    if search.diamondbar {
        parts.push("diamondbar".to_owned());
    }
    if search.without_other_items {
        parts.push("without-other-items".to_owned());
    }
    if search.with_personal_modules {
        parts.push("with-personal-modules".to_owned());
    }
    if search.in_jita {
        parts.push("in-jita".to_owned());
    }
    if search.created {
        parts.push("created".to_owned());
    }
    if search.without_fitted {
        parts.push("without-fitted".to_owned());
    }
    if search.without_assets {
        parts.push("without-assets".to_owned());
    }

    if search.page > 1 {
        parts.push(format!("page/{}", search.page));
    }

    if parts.is_empty() {
        format!("/{prefix}")
    } else {
        format!("/{prefix}/{}", parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::{format_fraction, format_number, module_id_from_slug, module_slug};

    #[test]
    fn module_ids_parse_from_slugs_and_bare_ids() {
        assert_eq!(
            module_id_from_slug("50mn-abyssal-microwarpdrive-1037153455177"),
            Some(1037153455177),
        );
        assert_eq!(module_id_from_slug("1037153455177"), Some(1037153455177));
        assert_eq!(module_id_from_slug("type/47408"), None);
        assert_eq!(module_id_from_slug("damage-control"), None);
        assert_eq!(module_id_from_slug(""), None);
    }

    #[test]
    fn module_slugs_normalize_type_names() {
        assert_eq!(
            module_slug("50MN Abyssal Microwarpdrive", 123),
            "50mn-abyssal-microwarpdrive-123",
        );
        assert_eq!(module_slug("Gistum C-Type Web", 5), "gistum-c-type-web-5");
    }

    #[test]
    fn numbers_and_fractions_format_compactly() {
        assert_eq!(format_number(241.919996), "241.92");
        assert_eq!(format_number(180.0), "180");
        assert_eq!(format_fraction(-0.86), "-86.0%");
        assert_eq!(format_fraction(0.67), "+67.0%");

        // Whole numbers keep their zeros: only fractional zeros trim.
        use super::format_url_number;
        assert_eq!(format_url_number(1_000_000.0), "1000000");
        assert_eq!(format_url_number(240.5), "240.5");
        assert_eq!(format_url_number(0.0), "0");
    }

    #[test]
    fn attribute_values_format_by_unit_like_the_legacy_formatter() {
        use super::{format_difference, format_value};

        // Milliseconds display as seconds.
        assert_eq!(format_value(5000.0, Some("Milliseconds"), Some("s")), "5s");
        assert_eq!(
            format_difference(4500.0, 5000.0, Some("Milliseconds"), Some("s")),
            "-0.5s",
        );

        // Modifier multipliers display as signed percent changes.
        assert_eq!(
            format_value(1.15, Some("Modifier Percent"), Some("%")),
            "15%"
        );
        assert_eq!(
            format_difference(1.2, 1.1, Some("Modifier Percent"), Some("%")),
            "+10%",
        );

        // Inverted modifiers: a 0.85 multiplier displays as its 15% bonus,
        // with up to three decimals.
        assert_eq!(
            format_value(0.85, Some("Inversed Modifier Percent"), Some("%")),
            "15%",
        );

        // Per-millisecond rates display per second.
        assert_eq!(
            format_value(0.0125, Some("Hitpoints/Second"), Some("HP/s")),
            "12.5HP/s",
        );
        assert_eq!(
            format_difference(0.0125, 0.01, Some("Hitpoints/Second"), Some("HP/s")),
            "+2.5HP/s",
        );

        // Multipliers carry three decimals and no suffix on differences.
        assert_eq!(
            format_value(1.2345678, Some("Multiplier"), Some("x")),
            "1.235x"
        );
        assert_eq!(
            format_difference(1.235, 1.2, Some("Multiplier"), Some("x")),
            "+0.035",
        );

        // Unknown units fall back to the raw value plus display name.
        assert_eq!(format_value(250.0, Some("Meters"), Some("m")), "250m");
        assert_eq!(format_value(42.5, None, None), "42.5");
    }

    #[test]
    fn slider_normalization_maps_between_worst_and_best() {
        use super::{to_normalized, to_original};

        // High is good: worst 100, best 200.
        assert_eq!(to_normalized(100.0, 200.0, 100.0), 0.0);
        assert_eq!(to_normalized(200.0, 200.0, 100.0), 100.0);
        assert_eq!(to_normalized(150.0, 200.0, 100.0), 50.0);
        assert_eq!(to_original(50.0, 200.0, 100.0), 150.0);

        // Low is good: worst 200, best 100 — direction handled by the map.
        assert_eq!(to_normalized(200.0, 100.0, 200.0), 0.0);
        assert_eq!(to_normalized(100.0, 100.0, 200.0), 100.0);
        assert_eq!(to_original(100.0, 100.0, 200.0), 100.0);
    }

    #[test]
    fn query_paths_build_in_the_legacy_segment_order() {
        use super::{UiAttributeFilter, UiSearch, build_query_path, parse_query_ui};

        let search = UiSearch {
            type_slug: Some("50mn-abyssal-microwarpdrive".to_owned()),
            meta_group: Some("t2".to_owned()),
            attributes: vec![UiAttributeFilter {
                name: "capacitorNeed".to_owned(),
                lower: 200.0,
                upper: Some(240.5),
            }],
            sort: Some(("price".to_owned(), true)),
            contract_type: Some("auction".to_owned()),
            price: Some((1000000.0, None)),
            goldbar: true,
            page: 3,
            ..UiSearch::default()
        };

        let path = build_query_path("modules", &search);
        assert_eq!(
            path,
            "/modules/type/50mn-abyssal-microwarpdrive/meta-group/t2\
             /attributes/capacitorneed/200-240.5/sort/price/desc/auction\
             /contract-price/1000000.00/goldbar/page/3"
                .replace(['\n', ' '], ""),
        );

        // Parsing the built path recovers the same search (names come back
        // as they appear in the URL).
        let parsed = parse_query_ui(path.trim_start_matches("/modules/"));
        assert_eq!(
            parsed.type_slug.as_deref(),
            Some("50mn-abyssal-microwarpdrive")
        );
        assert_eq!(parsed.meta_group.as_deref(), Some("t2"));
        assert_eq!(parsed.sort, Some(("price".to_owned(), true)));
        assert_eq!(parsed.contract_type.as_deref(), Some("auction"));
        assert_eq!(parsed.price, Some((1000000.0, None)));
        assert!(parsed.goldbar);
        assert_eq!(parsed.page, 3);
        assert_eq!(parsed.attributes.len(), 1);
        assert_eq!(parsed.attributes[0].name, "capacitorneed");
        assert_eq!(parsed.attributes[0].lower, 200.0);
        assert_eq!(parsed.attributes[0].upper, Some(240.5));
    }
}
