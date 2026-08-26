// Transliterated from the Rust unit tests in modules/view.rs and the
// removed pages, so the TS mirror stays pinned to the same grammar.

import { describe, expect, it } from 'vitest';

import {
	buildQueryPath,
	defaultUiSearch,
	formatUrlNumber,
	moduleIdFromSlug,
	moduleSlug,
	parseQueryUi
} from './query';

describe('moduleIdFromSlug', () => {
	it('parses ids from slugs and bare ids', () => {
		expect(moduleIdFromSlug('50mn-abyssal-microwarpdrive-1037153455177')).toBe(1037153455177);
		expect(moduleIdFromSlug('1037153455177')).toBe(1037153455177);
		expect(moduleIdFromSlug('type/47408')).toBeNull();
		expect(moduleIdFromSlug('damage-control')).toBeNull();
		expect(moduleIdFromSlug('')).toBeNull();
	});
});

describe('moduleSlug', () => {
	it('normalizes type names', () => {
		expect(moduleSlug('50MN Abyssal Microwarpdrive', 123)).toBe(
			'50mn-abyssal-microwarpdrive-123'
		);
		expect(moduleSlug('Gistum C-Type Web', 5)).toBe('gistum-c-type-web-5');
	});
});

describe('formatUrlNumber', () => {
	it('keeps six significant digits and trims zeros', () => {
		expect(formatUrlNumber(0)).toBe('0');
		expect(formatUrlNumber(200)).toBe('200');
		expect(formatUrlNumber(240.5)).toBe('240.5');
		expect(formatUrlNumber(0.0125)).toBe('0.0125');
		expect(formatUrlNumber(1000000)).toBe('1000000');
	});
});

describe('query paths', () => {
	it('build in the legacy segment order and round-trip through parse', () => {
		const search = {
			...defaultUiSearch(),
			typeSlug: '50mn-abyssal-microwarpdrive',
			metaGroup: 't2',
			attributes: [{ name: 'capacitorNeed', lower: 200.0, upper: 240.5 }],
			sort: ['price', true] as [string, boolean],
			contractType: 'auction',
			price: [1000000.0, null] as [number, number | null],
			goldbar: true,
			page: 3
		};

		const path = buildQueryPath('modules', search);
		expect(path).toBe(
			'/modules/type/50mn-abyssal-microwarpdrive/meta-group/t2' +
				'/attributes/capacitorneed/200-240.5/sort/price/desc/auction' +
				'/contract-price/1000000.00/goldbar/page/3'
		);

		// Parsing the built path recovers the same search (names come back
		// as they appear in the URL).
		const parsed = parseQueryUi(path.replace(/^\/modules\//, ''));
		expect(parsed.typeSlug).toBe('50mn-abyssal-microwarpdrive');
		expect(parsed.metaGroup).toBe('t2');
		expect(parsed.sort).toEqual(['price', true]);
		expect(parsed.contractType).toBe('auction');
		expect(parsed.price).toEqual([1000000.0, null]);
		expect(parsed.goldbar).toBe(true);
		expect(parsed.page).toBe(3);
		expect(parsed.attributes).toEqual([{ name: 'capacitorneed', lower: 200.0, upper: 240.5 }]);
	});

	it('parses flags, bounds and empty queries', () => {
		expect(parseQueryUi('')).toEqual(defaultUiSearch());
		expect(buildQueryPath('modules', defaultUiSearch())).toBe('/modules');

		const parsed = parseQueryUi(
			'type/x/item-exchange/no-multi-item-contracts/without-other-items' +
				'/estimated-value/100-5000/diamondbar/brownbar/contracts-only'
		);
		expect(parsed.typeSlug).toBe('x');
		expect(parsed.contractType).toBe('item_exchange');
		expect(parsed.noMultiItemContracts).toBe(true);
		expect(parsed.withoutOtherItems).toBe(true);
		expect(parsed.value).toEqual([100, 5000]);
		expect(parsed.diamondbar).toBe(true);
		expect(parsed.brownbar).toBe(true);
		expect(parsed.onlyContracts).toBe(true);
	});
});
