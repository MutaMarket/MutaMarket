// Transliterated from the Rust unit tests of the removed filter controls.

import { describe, expect, it } from 'vitest';

import { cycleSort, sortDirection } from './sort';

describe('cycleSort', () => {
	it('walks off, ascending, descending, off', () => {
		expect(cycleSort(null)).toBe(false);
		expect(cycleSort(false)).toBe(true);
		expect(cycleSort(true)).toBeNull();
	});
});

describe('sortDirection', () => {
	it('reads only the matching field', () => {
		const sort: [string, boolean] = ['price', true];
		expect(sortDirection(sort, 'price')).toBe(true);
		expect(sortDirection(sort, 'value')).toBeNull();
		expect(sortDirection(null, 'price')).toBeNull();
	});
});
