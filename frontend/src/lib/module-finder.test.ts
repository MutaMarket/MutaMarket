import { describe, expect, it } from 'vitest';
import { cheapestSearchPath, historicSearchPath, similarSearchPath } from './module-finder';
import type { AbyssalTypeStatistic, ModuleDetail } from './types';

function attribute(id: number, name: string, value: number) {
	return {
		id,
		name,
		display_name: name,
		value,
		base_value: value,
		fraction: 0.1,
		fraction_type: 0.1,
		fraction_absolute: 0.1,
		bar: 1,
		is_derived: false,
		unit: null,
		is_virtual: false,
		type_band: null
	};
}

const module = {
	id: 42,
	type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
	creator: null,
	mutated_attributes: [attribute(20, 'speedFactor', 500), attribute(30, 'power', 200)],
	source_type: null,
	mutaplasmid: null,
	contract: null,
	estimated_value: null,
	estimated_value_updated_at: null,
	public_asset: null,
	slug: '50mn-abyssal-microwarpdrive-42',
	average_fraction: null
} satisfies ModuleDetail;

const statistics: AbyssalTypeStatistic[] = [
	{ attribute_id: 20, best: 600, worst: 400, high_is_good: true, is_virtual: false },
	{ attribute_id: 30, best: 100, worst: 300, high_is_good: false, is_virtual: false }
];

describe('similarSearchPath', () => {
	it('windows each enabled attribute by variance percent of the roll range', () => {
		// Range 200 at 1% variance: ±2 around the roll's 500.
		expect(similarSearchPath(module, statistics, [20], 1)).toBe(
			'/modules/type/47408/attributes/speedfactor/498-502'
		);
	});

	it('emits every enabled attribute in module order', () => {
		expect(similarSearchPath(module, statistics, [30, 20], 5)).toBe(
			'/modules/type/47408/attributes/speedfactor/490-510/power/190-210'
		);
	});
});

describe('cheapestSearchPath', () => {
	it('uses a single at-least bound with for-sale and price sort', () => {
		expect(cheapestSearchPath(module, statistics, [20], 1)).toBe(
			'/modules/type/47408/attributes/speedfactor/498/sort/price/asc/contracts-only'
		);
	});

	it('flips the bound for low-is-good attributes', () => {
		// power: high_is_good false, so the bound sits above the roll.
		expect(cheapestSearchPath(module, statistics, [30], 1)).toContain('power/202');
	});
});

describe('historicSearchPath', () => {
	it('is the cheapest search over the historic-sales prefix', () => {
		expect(historicSearchPath(module, statistics, [20], 1)).toBe(
			'/historic-sales/type/47408/attributes/speedfactor/498/sort/price/asc/contracts-only'
		);
	});
});
