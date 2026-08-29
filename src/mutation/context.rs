use std::collections::HashMap;

/// Dogma attribute definition, including the derivation formula for
/// synthetic attributes such as "shield boost per second".
#[derive(Debug, Clone)]
pub struct AttributeDef {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub unit_id: Option<i64>,
    pub high_is_good: bool,
    pub derived: bool,
    pub derived_operation: Option<String>,
    pub derived_attributes: Vec<i64>,
}

/// One rollable attribute of a mutaplasmid: the roll-multiplier range and
/// optional overrides of the attribute definition.
#[derive(Debug, Clone)]
pub struct MutaplasmidAttribute {
    pub attribute_id: i64,
    pub value_min: f64,
    pub value_max: f64,
    /// Overrides the attribute definition's `high_is_good` when set.
    pub high_is_good: Option<bool>,
    pub is_virtual: bool,
    pub attribute: AttributeDef,
}

#[derive(Debug, Clone)]
pub struct Mutaplasmid {
    pub id: i64,
    pub name: String,
    pub output_type_id: i64,
}

/// The value ranges an attribute can span for one abyssal output type:
/// the combined roll-multiplier range across all mutaplasmids producing the
/// type, and the base-value range across all published source types those
/// mutaplasmids accept.
#[derive(Debug, Clone, Copy, Default)]
pub struct MutationRanges {
    pub mutator_min: Option<f64>,
    pub mutator_max: Option<f64>,
    pub source_value_min: Option<f64>,
    pub source_value_max: Option<f64>,
}

/// Best/worst possible rolled value for one (source type, mutaplasmid,
/// attribute) combination; used for the gold/brown bar markers.
#[derive(Debug, Clone, Copy)]
pub struct BarStatistic {
    pub best: f64,
    pub worst: f64,
}

/// All reference data needed to compute a module of one (mutaplasmid, source
/// type) combination. Loaded once up front so the attribute math itself is
/// pure and cheap to run in bulk.
#[derive(Debug, Clone)]
pub struct MutationContext {
    pub mutaplasmid: Mutaplasmid,
    pub mutaplasmid_attributes: Vec<MutaplasmidAttribute>,
    /// The source type's attribute records: attribute id -> value. A key with
    /// a `None` value means the record exists but carries no value, which the
    /// fraction math treats differently from a missing record.
    pub source_type_attributes: HashMap<i64, Option<f64>>,
    pub ranges: HashMap<i64, MutationRanges>,
    pub bar_statistics: HashMap<i64, BarStatistic>,
}

impl MutationContext {
    pub fn mutaplasmid_attribute(&self, attribute_id: i64) -> Option<&MutaplasmidAttribute> {
        self.mutaplasmid_attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
    }

    /// The source type's value for the attribute, if the record exists and
    /// carries a value.
    pub fn source_value(&self, attribute_id: i64) -> Option<f64> {
        self.source_type_attributes
            .get(&attribute_id)
            .copied()
            .flatten()
    }

    pub fn ranges(&self, attribute_id: i64) -> MutationRanges {
        self.ranges.get(&attribute_id).copied().unwrap_or_default()
    }

    pub fn bar_statistic(&self, attribute_id: i64) -> Option<BarStatistic> {
        self.bar_statistics.get(&attribute_id).copied()
    }
}
