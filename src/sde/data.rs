//! Parsed form of the SDE inputs: CCP's official JSONL files and the
//! community dynamic-item-attributes JSON (mutaplasmid roll definitions).

use std::io::{self, BufRead};

use serde_json::Value;

use crate::mutation::reference::TypeRow;

#[derive(Debug, Default)]
pub struct SdeData {
    pub types: Vec<TypeRow>,
    pub attributes: Vec<SdeAttribute>,
    /// Flattened `typeDogma.jsonl`: (type id, attribute id, value).
    pub type_dogma: Vec<(i64, i64, f64)>,
    pub dynamic_items: Vec<DynamicItem>,
}

#[derive(Debug, Clone)]
pub struct SdeAttribute {
    pub id: i64,
    pub name: String,
    pub high_is_good: bool,
}

/// One mutaplasmid from the dynamic-item-attributes data.
#[derive(Debug, Clone)]
pub struct DynamicItem {
    /// The mutaplasmid's own type id.
    pub type_id: i64,
    pub attributes: Vec<DynamicAttribute>,
    pub applicable_types: Vec<i64>,
    pub resulting_type: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct DynamicAttribute {
    pub attribute_id: i64,
    pub min: f64,
    pub max: f64,
    pub high_is_good: Option<bool>,
}

/// Parses `types.jsonl`.
pub fn parse_types(reader: impl BufRead) -> io::Result<Vec<TypeRow>> {
    map_jsonl(reader, |record| {
        Some(TypeRow {
            id: record["_key"].as_i64()?,
            name: record["name"]["en"].as_str().unwrap_or_default().to_owned(),
            published: record["published"].as_bool().unwrap_or(true),
        })
    })
}

/// Parses `dogmaAttributes.jsonl`.
pub fn parse_dogma_attributes(reader: impl BufRead) -> io::Result<Vec<SdeAttribute>> {
    map_jsonl(reader, |record| {
        Some(SdeAttribute {
            id: record["_key"].as_i64()?,
            name: record["name"].as_str().unwrap_or_default().to_owned(),
            high_is_good: record["highIsGood"].as_bool().unwrap_or(false),
        })
    })
}

/// Parses `typeDogma.jsonl` into flattened (type, attribute, value) rows.
pub fn parse_type_dogma(reader: impl BufRead) -> io::Result<Vec<(i64, i64, f64)>> {
    let mut rows = Vec::new();

    each_jsonl(reader, |record| {
        let Some(type_id) = record["_key"].as_i64() else {
            return;
        };

        for attribute in record["dogmaAttributes"].as_array().into_iter().flatten() {
            let (Some(attribute_id), Some(value)) =
                (attribute["attributeID"].as_i64(), attribute["value"].as_f64())
            else {
                continue;
            };

            rows.push((type_id, attribute_id, value));
        }
    })?;

    Ok(rows)
}

/// Parses the dynamic-item-attributes JSON object
/// (mutaplasmid type id -> roll definition).
pub fn parse_dynamic_items(root: &Value) -> Vec<DynamicItem> {
    let Some(entries) = root.as_object() else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|(key, entry)| {
            let type_id: i64 = key.parse().ok()?;
            let mapping = entry["inputOutputMapping"].get(0)?;

            let attributes = entry["attributeIDs"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(attribute_key, attribute)| {
                    Some(DynamicAttribute {
                        attribute_id: attribute_key.parse().ok()?,
                        min: attribute["min"].as_f64()?,
                        max: attribute["max"].as_f64()?,
                        high_is_good: lenient_bool(&attribute["highIsGood"]),
                    })
                })
                .collect();

            Some(DynamicItem {
                type_id,
                attributes,
                applicable_types: mapping["applicableTypes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_i64)
                    .collect(),
                resulting_type: mapping["resultingType"].as_i64()?,
            })
        })
        .collect()
}

/// The dynamic-item data encodes booleans as 0/1 integers (`highIsGood`),
/// which PHP's loose typing absorbed silently in the legacy importer.
fn lenient_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(boolean) => Some(*boolean),
        Value::Number(number) => Some(number.as_f64()? != 0.0),
        _ => None,
    }
}

fn map_jsonl<T>(
    reader: impl BufRead,
    mut map: impl FnMut(&Value) -> Option<T>,
) -> io::Result<Vec<T>> {
    let mut rows = Vec::new();
    each_jsonl(reader, |record| rows.extend(map(record)))?;
    Ok(rows)
}

fn each_jsonl(reader: impl BufRead, mut each: impl FnMut(&Value)) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let record: Value = serde_json::from_str(&line).map_err(io::Error::other)?;
        each(&record);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_dynamic_items;

    #[test]
    fn dynamic_item_high_is_good_overrides_parse_from_integers() {
        let root = serde_json::json!({
            "47699": {
                "attributeIDs": {
                    "20": { "min": 0.97, "max": 1.03, "highIsGood": 0 },
                    "50": { "min": 0.8, "max": 1.5 },
                    "73": { "min": 0.9, "max": 1.1, "highIsGood": true },
                },
                "inputOutputMapping": [
                    { "applicableTypes": [526], "resultingType": 47702 }
                ],
            }
        });

        let items = parse_dynamic_items(&root);
        assert_eq!(items.len(), 1);

        let by_id = |id: i64| {
            items[0]
                .attributes
                .iter()
                .find(|attribute| attribute.attribute_id == id)
                .expect("attribute parsed")
        };

        assert_eq!(by_id(20).high_is_good, Some(false));
        assert_eq!(by_id(50).high_is_good, None);
        assert_eq!(by_id(73).high_is_good, Some(true));
        assert_eq!(items[0].resulting_type, 47702);
        assert_eq!(items[0].applicable_types, vec![526]);
    }
}
