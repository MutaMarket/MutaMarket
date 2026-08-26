import { describe, expect, it } from 'vitest';
import { SPARKLE_VARIANTS, sparkleStyle, sparkleVariant } from './premium-foil';

describe('sparkleVariant', () => {
	it('is stable for the same key', () => {
		expect(sparkleVariant('quix-unar')).toBe(sparkleVariant('quix-unar'));
	});

	it('stays in range and spreads across variants', () => {
		const seen = new Set<number>();
		for (let index = 0; index < 60; index += 1) {
			const variant = sparkleVariant(`character-${index}`);
			expect(variant).toBeGreaterThanOrEqual(0);
			expect(variant).toBeLessThan(SPARKLE_VARIANTS);
			seen.add(variant);
		}
		expect(seen.size).toBe(SPARKLE_VARIANTS);
	});

	it('renders the css variable', () => {
		const style = sparkleStyle('quix-unar');
		expect(style).toMatch(/^--sparkle-image: url\('\/premium-sparkle-[0-5]\.webp'\)$/);
	});
});
