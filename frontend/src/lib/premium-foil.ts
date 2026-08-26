// Picks which glitter pattern a premium card wears. Six foil tiles ship
// as /premium-sparkle-{n}.webp; a stable hash of the card's key spreads
// them across cards while keeping each card's pattern fixed between
// visits.

/** Number of foil pattern tiles in frontend/static. */
export const SPARKLE_VARIANTS = 6;

export function sparkleVariant(key: string): number {
	let hash = 0;
	for (let index = 0; index < key.length; index += 1) {
		hash = (hash * 31 + key.charCodeAt(index)) >>> 0;
	}
	return hash % SPARKLE_VARIANTS;
}

/** The inline style carrying the card's pattern to the foil CSS. */
export function sparkleStyle(key: string): string {
	return `--sparkle-image: url('/premium-sparkle-${sparkleVariant(key)}.webp')`;
}
