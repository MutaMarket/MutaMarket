import { describe, expect, it } from 'vitest';
import { comparisonCells, compareTypes, metaGroupRank } from './source-types';
import type { ModuleAttributeView, SourceTypeComparison } from './types';

function attribute(overrides: Partial<ModuleAttributeView>): ModuleAttributeView {
	return {
		id: 20,
		name: 'speedFactor',
		display_name: 'Maximum Velocity Bonus',
		value: 5.6,
		base_value: 5.0,
		fraction: 0.1,
		fraction_type: 0.1,
		fraction_absolute: 0.1,
		bar: 1,
		is_derived: false,
		unit: { id: 124, name: 'Modifier Percent', display_name: '%' },
		is_virtual: false,
		type_band: null,
		...overrides,
	};
}

function comparison(
	values: { id: number; value: number }[],
	type?: Partial<SourceTypeComparison['type']>,
): SourceTypeComparison {
	return {
		type: { id: 440, name: '50MN Microwarpdrive I', meta_group_id: 1, meta_level: 0, ...type },
		attributes: values,
		average_price: null,
	};
}

describe('comparisonCells', () => {
	it('formats the input value and the roll difference', () => {
		const cells = comparisonCells([attribute({})], comparison([{ id: 20, value: 5.0 }]));
		expect(cells).toEqual([
			// Modifier Percent displays as (value - 1) * 100.
			{ attribute_id: 20, value: '400%', difference: '+60%', is_positive: true },
		]);
	});

	it('marks a better input type negative when high is good', () => {
		const cells = comparisonCells([attribute({})], comparison([{ id: 20, value: 6.0 }]));
		expect(cells[0].is_positive).toBe(false);
		expect(cells[0].difference).toBe('-40%');
	});

	it('follows the legacy direction quirk for a downward roll', () => {
		// A negative fraction with the value below base still counts as
		// high-is-good in the legacy table, so inputs above the roll read
		// negative.
		const low = attribute({ value: 4.4, fraction: -0.1 });
		expect(comparisonCells([low], comparison([{ id: 20, value: 5.0 }]))[0].is_positive).toBe(false);
		expect(comparisonCells([low], comparison([{ id: 20, value: 4.0 }]))[0].is_positive).toBe(true);
	});

	it('falls back to zero for attributes the input type lacks', () => {
		const cells = comparisonCells([attribute({})], comparison([]));
		expect(cells[0].value).toBe('-100%');
	});
});

describe('type ordering', () => {
	it('ranks meta groups in the legacy order', () => {
		expect([1, 2, 3, 4, 6, 5].map(metaGroupRank)).toEqual([1, 2, 3, 4, 5, 6]);
	});

	it('sorts by rank, then meta level, then name', () => {
		const officer = comparison([], { id: 1, name: 'A', meta_group_id: 5, meta_level: 14 });
		const deadspaceLow = comparison([], { id: 2, name: 'B', meta_group_id: 6, meta_level: 11 });
		const deadspaceHigh = comparison([], { id: 3, name: 'A', meta_group_id: 6, meta_level: 13 });
		const tied = comparison([], { id: 4, name: 'C', meta_group_id: 6, meta_level: 11 });

		const sorted = [officer, tied, deadspaceHigh, deadspaceLow].sort(compareTypes);
		expect(sorted.map((entry) => entry.type.id)).toEqual([2, 4, 3, 1]);
	});
});
