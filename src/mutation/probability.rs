//! Mutation probability, the legacy
//! `MutationProbabilityCalculatorService`: the chance one (mutaplasmid,
//! source type) roll lands every requested attribute inside its bounds,
//! against the recorded best/worst statistics.
//!
//! Deliberate divergence, and the reason this port exists: the legacy
//! service treated every attribute as an independent uniform roll, which
//! is wrong for derived attributes (a ratio of uniforms is not uniform,
//! and a derived value is fully correlated with its operands).
//!
//! The corrected math is exact wherever the geometry allows it. A bound
//! on a `{1}/{2}` ratio with a positive denominator, `z1 <= X/Y <= z2`,
//! is two linear constraints (`z1*Y <= X <= z2*Y`), so together with any
//! bounds on X and Y the feasible region inside the roll rectangle is a
//! polygon whose area we integrate exactly. Attributes not sharing a
//! ratio stay independent, so the total is a product of exact factors.
//! Only the cases with no closed form fall back to a joint Monte Carlo:
//! the four-operand mining formula, two derived bounds sharing an
//! operand, and degenerate or sign-crossing roll intervals.

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use super::context::MutationContext;
use super::derived::calculate_derived;
use super::reference::ReferenceData;

/// Samples per Monte Carlo fallback evaluation. At 20k the probability
/// estimate is stable to well under a percentage point, cheap enough to
/// run per combination on request.
const MONTE_CARLO_SAMPLES: usize = 20_000;

/// Deterministic sampling seed: the same query always reports the same
/// probability (and tests can pin values).
const MONTE_CARLO_SEED: u64 = 42;

/// The `derived_operation` of the six ratio attributes (boost/repair per
/// second and per gigajoule, DPS increase); the one shape with an exact
/// closed-form probability.
const RATIO_OPERATION: &str = "{1}/{2}";

