//! Parsed form of the SDE inputs: CCP's official JSONL files and the
//! community dynamic-item-attributes JSON (mutaplasmid roll definitions).

use std::io::{self, BufRead};

use serde_json::Value;

use crate::mutation::reference::{MetaGroupRow, RegionRow, TypeRow, UnitRow};

#[derive(Debug, Default)]
pub struct SdeData {
    pub types: Vec<TypeRow>,
    pub attributes: Vec<SdeAttribute>,
    pub units: Vec<UnitRow>,
    pub meta_groups: Vec<MetaGroupRow>,
    pub regions: Vec<RegionRow>,
    /// Flattened `typeDogma.jsonl`: (type id, attribute id, value).
    pub type_dogma: Vec<(i64, i64, f64)>,
    pub dynamic_items: Vec<DynamicItem>,
    /// NPC stations with composed names, from [`build_stations`].
    pub stations: Vec<crate::mutation::reference::StationRow>,
}

#[derive(Debug, Clone)]
pub struct SdeAttribute {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub unit_id: Option<i64>,
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
            meta_group_id: record["metaGroupID"].as_i64(),
        })
    })
}

/// Parses `dogmaAttributes.jsonl`.
pub fn parse_dogma_attributes(reader: impl BufRead) -> io::Result<Vec<SdeAttribute>> {
    map_jsonl(reader, |record| {
        Some(SdeAttribute {
            id: record["_key"].as_i64()?,
            name: record["name"].as_str().unwrap_or_default().to_owned(),
            display_name: record["displayName"]["en"].as_str().unwrap_or_default().to_owned(),
            unit_id: record["unitID"].as_i64(),
            high_is_good: record["highIsGood"].as_bool().unwrap_or(false),
        })
    })
}

/// Parses `dogmaUnits.jsonl`.
pub fn parse_dogma_units(reader: impl BufRead) -> io::Result<Vec<UnitRow>> {
    map_jsonl(reader, |record| {
        Some(UnitRow {
            id: record["_key"].as_i64()?,
            name: record["name"].as_str().unwrap_or_default().to_owned(),
            display_name: record["displayName"]["en"].as_str().unwrap_or_default().to_owned(),
        })
    })
}

/// Parses `metaGroups.jsonl`.
pub fn parse_meta_groups(reader: impl BufRead) -> io::Result<Vec<MetaGroupRow>> {
    map_jsonl(reader, |record| {
        Some(MetaGroupRow {
            id: record["_key"].as_i64()?,
            name: record["name"]["en"].as_str().unwrap_or_default().to_owned(),
        })
    })
}

