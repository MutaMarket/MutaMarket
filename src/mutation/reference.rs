//! In-memory EVE reference data and the context builder mirroring the legacy
//! `MutationContextLoader` queries.
//!
//! All sources produce the same plain-row [`ReferenceTables`] — the gzipped
//! legacy dumps (test fixtures), Postgres, and eventually the native SDE
//! import — so indexing and the mutation math never care where the data came
//! from.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::path::Path;

use flate2::read::GzDecoder;
use serde_json::Value;

use super::context::{
    AttributeDef, BarStatistic, MutaplasmidAttribute, Mutaplasmid, MutationContext, MutationRanges,
};

/// Plain-row form of the reference tables, agnostic of their source.
#[derive(Debug, Default)]
pub struct ReferenceTables {
    pub attributes: Vec<AttributeDef>,
    pub types: Vec<TypeRow>,
    pub type_attributes: Vec<TypeAttributeRow>,
    pub mutaplasmids: Vec<Mutaplasmid>,
    pub mutaplasmid_attributes: Vec<MutaplasmidAttributeRow>,
    pub input_types: Vec<InputTypeRow>,
    pub statistics: Vec<StatisticRow>,
}

#[derive(Debug, Clone)]
pub struct TypeRow {
    pub id: i64,
    pub name: String,
    pub published: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TypeAttributeRow {
    pub id: i64,
    pub type_id: i64,
    pub attribute_id: i64,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct MutaplasmidAttributeRow {
    pub id: i64,
    pub mutaplasmid_id: i64,
    pub attribute_id: i64,
    pub value_min: f64,
    pub value_max: f64,
    pub high_is_good: Option<bool>,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct InputTypeRow {
    pub id: i64,
    pub mutaplasmid_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct StatisticRow {
    pub id: i64,
    pub type_id: i64,
    pub mutaplasmid_id: i64,
    pub attribute_id: i64,
    pub best: f64,
    pub worst: f64,
}

impl ReferenceTables {
    /// Reads the gzipped JSON table dumps (`attributes.json.gz` etc.)
    /// exported by the legacy app. Test-fixture path only — production data
    /// comes from Postgres, filled by the SDE import.
    pub fn load_from_dir(dir: &Path) -> io::Result<Self> {
        let mut tables = Self::default();

        for row in read_rows(&dir.join("attributes.json.gz"))? {
            tables.attributes.push(AttributeDef {
                id: int(&row["id"]).expect("attribute id"),
                name: row["name"].as_str().unwrap_or_default().to_owned(),
                high_is_good: boolish(&row["high_is_good"]).unwrap_or(false),
                derived: boolish(&row["derived"]).unwrap_or(false),
                derived_operation: row["derived_operation"].as_str().map(str::to_owned),
                derived_attributes: id_list(&row["derived_attributes"]),
            });
        }

        for row in read_rows(&dir.join("types.json.gz"))? {
            tables.types.push(TypeRow {
                id: int(&row["id"]).expect("type id"),
                name: row["name"].as_str().unwrap_or_default().to_owned(),
                published: boolish(&row["published"]).unwrap_or(false),
            });
        }

        for row in read_rows(&dir.join("type_attributes.json.gz"))? {
            tables.type_attributes.push(TypeAttributeRow {
                id: int(&row["id"]).expect("type attribute id"),
                type_id: int(&row["type_id"]).expect("type_id"),
                attribute_id: int(&row["attribute_id"]).expect("attribute_id"),
                value: num(&row["value"]),
            });
        }

        for row in read_rows(&dir.join("mutaplasmids.json.gz"))? {
            tables.mutaplasmids.push(Mutaplasmid {
                id: int(&row["id"]).expect("mutaplasmid id"),
                name: row["name"].as_str().unwrap_or_default().to_owned(),
                output_type_id: int(&row["output_type_id"]).expect("output_type_id"),
            });
        }

        for row in read_rows(&dir.join("mutaplasmid_attributes.json.gz"))? {
            tables.mutaplasmid_attributes.push(MutaplasmidAttributeRow {
                id: int(&row["id"]).expect("mutaplasmid attribute id"),
                mutaplasmid_id: int(&row["mutaplasmid_id"]).expect("mutaplasmid_id"),
                attribute_id: int(&row["attribute_id"]).expect("attribute_id"),
                value_min: num(&row["value_min"]).expect("value_min"),
                value_max: num(&row["value_max"]).expect("value_max"),
                high_is_good: boolish(&row["high_is_good"]),
                is_virtual: boolish(&row["is_virtual"]).unwrap_or(false),
            });
        }

        for row in read_rows(&dir.join("mutaplasmid_input_types.json.gz"))? {
            tables.input_types.push(InputTypeRow {
                id: int(&row["id"]).expect("input type id"),
                mutaplasmid_id: int(&row["mutaplasmid_id"]).expect("mutaplasmid_id"),
                type_id: int(&row["type_id"]).expect("type_id"),
            });
        }

        for row in read_rows(&dir.join("mutaplasmid_type_statistics.json.gz"))? {
            tables.statistics.push(StatisticRow {
                id: int(&row["id"]).expect("statistic id"),
                type_id: int(&row["type_id"]).expect("type_id"),
                mutaplasmid_id: int(&row["mutaplasmid_id"]).expect("mutaplasmid_id"),
                attribute_id: int(&row["attribute_id"]).expect("attribute_id"),
                best: num(&row["best"]).unwrap_or(0.0),
                worst: num(&row["worst"]).unwrap_or(0.0),
            });
        }

        Ok(tables)
    }
}

#[derive(Debug, Clone)]
struct RawMutaplasmidAttribute {
    attribute_id: i64,
    value_min: f64,
    value_max: f64,
    high_is_good: Option<bool>,
    is_virtual: bool,
}

#[derive(Debug, Default)]
pub struct ReferenceData {
    attributes: HashMap<i64, AttributeDef>,
    /// type id -> (attribute id, value) records, in row order.
    type_attributes: HashMap<i64, Vec<(i64, Option<f64>)>>,
    mutaplasmids: HashMap<i64, Mutaplasmid>,
    /// mutaplasmid id -> its attributes, in row order.
    mutaplasmid_attributes: HashMap<i64, Vec<RawMutaplasmidAttribute>>,
    /// (source type id, mutaplasmid id) -> attribute id -> best/worst roll.
    statistics: HashMap<(i64, i64), HashMap<i64, BarStatistic>>,

    // Indexes precomputed up front, so building a context does not rescan
    // the full mutaplasmid and input-type tables.
    /// output type id -> every mutaplasmid producing it.
    mutaplasmid_ids_by_output_type: HashMap<i64, Vec<i64>>,
    /// output type id -> every published source type accepted by any of
    /// those mutaplasmids.
    source_type_ids_by_output_type: HashMap<i64, HashSet<i64>>,
}

impl ReferenceData {
    pub fn load_from_dir(dir: &Path) -> io::Result<Self> {
        Ok(Self::from_tables(ReferenceTables::load_from_dir(dir)?))
    }

    /// Indexes plain reference rows for fast context building.
    pub fn from_tables(tables: ReferenceTables) -> Self {
        let mut data = Self::default();

        for attribute in tables.attributes {
            data.attributes.insert(attribute.id, attribute);
        }

        let published_type_ids: HashSet<i64> = tables
            .types
            .iter()
            .filter(|row| row.published)
            .map(|row| row.id)
            .collect();

        for row in tables.type_attributes {
            data.type_attributes
                .entry(row.type_id)
                .or_default()
                .push((row.attribute_id, row.value));
        }

        for mutaplasmid in tables.mutaplasmids {
            data.mutaplasmid_ids_by_output_type
                .entry(mutaplasmid.output_type_id)
                .or_default()
                .push(mutaplasmid.id);
            data.mutaplasmids.insert(mutaplasmid.id, mutaplasmid);
        }

        for row in tables.mutaplasmid_attributes {
            data.mutaplasmid_attributes
                .entry(row.mutaplasmid_id)
                .or_default()
                .push(RawMutaplasmidAttribute {
                    attribute_id: row.attribute_id,
                    value_min: row.value_min,
                    value_max: row.value_max,
                    high_is_good: row.high_is_good,
                    is_virtual: row.is_virtual,
                });
        }

        for row in tables.input_types {
            if !published_type_ids.contains(&row.type_id) {
                continue;
            }

            if let Some(mutaplasmid) = data.mutaplasmids.get(&row.mutaplasmid_id) {
                data.source_type_ids_by_output_type
                    .entry(mutaplasmid.output_type_id)
                    .or_default()
                    .insert(row.type_id);
            }
        }

        for row in tables.statistics {
            data.statistics
                .entry((row.type_id, row.mutaplasmid_id))
                .or_default()
                .insert(
                    row.attribute_id,
                    BarStatistic {
                        best: row.best,
                        worst: row.worst,
                    },
                );
        }

        data
    }

    /// Builds the mutation context for a (mutaplasmid, source type) pair,
    /// mirroring the legacy `MutationContextLoader` queries. Use a
    /// [`ContextCache`] when computing modules in bulk.
    pub fn context(&self, mutaplasmid_id: i64, source_type_id: i64) -> Option<MutationContext> {
        let mutaplasmid = self.mutaplasmids.get(&mutaplasmid_id)?.clone();

        let mutaplasmid_attributes: Vec<MutaplasmidAttribute> = self
            .mutaplasmid_attributes
            .get(&mutaplasmid_id)?
            .iter()
            .map(|raw| {
                let attribute = self
                    .attributes
                    .get(&raw.attribute_id)
                    .cloned()
                    .unwrap_or_else(|| panic!("unknown attribute {}", raw.attribute_id));

                MutaplasmidAttribute {
                    attribute_id: raw.attribute_id,
                    value_min: raw.value_min,
                    value_max: raw.value_max,
                    high_is_good: raw.high_is_good,
                    is_virtual: raw.is_virtual,
                    attribute,
                }
            })
            .collect();

        let attribute_ids: HashSet<i64> = mutaplasmid_attributes
            .iter()
            .map(|attribute| attribute.attribute_id)
            .collect();

        let ranges = self.ranges(&mutaplasmid, &mutaplasmid_attributes, &attribute_ids);

        let source_type_attributes: HashMap<i64, Option<f64>> = self
            .type_attributes
            .get(&source_type_id)
            .map(|records| records.iter().copied().collect())
            .unwrap_or_default();

        let bar_statistics = self
            .statistics
            .get(&(source_type_id, mutaplasmid_id))
            .cloned()
            .unwrap_or_default();

        Some(MutationContext {
            mutaplasmid,
            mutaplasmid_attributes,
            source_type_attributes,
            ranges,
            bar_statistics,
        })
    }

    fn ranges(
        &self,
        mutaplasmid: &Mutaplasmid,
        mutaplasmid_attributes: &[MutaplasmidAttribute],
        attribute_ids: &HashSet<i64>,
    ) -> HashMap<i64, MutationRanges> {
        // Roll ranges of every mutaplasmid producing the same abyssal type.
        let sibling_ids = self
            .mutaplasmid_ids_by_output_type
            .get(&mutaplasmid.output_type_id)
            .map(Vec::as_slice)
            .unwrap_or_default();

        let mut mutator_values: HashMap<i64, Vec<(f64, f64)>> = HashMap::new();
        for sibling_id in sibling_ids {
            for raw in self.mutaplasmid_attributes.get(sibling_id).into_iter().flatten() {
                if attribute_ids.contains(&raw.attribute_id) {
                    mutator_values
                        .entry(raw.attribute_id)
                        .or_default()
                        .push((raw.value_min, raw.value_max));
                }
            }
        }

        // Base-value extremes across every published source type accepted by
        // any of those mutaplasmids.
        let source_type_ids = self
            .source_type_ids_by_output_type
            .get(&mutaplasmid.output_type_id);

        let mut source_values: HashMap<i64, (f64, f64)> = HashMap::new();
        for type_id in source_type_ids.into_iter().flatten() {
            for (attribute_id, value) in self.type_attributes.get(type_id).into_iter().flatten() {
                let (Some(value), true) = (value, attribute_ids.contains(attribute_id)) else {
                    continue;
                };

                source_values
                    .entry(*attribute_id)
                    .and_modify(|(min, max)| {
                        *min = min.min(*value);
                        *max = max.max(*value);
                    })
                    .or_insert((*value, *value));
            }
        }

        mutaplasmid_attributes
            .iter()
            .map(|attribute| {
                let values = mutator_values
                    .get(&attribute.attribute_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default();

                // For attributes whose own roll range is inverted (max <= min,
                // e.g. fixed virtual attributes) the narrowest range across
                // all mutaplasmids applies; otherwise the widest.
                let narrowest = attribute.value_max <= attribute.value_min;

                let mins = values.iter().map(|&(min, _)| min);
                let maxs = values.iter().map(|&(_, max)| max);

                let source = source_values.get(&attribute.attribute_id);

                (
                    attribute.attribute_id,
                    MutationRanges {
                        mutator_min: if narrowest { fold_max(mins) } else { fold_min(mins) },
                        mutator_max: if narrowest { fold_min(maxs) } else { fold_max(maxs) },
                        source_value_min: source.map(|&(min, _)| min),
                        source_value_max: source.map(|&(_, max)| max),
                    },
                )
            })
            .collect()
    }
}

/// Memoizes contexts per (mutaplasmid, source type) pair, mirroring the
/// legacy `MutationContextLoader`: bulk recalculation across many modules
/// builds each combination's context only once.
pub struct ContextCache<'a> {
    reference: &'a ReferenceData,
    contexts: HashMap<(i64, i64), Option<MutationContext>>,
}

impl<'a> ContextCache<'a> {
    pub fn new(reference: &'a ReferenceData) -> Self {
        Self {
            reference,
            contexts: HashMap::new(),
        }
    }

    pub fn context(&mut self, mutaplasmid_id: i64, source_type_id: i64) -> Option<&MutationContext> {
        let Self { reference, contexts } = self;

        contexts
            .entry((mutaplasmid_id, source_type_id))
            .or_insert_with(|| reference.context(mutaplasmid_id, source_type_id))
            .as_ref()
    }
}

fn fold_min(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |acc, value| Some(acc.map_or(value, |a: f64| a.min(value))))
}

fn fold_max(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |acc, value| Some(acc.map_or(value, |a: f64| a.max(value))))
}

fn read_rows(path: &Path) -> io::Result<Vec<serde_json::Map<String, Value>>> {
    let file = File::open(path)?;
    serde_json::from_reader(GzDecoder::new(file)).map_err(io::Error::other)
}

fn int(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

/// Numbers in the dumps may be plain JSON numbers or decimal strings.
fn num(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

fn boolish(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(boolean) => Some(*boolean),
        Value::Number(_) | Value::String(_) => int(value).map(|number| number != 0),
        _ => None,
    }
}

/// `derived_attributes` is stored as a JSON string inside the dumps.
fn id_list(value: &Value) -> Vec<i64> {
    match value {
        Value::Array(items) => items.iter().filter_map(int).collect(),
        Value::String(string) => serde_json::from_str(string).unwrap_or_default(),
        _ => Vec::new(),
    }
}
