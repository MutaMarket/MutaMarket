//! Presentation types and pure helpers for modules, shared by the JSON API
//! and the Leptos pages (and therefore compiled for the browser as well).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub id: i64,
    pub slug: String,
    pub type_id: i64,
    pub type_name: String,
    pub average_fraction: Option<f64>,
    pub creator_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleDetail {
    #[serde(flatten)]
    pub summary: ModuleSummary,
    pub source_type_id: Option<i64>,
    pub source_type_name: Option<String>,
    pub mutaplasmid_id: Option<i64>,
    pub mutaplasmid_name: Option<String>,
    pub attributes: Vec<ModuleAttributeView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleAttributeView {
    pub attribute_id: i64,
    pub name: String,
    pub value: f64,
    pub base_value: f64,
    pub fraction: f64,
    pub fraction_type: f64,
    pub fraction_absolute: f64,
    pub bar: i16,
    pub is_virtual: bool,
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
    let formatted = format!("{value:.2}");
    formatted.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A roll-quality fraction as a signed percentage.
pub fn format_fraction(fraction: f64) -> String {
    format!("{:+.1}%", fraction * 100.0)
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
}
