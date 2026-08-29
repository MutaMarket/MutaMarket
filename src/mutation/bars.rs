//! The gold/diamond/brown bar markers for best- and worst-possible rolls,
//! port of the legacy `AttributeBarResolver`.

use super::context::MutationContext;

/// Roll-quality marker: gold/diamond for the best possible roll of the
/// source type + mutaplasmid combination, brown for the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeBar {
    BrownBar,
    NoBar,
    GoldBar,
    DiamondBar,
}

impl AttributeBar {
    pub fn as_int(self) -> i64 {
        match self {
            AttributeBar::BrownBar => -1,
            AttributeBar::NoBar => 0,
            AttributeBar::GoldBar => 1,
            AttributeBar::DiamondBar => 2,
        }
    }
}

/// Mutaplasmid grades that can never yield the overall best roll of a type,
/// so their perfect rolls don't get a bar.
const WEAK_MUTATORS: [&str; 6] = [
    "Decayed",
    "Glorified Decayed",
    "Gravid",
    "Glorified Gravid",
    "Radical",
    "Exigent",
];

pub(super) fn resolve_bar(
    context: &MutationContext,
    attribute_id: i64,
    value: f64,
) -> AttributeBar {
    let name = context.mutaplasmid.name.as_str();

    if WEAK_MUTATORS.iter().any(|weak| name.starts_with(weak)) {
        return AttributeBar::NoBar;
    }

    let Some(statistic) = context.bar_statistic(attribute_id) else {
        return AttributeBar::NoBar;
    };

    // Attributes that cannot vary (best == worst) never get a bar.
    if statistic.best == statistic.worst {
        return AttributeBar::NoBar;
    }

    if approximately_same(statistic.best, value) {
        if name.starts_with("Glorified") {
            return AttributeBar::DiamondBar;
        }

        return AttributeBar::GoldBar;
    }

    if approximately_same(statistic.worst, value) {
        return AttributeBar::BrownBar;
    }

    AttributeBar::NoBar
}

/// Relative tolerance when comparing a rolled value against the recorded
/// best/worst extreme; absorbs float noise between ESI values and the
/// precomputed statistics (legacy used the same 1e-7).
const EXTREME_MATCH_TOLERANCE: f64 = 1e-7;

fn approximately_same(a: f64, b: f64) -> bool {
    (a - b).abs() <= EXTREME_MATCH_TOLERANCE * a.abs().max(b.abs())
}
