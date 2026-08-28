import { describe, expect, it } from 'vitest';

import { toCompact, toIsk, toIskCompact, toMillions, toVeryCompact } from './format-number';

describe('FormatNumber port', () => {
	it('formats ISK compactly with the legacy wording', () => {
		expect(toIskCompact(142_000_000)).toBe('142 million ISK');
		expect(toIskCompact(215_000_000, false)).toBe('215 million');
		expect(toIskCompact(null)).toBe('N/A');
	});

	it('short and long compact forms', () => {
		expect(toVeryCompact(1234)).toBe('1.2K');
		expect(toVeryCompact(98_000_000)).toBe('98M');
		expect(toCompact(1500)).toBe('1.5 thousand');
	});

	it('whole grouped millions for donation amounts', () => {
		expect(toMillions(150_000_000)).toBe('150M');
		expect(toMillions(1_500_000_000)).toBe('1,500M');
		expect(toMillions(15_400_000)).toBe('15M');
		expect(toMillions(500_000)).toBe('1M');
	});

	it('full currency form', () => {
		// Intl separates the currency code with a non-breaking space.
		expect(toIsk(1234.5)).toBe('ISK 1,235');
		expect(toIsk(1234.5, false)).toBe(' 1,235');
	});
});
