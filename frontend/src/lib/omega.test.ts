import { describe, expect, it } from 'vitest';
import {
	OMEGA_PACKAGES,
	PLEX_PACKAGES,
	calculateScenario,
	costPerMonth,
	discountedOmegaPlex,
	discountedPlexPrice,
	effectiveTotalDiscount,
	omegaMonthsAffordable,
	regularCostPerMonth,
	regularOmegaMonths,
	scenarios,
} from './omega';

// The legacy defaults: 20,000 PLEX at $650 and the 24-month package.
const plexPkg = PLEX_PACKAGES[0];
const omegaPkg = OMEGA_PACKAGES[0];

describe('discountedPlexPrice', () => {
	it('applies the sale first, then the MarkeeDragon 3%', () => {
		expect(discountedPlexPrice(plexPkg, 20, false)).toBeCloseTo(520, 10);
		expect(discountedPlexPrice(plexPkg, 20, true)).toBeCloseTo(504.4, 10);
		expect(discountedPlexPrice(plexPkg, 0, false)).toBe(650);
	});
});

describe('effectiveTotalDiscount', () => {
	it('folds both discounts into the one-decimal readout', () => {
		expect(effectiveTotalDiscount(plexPkg, 20, true)).toBe('22.4');
		expect(effectiveTotalDiscount(plexPkg, 0, false)).toBe('0.0');
	});
});

describe('discountedOmegaPlex', () => {
	it('rounds the NES-sale PLEX price', () => {
		expect(discountedOmegaPlex(omegaPkg, 20)).toBe(5280);
		expect(discountedOmegaPlex(omegaPkg, 0)).toBe(6600);
		// A price that does not divide evenly rounds like Math.round.
		expect(discountedOmegaPlex(OMEGA_PACKAGES[1], 21)).toBe(2844);
	});
});

describe('omegaMonthsAffordable', () => {
	it('floors whole months at the discounted rate', () => {
		expect(omegaMonthsAffordable(plexPkg, omegaPkg, 20)).toBe(90);
		expect(omegaMonthsAffordable(plexPkg, omegaPkg, 0)).toBe(72);
	});
});

describe('costPerMonth', () => {
	it('divides the discounted PLEX cost across the affordable months', () => {
		expect(costPerMonth(plexPkg, omegaPkg, 20, true, 20)).toBeCloseTo(504.4 / 90, 10);
	});
});

describe('regular price comparison', () => {
	it('matches the no-discount columns', () => {
		expect(regularOmegaMonths(plexPkg, omegaPkg)).toBe(72);
		expect(regularCostPerMonth(plexPkg, omegaPkg)).toBeCloseTo(650 / 72, 10);
	});
});

describe('scenarios', () => {
	it('builds the legacy five rows, code always on for the third', () => {
		const rows = scenarios(20, false, 25, 24);
		expect(rows.map((row) => row.name)).toEqual([
			'No Sales (Baseline)',
			'PLEX Sale Only (20%)',
			'PLEX + MarkeeDragon',
			'NES Sale Only (25%)',
			'Full Stack',
		]);
		expect(rows[2].markeedragon).toBe(true);
		// The full stack honors the checkbox (off here), a legacy quirk.
		expect(rows[4].markeedragon).toBe(false);
		expect(rows[4].isFullStack).toBe(true);
	});
});

describe('calculateScenario', () => {
	it('formats the baseline row', () => {
		const [baseline] = scenarios(20, true, 20, 24);
		expect(calculateScenario(plexPkg, baseline)).toEqual({
			plexCost: '650.00',
			months: 72,
			costPerMonth: '9.03',
			moneySaved: '0.00',
			extraMonths: 0,
			savingsPct: '0.0',
		});
	});

	it('formats the full stack row', () => {
		const rows = scenarios(20, true, 20, 24);
		expect(calculateScenario(plexPkg, rows[4])).toEqual({
			plexCost: '504.40',
			months: 90,
			costPerMonth: '5.60',
			moneySaved: '145.60',
			extraMonths: 18,
			savingsPct: '22.4',
		});
	});

	it('uses the 12-month package for its rows', () => {
		const rows = scenarios(0, false, 25, 12);
		const nesOnly = calculateScenario(plexPkg, rows[3]);
		// 3600 * 0.75 = 2700 PLEX for 12 months -> 225/month -> 88 months.
		expect(nesOnly.months).toBe(88);
		expect(nesOnly.extraMonths).toBe(88 - 66);
	});
});