/// One requested attribute bound, direction-resolved like the search
/// grammar: open ends mean "up to the best the roll allows".
#[derive(Debug, Clone, Copy)]
pub struct RequestedBound {
    pub attribute_id: i64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The chance one (source type, mutaplasmid) roll satisfies all bounds,
/// or `None` when the combination cannot be evaluated (no context).
/// A requested attribute the combination never rolls yields 0, like the
/// legacy service's unmatchable ranges.
pub fn combination_probability(
    reference: &ReferenceData,
    source_type_id: i64,
    mutaplasmid_id: i64,
    requested: &[RequestedBound],
) -> Option<f64> {
    if requested.is_empty() {
        return Some(1.0);
    }
    let context = reference.context(mutaplasmid_id, source_type_id)?;
    Some(context_probability(&context, requested))
}

/// The probability for one resolved context: the exact factorized form
/// when the request decomposes into independent intervals and ratio
/// polygons, the Monte Carlo fallback otherwise.
pub(super) fn context_probability(context: &MutationContext, requested: &[RequestedBound]) -> f64 {
    exact_probability(context, requested).unwrap_or_else(|| joint_monte_carlo(context, requested))
}

/// The achievable roll interval of one attribute: the recorded best and
/// worst (which cover derived attributes too), as (low, high).
fn achievable(context: &MutationContext, attribute_id: i64) -> Option<(f64, f64)> {
    let statistic = context.bar_statistics.get(&attribute_id)?;
    let low = statistic.best.min(statistic.worst);
    let high = statistic.best.max(statistic.worst);
    Some((low, high))
}

/// The requested interval clamped into the achievable one, as
/// (low, high); open ends extend to the achievable extreme, matching
/// the legacy best-value defaulting.
fn desired(bound: &Bound, achievable: (f64, f64)) -> (f64, f64) {
    let low = bound.min.unwrap_or(achievable.0).max(achievable.0);
    let high = bound.max.unwrap_or(achievable.1).min(achievable.1);
    (low, high)
}

/// A merged per-attribute bound (several requested bounds on the same
/// attribute intersect).
#[derive(Debug, Clone, Copy, Default)]
struct Bound {
    min: Option<f64>,
    max: Option<f64>,
}

impl Bound {
    fn merge(&mut self, other: &RequestedBound) {
        self.min = match (self.min, other.min) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        self.max = match (self.max, other.max) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
}

/// The exact evaluation, or `None` when the request has no closed form
/// and needs the Monte Carlo fallback.
fn exact_probability(context: &MutationContext, requested: &[RequestedBound]) -> Option<f64> {
    // Merge bounds per attribute, split real from ratio-derived.
    let mut real: Vec<(i64, Bound)> = Vec::new();
    let mut ratios: Vec<(i64, i64, i64, Bound)> = Vec::new(); // (derived, num, den)
    for bound in requested {
        let attribute = context.mutaplasmid_attribute(bound.attribute_id);
        let derived = attribute.is_some_and(|attribute| attribute.attribute.derived);
        if derived {
            let definition = &attribute?.attribute;
            if definition.derived_operation.as_deref() != Some(RATIO_OPERATION) {
                return None;
            }
            let [numerator, denominator] = definition.derived_attributes[..] else {
                return None;
            };
            // Both operands must be real rolled attributes of this
            // combination; otherwise the derived value degenerates the
            // way only the sampler reproduces faithfully.
            for operand in [numerator, denominator] {
                let real_roll = context
                    .mutaplasmid_attribute(operand)
                    .is_some_and(|attribute| !attribute.attribute.derived);
                if !real_roll {
                    return None;
                }
            }
            match ratios.iter_mut().find(|(id, ..)| *id == bound.attribute_id) {
                Some((.., merged)) => merged.merge(bound),
                None => {
                    let mut merged = Bound::default();
                    merged.merge(bound);
                    ratios.push((bound.attribute_id, numerator, denominator, merged));
                }
            }
        } else {
            match real.iter_mut().find(|(id, _)| *id == bound.attribute_id) {
                Some((_, merged)) => merged.merge(bound),
                None => {
                    let mut merged = Bound::default();
                    merged.merge(bound);
                    real.push((bound.attribute_id, merged));
                }
            }
        }
    }

    // Two ratio constraints sharing an operand couple three or more
    // dimensions; no closed form.
    for (index, (_, numerator, denominator, _)) in ratios.iter().enumerate() {
        for (_, other_num, other_den, _) in &ratios[index + 1..] {
            let operands = [*numerator, *denominator];
            if operands.contains(other_num) || operands.contains(other_den) {
                return None;
            }
        }
    }

    let mut probability = 1.0;

    for (derived_id, numerator, denominator, ratio_bound) in &ratios {
        let take = |attribute_id: i64, real: &mut Vec<(i64, Bound)>| {
            real.iter()
                .position(|(id, _)| *id == attribute_id)
                .map(|index| real.swap_remove(index).1)
                .unwrap_or_default()
        };
        let numerator_bound = take(*numerator, &mut real);
        let denominator_bound = take(*denominator, &mut real);
        probability *= ratio_probability(
            context,
            *derived_id,
            (*numerator, numerator_bound),
            (*denominator, denominator_bound),
            *ratio_bound,
        )?;
    }

    for (attribute_id, bound) in &real {
        probability *= interval_probability(context, *attribute_id, bound);
    }

    Some(probability)
}

/// The exact legacy interval arithmetic for one uniform real roll.
fn interval_probability(context: &MutationContext, attribute_id: i64, bound: &Bound) -> f64 {
    let Some(achievable) = achievable(context, attribute_id) else {
        return 0.0;
    };
    let (low, high) = desired(bound, achievable);
    if high < low {
        return 0.0;
    }
    let range = achievable.1 - achievable.0;
    if range <= 0.0 {
        // A fixed roll: inside the (clamped, non-empty) interval always.
        return 1.0;
    }
    ((high - low) / range).clamp(0.0, 1.0)
}

/// Exact probability of the coupled pair: X uniform on its roll interval,
/// Y uniform on its (strictly positive) roll interval, jointly satisfying
/// the X bound, the Y bound and `z_low <= X/Y <= z_high`. With Y > 0 the
/// ratio bound is the wedge `z_low*Y <= X <= z_high*Y`, so the feasible
/// region is a polygon; its area over the rectangle area is the answer.
/// `None` requests the Monte Carlo fallback (degenerate or sign-crossing
/// intervals).
fn ratio_probability(
    context: &MutationContext,
    derived_id: i64,
    (numerator, numerator_bound): (i64, Bound),
    (denominator, denominator_bound): (i64, Bound),
    ratio_bound: Bound,
) -> Option<f64> {
    let x_roll = achievable(context, numerator)?;
    let y_roll = achievable(context, denominator)?;
    if x_roll.1 <= x_roll.0 || y_roll.1 <= y_roll.0 || y_roll.0 <= 0.0 {
        return None;
    }
    // An unrecorded derived statistic means the combination never rolls
    // the requested value: 0, like the sampler's unmatchable target.
    let Some(z_roll) = achievable(context, derived_id) else {
        return Some(0.0);
    };

    let (x_low, x_high) = desired(&numerator_bound, x_roll);
    let (y_low, y_high) = desired(&denominator_bound, y_roll);
    let (z_low, z_high) = desired(&ratio_bound, z_roll);
    if x_high < x_low || y_high < y_low || z_high < z_low {
        return Some(0.0);
    }

    // Feasible x-width at a given y: [max(x_low, z_low*y), min(x_high, z_high*y)].
    // Piecewise linear in y; its kinks and zero crossings all sit where
    // one of the x limits meets one of the wedge lines, i.e. at x/z.
    let mut breakpoints = vec![y_low, y_high];
    for x in [x_low, x_high] {
        for z in [z_low, z_high] {
            let y = x / z;
            if y.is_finite() && y > y_low && y < y_high {
                breakpoints.push(y);
            }
        }
    }
    breakpoints.sort_by(|a, b| a.total_cmp(b));

    let width_at = |y: f64| (x_high.min(z_high * y)) - (x_low.max(z_low * y));
    let mut area = 0.0;
    for pair in breakpoints.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if to <= from {
            continue;
        }
        // Linear between breakpoints, so the trapezoid is exact; the
        // midpoint decides whether the segment is inside the polygon.
        if width_at((from + to) / 2.0) > 0.0 {
            area += (width_at(from).max(0.0) + width_at(to).max(0.0)) / 2.0 * (to - from);
        }
    }

