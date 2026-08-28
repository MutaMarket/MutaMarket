import { describe, expect, it } from 'vitest';

import { heroColumns, yearlySavings } from './premium';
import type { ModuleDetail } from './types';

function fakeModules(count: number): ModuleDetail[] {
	return Array.from({ length: count }, (_, index) => ({ id: index + 1 }) as ModuleDetail);
}

describe('premium page helpers', () => {
	it('deals the sample modules round-robin into three columns', () => {
		const columns = heroColumns(fakeModules(9));
		expect(columns.map((column) => column.map((module) => module.id))).toEqual([
			[1, 4, 7],
			[2, 5, 8],
			[3, 6, 9]
		]);
	});

	it('drops empty columns like the legacy filter', () => {
		expect(heroColumns(fakeModules(2)).length).toBe(2);
		expect(heroColumns([]).length).toBe(0);
	});

	it('the yearly plan saves two months', () => {
		expect(yearlySavings()).toBe(200_000_000);
	});
});
