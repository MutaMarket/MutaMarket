//! Presentation types and pure helpers for modules, shared by the JSON API
//! and the Leptos pages (and therefore compiled for the browser as well).

use serde::{Deserialize, Serialize};

/// A module as the legacy `ModuleResource` emits it for guests with the
/// default relations loaded: same field names, same nesting. Keys owned by
/// unported features (`contract`, `public_asset`, estimator values) are
/// present and null, like the legacy loaded-but-empty relations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDetail {
    pub id: i64,
    pub r#type: TypeRef,
    pub creator: Option<CharacterRef>,
    pub mutated_attributes: Vec<ModuleAttributeView>,
    pub source_type: Option<SourceTypeRef>,
    pub mutaplasmid: Option<MutaplasmidRef>,
    /// The sale contract; arrives with the contracts milestone.
    pub contract: Option<serde_json::Value>,
    pub estimated_value: Option<f64>,
    pub estimated_value_updated_at: Option<String>,
    /// The MutaMarket sell listing; arrives with the assets milestone.
    pub public_asset: Option<serde_json::Value>,
    pub slug: String,
    pub average_fraction: Option<f64>,
}

/// Legacy `ModuleTypeResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    pub id: i64,
    pub name: String,
}

/// Legacy `MutaplasmidResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutaplasmidRef {
    pub id: i64,
    pub name: String,
}

/// Legacy `TypeResource` as loaded for the source type (meta group loaded,
/// type attributes and meta level not).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceTypeRef {
    pub id: i64,
    pub name: String,
    pub meta_group: Option<String>,
    pub meta_group_id: Option<i64>,
    pub published: bool,
}

/// Legacy `CharacterResource` without the user-conditional fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterRef {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub has_premium: bool,
    pub corporation_id: Option<i64>,
}

/// Legacy `UnitResource`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitRef {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

/// Legacy `MutatedAttributeResource`, plus the server-computed `type_band`
/// the legacy frontend derives from its bundled static data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        if score > 0 { format!("+{score}") } else { score.to_string() }
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

/// URL slug of a module: the slugified type name plus the item id.
pub fn module_slug(type_name: &str, item_id: i64) -> String {
    let mut slug = String::with_capacity(type_name.len() + 16);

    for c in type_name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }

    let slug = slug.trim_end_matches('-');
    format!("{slug}-{item_id}")
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
pub fn format_value(value: f64, unit_name: Option<&str>, unit_display: Option<&str>) -> String {
    let transformed = transform_value(value, unit_name);
    let display = unit_display.unwrap_or_default();

    match unit_name {
        // Multipliers and inverted modifiers carry three decimals, like the
        // legacy frontend formatter.
        Some("Multiplier") | Some("Inversed Modifier Percent") => {
            format!("{}{display}", to_precision(transformed, 3))
        }
        Some(_) => format!("{}{display}", to_precision(transformed, 2)),
        None => format!("{}{display}", to_precision(value, 2)),
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
    let difference = transform_value(value, unit_name) - transform_value(base_value, unit_name);

    let signed = |formatted: String| {
        if difference > 0.0 { format!("+{formatted}") } else { formatted }
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
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

    if trimmed == "-0" { "0".to_owned() } else { trimmed.to_owned() }
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
        assert_eq!(format_value(1.15, Some("Modifier Percent"), Some("%")), "15%");
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
        assert_eq!(format_value(1.2345678, Some("Multiplier"), Some("x")), "1.235x");
        assert_eq!(
            format_difference(1.235, 1.2, Some("Multiplier"), Some("x")),
            "+0.035",
        );

        // Unknown units fall back to the raw value plus display name.
        assert_eq!(format_value(250.0, Some("Meters"), Some("m")), "250m");
        assert_eq!(format_value(42.5, None, None), "42.5");
    }
}
