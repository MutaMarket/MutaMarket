// Slider position mapping, ported from the legacy AttributeMapper and
// CurrencyMapper: attribute sliders are linear between the type's worst
// (position 0) and best (position 100); the ISK sliders are log10 so a
// 1M..100B range stays draggable.
import { mapMinMax } from './attributes';

export function clamp(value: number, min: number, max: number): number {
	return Math.min(max, Math.max(min, value));
}

/** A raw attribute value to its 0-100 slider position. */
export function attributeToNormalized(value: number, best: number, worst: number): number {
	return mapMinMax(value, worst, best, 0, 100);
}

/** A 0-100 slider position back to the raw attribute value. */
export function attributeToOriginal(position: number, best: number, worst: number): number {
	return mapMinMax(position, 0, 100, worst, best);
}

/** An ISK amount to its 0-100 log-scale position. */
export function currencyToNormalized(value: number, lowest: number, highest: number): number {
	return (
		(100 * (Math.log10(value + 1) - Math.log10(lowest + 1))) /
		(Math.log10(highest + 1) - Math.log10(lowest + 1))
	);
}

/** A 0-100 log-scale position back to the ISK amount. */
export function currencyToOriginal(position: number, lowest: number, highest: number): number {
	return (
		Math.pow(
			10,
			(position / 100) * (Math.log10(highest + 1) - Math.log10(lowest + 1)) +
				Math.log10(lowest + 1),
		) - 1
	);
}