    let rectangle = (x_roll.1 - x_roll.0) * (y_roll.1 - y_roll.0);
    Some((area / rectangle).clamp(0.0, 1.0))
}

/// Joint Monte Carlo fallback: sample every real roll once per iteration,
/// evaluate the derived formulas from those samples, and count the
/// iterations satisfying every requested bound together.
fn joint_monte_carlo(context: &MutationContext, requested: &[RequestedBound]) -> f64 {
    // Pre-resolve the desired intervals; an unmatchable request is 0.
    let mut targets = Vec::with_capacity(requested.len());
    for bound in requested {
        let Some(achievable) = achievable(context, bound.attribute_id) else {
            return 0.0;
        };
        let merged = Bound {
            min: bound.min,
            max: bound.max,
        };
        let (low, high) = desired(&merged, achievable);
        if high < low {
            return 0.0;
        }
        targets.push((bound.attribute_id, low, high));
    }

    let real_rolls: Vec<(i64, f64, f64)> = context
        .mutaplasmid_attributes
        .iter()
        .filter(|attribute| !attribute.attribute.derived)
        .filter_map(|attribute| {
            achievable(context, attribute.attribute_id)
                .map(|(low, high)| (attribute.attribute_id, low, high))
        })
        .collect();

    let mut rng = StdRng::seed_from_u64(MONTE_CARLO_SEED);
    let mut hits = 0usize;
    let mut rolled = std::collections::HashMap::with_capacity(real_rolls.len());
    for _ in 0..MONTE_CARLO_SAMPLES {
        rolled.clear();
        for &(attribute_id, low, high) in &real_rolls {
            let value = if high > low {
                rng.random_range(low..high)
            } else {
                low
            };
            rolled.insert(attribute_id, value);
        }
        let derived = calculate_derived(context, &rolled);

        let satisfied = targets.iter().all(|&(attribute_id, low, high)| {
            let value = rolled
                .get(&attribute_id)
                .copied()
                .or_else(|| derived.get(&attribute_id).map(|values| values.value));
            value.is_some_and(|value| value >= low && value <= high)
        });
        if satisfied {
            hits += 1;
        }
    }

    hits as f64 / MONTE_CARLO_SAMPLES as f64
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::context::{
        AttributeDef, BarStatistic, Mutaplasmid, MutaplasmidAttribute, MutationContext,
    };
    use super::*;

    const NUMERATOR_ID: i64 = 1;
    const DENOMINATOR_ID: i64 = 2;
    const DERIVED_ID: i64 = 3;

    fn attribute(id: i64, derived: bool) -> MutaplasmidAttribute {
        MutaplasmidAttribute {
            attribute_id: id,
            value_min: 0.0,
            value_max: 0.0,
            high_is_good: Some(true),
            is_virtual: derived,
            attribute: AttributeDef {
                id,
                name: format!("attr{id}"),
                display_name: format!("Attr {id}"),
                unit_id: None,
                high_is_good: true,
                derived,
                derived_operation: derived.then(|| RATIO_OPERATION.to_owned()),
                derived_attributes: if derived {
                    vec![NUMERATOR_ID, DENOMINATOR_ID]
                } else {
                    vec![]
                },
            },
        }
    }

    /// X uniform on [10, 20], Y uniform on [2, 4], D = X/Y on [2.5, 10].
    fn ratio_context() -> MutationContext {
        MutationContext {
            mutaplasmid: Mutaplasmid {
                id: 100,
                name: "Test".into(),
                output_type_id: 200,
            },
            mutaplasmid_attributes: vec![
                attribute(NUMERATOR_ID, false),
                attribute(DENOMINATOR_ID, false),
                attribute(DERIVED_ID, true),
            ],
            source_type_attributes: HashMap::new(),
            ranges: HashMap::new(),
            bar_statistics: HashMap::from([
                (
                    NUMERATOR_ID,
                    BarStatistic {
                        best: 20.0,
                        worst: 10.0,
                    },
                ),
                (
                    DENOMINATOR_ID,
                    BarStatistic {
                        best: 2.0,
                        worst: 4.0,
                    },
                ),
                (
                    DERIVED_ID,
                    BarStatistic {
                        best: 10.0,
                        worst: 2.5,
                    },
                ),
            ]),
        }
    }

    fn bound(attribute_id: i64, min: Option<f64>, max: Option<f64>) -> RequestedBound {
        RequestedBound {
            attribute_id,
            min,
            max,
        }
    }

    #[test]
    fn real_bound_keeps_the_interval_arithmetic() {
        let context = ratio_context();
        let p = context_probability(&context, &[bound(NUMERATOR_ID, Some(15.0), None)]);
        assert!((p - 0.5).abs() < 1e-12);
    }

    #[test]
    fn ratio_bound_is_the_exact_polygon_area() {
        let context = ratio_context();
        // P(X/Y >= 5) over [10,20]x[2,4]: area of {x >= 5y} is
        // ∫_2^4 (20 - 5y) dy = 10, over rectangle area 20.
        let p = context_probability(&context, &[bound(DERIVED_ID, Some(5.0), None)]);
        assert!((p - 0.5).abs() < 1e-12, "got {p}");
    }

    #[test]
    fn ratio_and_operand_bounds_combine_in_one_polygon() {
        let context = ratio_context();
        // X >= 15 and X/Y >= 5: 5 (for y in [2,3]) + 2.5 (for y in [3,4])
        // = 7.5 over 20.
        let p = context_probability(
            &context,
            &[
                bound(NUMERATOR_ID, Some(15.0), None),
                bound(DERIVED_ID, Some(5.0), None),
            ],
        );
        assert!((p - 0.375).abs() < 1e-12, "got {p}");
    }

    #[test]
    fn exact_ratio_matches_the_sampler() {
        let context = ratio_context();
        let requested = [
            bound(DERIVED_ID, Some(4.0), Some(8.0)),
            bound(DENOMINATOR_ID, None, Some(3.5)),
        ];
        let exact = exact_probability(&context, &requested).expect("closed form applies");
        let sampled = joint_monte_carlo(&context, &requested);
        assert!(
            (exact - sampled).abs() < 0.02,
            "exact {exact} vs sampled {sampled}"
        );
    }

    #[test]
    fn exact_beats_the_independence_assumption() {
        let context = ratio_context();
        // The legacy independent-uniform product for P(X/Y >= 5) would be
        // the derived interval ratio (10-5)/(10-2.5) ≈ 0.667; the true
        // joint probability is 0.5.
        let naive = interval_probability(
            &context,
            DERIVED_ID,
            &Bound {
                min: Some(5.0),
                max: None,
            },
        );
        let exact = context_probability(&context, &[bound(DERIVED_ID, Some(5.0), None)]);
        assert!((naive - exact).abs() > 0.1);
    }

    #[test]
    fn unmatchable_ratio_bound_is_zero() {
        let context = ratio_context();
        let p = context_probability(&context, &[bound(DERIVED_ID, Some(11.0), Some(12.0))]);
        assert_eq!(p, 0.0);
    }
}
