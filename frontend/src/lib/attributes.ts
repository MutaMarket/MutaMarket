// Attribute display logic, mirroring the Rust `modules::view` helpers
// (themselves ports of the legacy AttributeFormatter and card
// components). The formatted strings must match the Rust side exactly.

import type { ModuleAttributeView } from './types';
import { toPrecision } from './query';

/**
 * Converts a raw dogma value into its display value based on the unit:
 * milliseconds become seconds, modifier multipliers become signed percent
 * changes, per-millisecond rates become per-second.
 */
export function transformValue(value: number, unitName: string | null): number {
	switch (unitName) {
		case 'Milliseconds':
			return value / 1000;
		case 'Inversed Modifier Percent':
		case 'Inverse Absolute Percent':
			return (1 - value) * 100;
		case 'Hitpoints/Second':
		case 'CubicMetersPerSecond':
			return value * 1000;
		case 'Modifier Percent':
			return (value - 1) * 100;
		case 'Absolute Percent':
			return value * 100;
		default:
			return value;
	}
}

/**
 * The inverse of `transformValue`: a display-unit number typed by the
 * user back to the raw dogma value (legacy `revertTransformValue`).
 */
export function revertTransformValue(value: number, unitName: string | null): number {
	switch (unitName) {
		case 'Milliseconds':
			return value * 1000;
		case 'Inversed Modifier Percent':
		case 'Inverse Absolute Percent':
			return 1 - value / 100;
		case 'Hitpoints/Second':
		case 'CubicMetersPerSecond':
			return value / 1000;
		case 'Modifier Percent':
			return value / 100 + 1;
		case 'Absolute Percent':
			return value / 100;
		default:
			return value;
	}
}

/** The rolled value with its unit suffix, e.g. `12.5HP/s` or `1.234x`. */
export function formatValue(
	value: number,
	unitName: string | null,
	unitDisplay: string | null
): string {
	const transformed = transformValue(value, unitName);
	const display = unitDisplay ?? '';

	if (unitName === 'Multiplier' || unitName === 'Inversed Modifier Percent') {
		return `${toPrecision(transformed, 3)}${display}`;
	}
	if (unitName !== null) {
		return `${toPrecision(transformed, 2)}${display}`;
	}
	return `${toPrecision(value, 2)}${display}`;
}

/**
 * The signed difference between the rolled and the base value, in display
 * units, e.g. `+1.2s` or `-3.5%`.
 */
export function formatDifference(
	value: number,
	baseValue: number,
	unitName: string | null,
	unitDisplay: string | null
): string {
	const difference = transformValue(value, unitName) - transformValue(baseValue, unitName);
	const signed = (formatted: string) => (difference > 0 ? `+${formatted}` : formatted);

	switch (unitName) {
		case 'Milliseconds':
			return `${signed(toPrecision(difference, 2))}s`;
		case 'Inversed Modifier Percent':
		case 'Inverse Absolute Percent':
		case 'Modifier Percent':
		case 'Absolute Percent':
		case 'Percentage':
			return `${signed(toPrecision(difference, 2))}%`;
		case 'Hitpoints/Second':
			return `${signed(toPrecision(difference, 2))}HP/s`;
		case 'CubicMetersPerSecond':
			return `${signed(toPrecision(difference, 2))}m³/s`;
		case 'Multiplier':
			return signed(toPrecision(difference, 3));
		default:
			return `${signed(toPrecision(difference, 2))}${unitDisplay ?? ''}`;
	}
}

/** Compact display of an attribute value: two decimals, zeros trimmed. */
export function formatNumber(value: number): string {
	return toPrecision(value, 2);
}

/** A roll-quality fraction as a signed percentage. */
export function formatFraction(fraction: number): string {
	const percent = (fraction * 100).toFixed(1);
	return fraction * 100 >= 0 ? `+${percent}%` : `${percent}%`;
}

/** Linear interpolation between ranges, the legacy `mapMinMax`. */
export function mapMinMax(
	value: number,
	inMin: number,
	inMax: number,
	outMin: number,
	outMax: number
): number {
	return ((value - inMin) * (outMax - outMin)) / (inMax - inMin) + outMin;
}

/** A raw rolled value on the slider's normalized 0..100 scale. */
export function toNormalized(value: number, best: number, worst: number): number {
	return mapMinMax(value, worst, best, 0, 100);
}

/** A normalized slider value back in raw rolled units. */
export function toOriginal(value: number, best: number, worst: number): number {
	return mapMinMax(value, 0, 100, worst, best);
}

export function attributeFormattedValue(attribute: ModuleAttributeView): string {
	return formatValue(attribute.value, attribute.unit?.name ?? null, attribute.unit?.display_name ?? null);
}

export function attributeFormattedDifference(attribute: ModuleAttributeView): string {
	return formatDifference(
		attribute.value,
		attribute.base_value,
		attribute.unit?.name ?? null,
		attribute.unit?.display_name ?? null
	);
}

/** Shown in cards: real attributes with a non-zero rolled value. */
export function isVisual(attribute: ModuleAttributeView): boolean {
	return !attribute.is_virtual && Math.abs(attribute.value) > Number.EPSILON;
}

export type AttributeVariant =
	| 'gold'
	| 'diamond'
	| 'brown'
	| 'positive'
	| 'negative'
	| 'positive-derived'
	| 'negative-derived';

/** The color/style variant of the difference and the roll bar. */
export function attributeVariant(attribute: ModuleAttributeView): AttributeVariant {
	if (attribute.bar === 1) return 'gold';
	if (attribute.bar === 2) return 'diamond';
	if (attribute.bar === -1) return 'brown';
	if (attribute.is_derived) {
		return attribute.fraction >= 0 ? 'positive-derived' : 'negative-derived';
	}
	return attribute.fraction >= 0 ? 'positive' : 'negative';
}

/** The -10..+10 roll score of the absolute fraction. */
export function attributeScore(attribute: ModuleAttributeView): number {
	return Math.round(attribute.fraction_absolute * 20 - 10);
}

export function attributeScoreLabel(attribute: ModuleAttributeView): string {
	const score = attributeScore(attribute);
	return score > 0 ? `+${score}` : String(score);
}

/** Score color thresholds: green from 0.66, yellow from 0.33, red below. */
export function attributeScoreClass(attribute: ModuleAttributeView): string {
	if (attribute.fraction_absolute >= 0.66) return 'text-green-500';
	if (attribute.fraction_absolute >= 0.33) return 'text-yellow-500';
	return 'text-red-500';
}

/** The card accent key of a meta group. */
export function metaGroupKey(metaGroupId: number | null): string {
	switch (metaGroupId) {
		case 2:
			return 't2';
		case 3:
			return 'storyline';
		case 4:
			return 'faction';
		case 5:
			return 'officer';
		case 6:
			return 'deadspace';
		default:
			return 't1';
	}
}
