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

/// Mutaplasmid grades that lose to a stronger grade on the same module, so
/// even a perfect roll cannot reach the type's extreme and earns no bar.
/// Matched by prefix, which is why the `Glorified` forms are spelled out.
///
/// Deliberate divergence from legacy, which also listed `Radical` and
/// `Exigent`. Neither is weak: they are the only grades their module
/// families have, with no stronger sibling to lose to. Listing them meant
/// the four mining and ice drone types could never earn a bar at all, and
/// the six other drone types could never earn gold, because their only
/// unlisted grade was the `Glorified` one and gold needs a bare grade.
const WEAK_MUTATORS: [&str; 4] = ["Decayed", "Glorified Decayed", "Gravid", "Glorified Gravid"];

/// Public so a one-off recompute can rescore stored rolls without
/// re-running the whole calculation: the bar depends only on the
/// attribute's final value, which is what `mutated_attributes` holds.
pub fn resolve_bar(context: &MutationContext, attribute_id: i64, value: f64) -> AttributeBar {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::mutation::context::{BarStatistic, Mutaplasmid, MutationContext};

    const ATTRIBUTE: i64 = 42;
    const BEST: f64 = 20.0;
    const WORST: f64 = 10.0;

    fn context(grade: &str) -> MutationContext {
        MutationContext {
            mutaplasmid: Mutaplasmid {
                id: 1,
                name: format!("{grade} Test Mutaplasmid"),
                output_type_id: 2,
            },
            mutaplasmid_attributes: Vec::new(),
            source_type_attributes: HashMap::new(),
            ranges: HashMap::new(),
            bar_statistics: HashMap::from([(
                ATTRIBUTE,
                BarStatistic {
                    best: BEST,
                    worst: WORST,
                },
            )]),
        }
    }

    fn bar(grade: &str, value: f64) -> AttributeBar {
        resolve_bar(&context(grade), ATTRIBUTE, value)
    }

    #[test]
    fn unstable_earns_gold_and_glorified_unstable_earns_diamond() {
        assert_eq!(bar("Unstable", BEST), AttributeBar::GoldBar);
        assert_eq!(bar("Glorified Unstable", BEST), AttributeBar::DiamondBar);
    }

    #[test]
    fn a_qualifying_grade_earns_brown_for_the_worst_roll() {
        assert_eq!(bar("Unstable", WORST), AttributeBar::BrownBar);
        assert_eq!(bar("Glorified Unstable", WORST), AttributeBar::BrownBar);
    }

    #[test]
    fn a_roll_between_the_extremes_earns_nothing() {
        assert_eq!(bar("Unstable", 15.0), AttributeBar::NoBar);
        assert_eq!(bar("Glorified Unstable", 15.0), AttributeBar::NoBar);
    }

    #[test]
    fn the_weak_grades_earn_nothing_at_either_extreme() {
        for grade in ["Decayed", "Glorified Decayed", "Gravid", "Glorified Gravid"] {
            assert_eq!(bar(grade, BEST), AttributeBar::NoBar, "{grade} at best");
            assert_eq!(bar(grade, WORST), AttributeBar::NoBar, "{grade} at worst");
        }
    }

    #[test]
    fn exigent_and_radical_earn_bars_like_any_top_grade() {
        // They have no stronger sibling: they are the best a Mutated Drone
        // or a Drone Damage Amplifier can be rolled with. Legacy filed them
        // with the weak grades, which left four types unable to earn a bar
        // at all and six others unable to earn gold.
        for grade in ["Exigent", "Radical"] {
            assert_eq!(bar(grade, BEST), AttributeBar::GoldBar, "{grade} at best");
            assert_eq!(
                bar(grade, WORST),
                AttributeBar::BrownBar,
                "{grade} at worst"
            );
        }
        for grade in ["Glorified Exigent", "Glorified Radical"] {
            assert_eq!(
                bar(grade, BEST),
                AttributeBar::DiamondBar,
                "{grade} at best"
            );
            assert_eq!(
                bar(grade, WORST),
                AttributeBar::BrownBar,
                "{grade} at worst"
            );
        }
    }

    #[test]
    fn an_attribute_that_cannot_vary_earns_nothing() {
        let mut fixed = context("Unstable");
        fixed.bar_statistics.insert(
            ATTRIBUTE,
            BarStatistic {
                best: BEST,
                worst: BEST,
            },
        );
        assert_eq!(resolve_bar(&fixed, ATTRIBUTE, BEST), AttributeBar::NoBar);
    }

    #[test]
    fn an_attribute_with_no_recorded_extremes_earns_nothing() {
        assert_eq!(
            resolve_bar(&context("Unstable"), 999, BEST),
            AttributeBar::NoBar,
        );
    }
}
