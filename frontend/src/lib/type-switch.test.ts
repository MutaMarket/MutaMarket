// Transliterated from the Rust type_dialog unit tests.

import { describe, expect, it } from 'vitest';

import { buildQueryPath, defaultUiSearch } from './query';
import { typeSwitchSearch } from './type-switch';

describe('typeSwitchSearch', () => {
	it('keeps only the legacy flag subset', () => {
		const current = {
			...defaultUiSearch(),
			typeSlug: '47408',
			metaGroup: 't2',
			attributes: [{ name: 'speedfactor', lower: 500, upper: null }],
			sort: ['price', true] as [string, boolean],
			contractType: 'auction',
			price: [100, null] as [number, number | null],
			goldbar: true,
			onlyContracts: true
		};

		// Switching to another type: flags survive, the rest resets.
		expect(buildQueryPath('modules', typeSwitchSearch(current, 47408, 47740))).toBe(
			'/modules/type/47740/auction/contracts-only/goldbar'
		);

		// Clicking the active type deselects it.
		expect(buildQueryPath('modules', typeSwitchSearch(current, 47408, 47408))).toBe(
			'/modules/auction/contracts-only/goldbar'
		);
	});
});