/// Parses `mapRegions.jsonl`.
pub fn parse_regions(reader: impl BufRead) -> io::Result<Vec<RegionRow>> {
    map_jsonl(reader, |record| {
        Some(RegionRow {
            id: record["_key"].as_i64()?,
            name: record["name"]["en"].as_str().unwrap_or_default().to_owned(),
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

/// Roman numeral of a planet's celestial index, the legacy `toRoman`.
fn to_roman(mut number: i64) -> String {
    const MAP: [(&str, i64); 13] = [
        ("M", 1000), ("CM", 900), ("D", 500), ("CD", 400), ("C", 100), ("XC", 90),
        ("L", 50), ("XL", 40), ("X", 10), ("IX", 9), ("V", 5), ("IV", 4), ("I", 1),
    ];

    let mut result = String::new();
    for (roman, value) in MAP {
        while number >= value {
            result.push_str(roman);
            number -= value;
        }
    }
    result
}

/// Builds the NPC station rows with their composed names, the legacy
/// `CreateStaticDataCommand` celestial naming chain: planets are
/// "{system} {roman}", moons "{planet} - Moon {n}", stations
/// "{orbit} - {corporation}[ {operation}]". Explicit SDE names win at every
/// level. Only the planets/moons stations actually orbit are read from the
/// (large) map files; stations orbiting anything else fall back to the
/// system name (stars carry the system name in legacy too).
pub fn build_stations(
    stations: impl BufRead,
    operations: impl BufRead,
    corporations: impl BufRead,
    systems: impl BufRead,
    moons: impl BufRead,
    planets: impl BufRead,
) -> io::Result<Vec<crate::mutation::reference::StationRow>> {
    struct NpcStation {
        id: i64,
        orbit_id: Option<i64>,
        owner_id: Option<i64>,
        operation_id: Option<i64>,
        use_operation_name: bool,
        type_id: Option<i64>,
        solar_system_id: i64,
        name: Option<String>,
    }

    let stations = map_jsonl(stations, |record| {
        Some(NpcStation {
            id: record["_key"].as_i64()?,
            orbit_id: record["orbitID"].as_i64(),
            owner_id: record["ownerID"].as_i64(),
            operation_id: record["operationID"].as_i64(),
            use_operation_name: record["useOperationName"].as_bool().unwrap_or(false),
            type_id: record["typeID"].as_i64(),
            solar_system_id: record["solarSystemID"].as_i64()?,
            name: record["name"]["en"].as_str().map(str::to_owned),
        })
    })?;

    let operations: std::collections::HashMap<i64, String> = map_jsonl(operations, |record| {
        Some((
            record["_key"].as_i64()?,
            record["operationName"]["en"].as_str().unwrap_or_default().to_owned(),
        ))
    })?
    .into_iter()
    .collect();

    let corporations: std::collections::HashMap<i64, String> = map_jsonl(corporations, |record| {
        Some((
            record["_key"].as_i64()?,
            record["name"]["en"].as_str().unwrap_or_default().to_owned(),
        ))
    })?
    .into_iter()
    .collect();

    // mapSolarSystems names are plain strings; other SDE files localize.
    let systems: std::collections::HashMap<i64, String> = map_jsonl(systems, |record| {
        Some((
            record["_key"].as_i64()?,
            record["name"]
                .as_str()
                .or_else(|| record["name"]["en"].as_str())
                .unwrap_or_default()
                .to_owned(),
        ))
    })?
    .into_iter()
    .collect();

    let orbit_ids: std::collections::HashSet<i64> =
        stations.iter().filter_map(|station| station.orbit_id).collect();

    // Moons stations orbit; collects the parent planet ids on the way.
    struct MoonPart {
        orbit_id: Option<i64>,
        orbit_index: i64,
        name: Option<String>,
    }
    let moons: std::collections::HashMap<i64, MoonPart> = map_jsonl(moons, |record| {
        let id = record["_key"].as_i64()?;
        if !orbit_ids.contains(&id) {
            return None;
        }
        Some((
            id,
            MoonPart {
                orbit_id: record["orbitID"].as_i64(),
                orbit_index: record["orbitIndex"].as_i64().unwrap_or(0),
                name: record["name"]["en"].as_str().map(str::to_owned),
            },
        ))
    })?
    .into_iter()
    .collect();

    let planet_ids: std::collections::HashSet<i64> = orbit_ids
        .iter()
        .copied()
        .chain(moons.values().filter_map(|moon| moon.orbit_id))
        .collect();

    struct PlanetPart {
        celestial_index: i64,
        solar_system_id: Option<i64>,
        name: Option<String>,
    }
    let planets: std::collections::HashMap<i64, PlanetPart> = map_jsonl(planets, |record| {
        let id = record["_key"].as_i64()?;
        if !planet_ids.contains(&id) {
            return None;
        }
        Some((
            id,
            PlanetPart {
                celestial_index: record["celestialIndex"].as_i64().unwrap_or(0),
                solar_system_id: record["solarSystemID"].as_i64(),
                name: record["name"]["en"].as_str().map(str::to_owned),
            },
        ))
    })?
    .into_iter()
    .collect();

    let system_name = |id: Option<i64>| {
        id.and_then(|id| systems.get(&id)).cloned().unwrap_or_else(|| "Unknown".to_owned())
    };
    let planet_name = |id: i64| -> Option<String> {
        let planet = planets.get(&id)?;
        Some(planet.name.clone().unwrap_or_else(|| {
            format!("{} {}", system_name(planet.solar_system_id), to_roman(planet.celestial_index))
        }))
    };

    Ok(stations
        .into_iter()
        .map(|station| {
            let name = station.name.clone().unwrap_or_else(|| {
                let orbit_name = station
                    .orbit_id
                    .and_then(|orbit_id| {
                        if let Some(moon) = moons.get(&orbit_id) {
                            Some(moon.name.clone().unwrap_or_else(|| {
                                let planet = moon
                                    .orbit_id
                                    .and_then(planet_name)
                                    .unwrap_or_else(|| "Unknown".to_owned());
                                format!("{planet} - Moon {}", moon.orbit_index)
                            }))
                        } else {
                            planet_name(orbit_id)
                        }
                    })
                    .unwrap_or_else(|| system_name(Some(station.solar_system_id)));

                let corporation = station
                    .owner_id
                    .and_then(|id| corporations.get(&id))
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_owned());

                if station.use_operation_name {
                    let operation = station
                        .operation_id
                        .and_then(|id| operations.get(&id))
                        .cloned()
                        .unwrap_or_default();
                    format!("{orbit_name} - {corporation} {operation}")
                } else {
                    format!("{orbit_name} - {corporation}")
                }
            });

            crate::mutation::reference::StationRow {
                id: station.id,
                name,
                type_id: station.type_id,
                solarsystem_id: station.solar_system_id,
            }
        })
        .collect())
}

#[cfg(test)]
mod station_tests {
    use super::build_stations;

    #[test]
    fn station_names_compose_like_the_legacy_celestial_chain() {
        // Jita 4-4 style: station orbits moon 40009087 on planet 50003760.
        let stations = concat!(
            r#"{"_key": 60003760, "orbitID": 40009087, "orbitIndex": 4, "ownerID": 1000035, "#,
            r#""operationID": 14, "useOperationName": true, "typeID": 52678, "solarSystemID": 30000142}"#,
            "\n",
            // A station orbiting the planet directly, without operation name.
            r#"{"_key": 60000001, "orbitID": 50003760, "ownerID": 1000035, "operationID": 14, "#,
            r#""useOperationName": false, "typeID": 1531, "solarSystemID": 30000142}"#,
            "\n",
            // An explicit SDE name wins over composition.
            r#"{"_key": 60000002, "name": {"en": "Custom Station"}, "solarSystemID": 30000142}"#,
        );
        let operations = r#"{"_key": 14, "operationName": {"en": "Assembly Plant"}}"#;
        let corporations = r#"{"_key": 1000035, "name": {"en": "Caldari Navy"}}"#;
        let systems = r#"{"_key": 30000142, "name": "Jita"}"#;
        let moons = concat!(
            r#"{"_key": 40009087, "orbitID": 50003760, "orbitIndex": 4}"#,
            "\n",
            r#"{"_key": 40000001, "orbitID": 50003760, "orbitIndex": 1}"#,
        );
        let planets = r#"{"_key": 50003760, "orbitID": 40000000, "celestialIndex": 4, "solarSystemID": 30000142}"#;

        let rows = build_stations(
            stations.as_bytes(),
            operations.as_bytes(),
            corporations.as_bytes(),
            systems.as_bytes(),
            moons.as_bytes(),
            planets.as_bytes(),
        )
        .expect("stations build");

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "Jita IV - Moon 4 - Caldari Navy Assembly Plant");
        assert_eq!(rows[0].solarsystem_id, 30000142);
        assert_eq!(rows[0].type_id, Some(52678));
        assert_eq!(rows[1].name, "Jita IV - Caldari Navy");
        assert_eq!(rows[2].name, "Custom Station");
    }
}
