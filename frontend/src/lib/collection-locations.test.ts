import { describe, expect, it } from 'vitest';
import {
	couldBeContainer,
	nextSort,
	sortLocations,
	withParents,
	type LocationWithParent
} from './collection-locations';
import type { CharacterLocationView } from './types-social';

function location(overrides: Partial<CharacterLocationView>): CharacterLocationView {
	return {
		asset_id: 1,
		item_id: 1,
		name: null,
		type_id: 100,
		type_name: null,
		location_id: null,
		station: null,
		modules_count: 0,
		public_asset_id: null,
		corporation_id: null,
		slug: 'x-1',
		...overrides
	};
}

describe('withParents', () => {
	it('attaches the containing row by item id', () => {
		const ship = location({ asset_id: 10, item_id: 501, name: 'Ship' });
		const container = location({ asset_id: 11, item_id: 502, location_id: 501 });
		const [shipRow, containerRow] = withParents([ship, container]);
		expect(shipRow.parent).toBeUndefined();
		expect(containerRow.parent?.name).toBe('Ship');
	});
});

describe('couldBeContainer', () => {
	it('matches on the type name, case-insensitively', () => {
		expect(couldBeContainer(location({ type_name: 'Station Container' }))).toBe(true);
		expect(couldBeContainer(location({ type_name: 'Sigil' }))).toBe(false);
		expect(couldBeContainer(location({ type_name: null }))).toBe(false);
	});
});

describe('sortLocations', () => {
	const rows: LocationWithParent[] = withParents([
		location({
			asset_id: 1,
			item_id: 1,
			name: 'Zulu',
			type_name: 'Sigil',
			modules_count: 5,
			public_asset_id: 7,
			station: { id: 60, name: 'Amarr VIII', type_id: null, slug: 'amarr-viii-60' }
		}),
		location({
			asset_id: 2,
			item_id: 2,
			name: 'Alpha',
			type_name: 'Station Container',
			modules_count: 1,
			station: { id: 61, name: 'Jita IV', type_id: null, slug: 'jita-iv-61' }
		}),
		location({
			asset_id: 3,
			item_id: 3,
			name: 'Mike',
			type_name: 'Bestower',
			modules_count: 3,
			station: { id: 62, name: 'Dodixie IX', type_id: null, slug: 'dodixie-ix-62' }
		})
	]);

	it('puts containers first for the default field', () => {
		expect(sortLocations(rows, 'container', 'asc').map((row) => row.asset_id)).toEqual([2, 1, 3]);
	});

	it('sorts by name, both directions', () => {
		expect(sortLocations(rows, 'name', 'asc').map((row) => row.name)).toEqual([
			'Alpha',
			'Mike',
			'Zulu'
		]);
		expect(sortLocations(rows, 'name', 'desc').map((row) => row.name)).toEqual([
			'Zulu',
			'Mike',
			'Alpha'
		]);
	});

	it('sorts by module count and station name', () => {
		expect(sortLocations(rows, 'modules', 'asc').map((row) => row.modules_count)).toEqual([
			1, 3, 5
		]);
		expect(sortLocations(rows, 'station', 'asc').map((row) => row.station?.name)).toEqual([
			'Amarr VIII',
			'Dodixie IX',
			'Jita IV'
		]);
	});

	it('keeps the legacy visibility quirk: rows without a public asset always compare -1', () => {
		expect(sortLocations(rows, 'visibility', 'asc').map((row) => row.asset_id)).toEqual([
			3, 2, 1
		]);
	});
});

describe('nextSort', () => {
	it('flips direction on the same field, resets on a new one', () => {
		expect(nextSort('container', 'asc', 'container')).toEqual({
			field: 'container',
			direction: 'desc'
		});
		expect(nextSort('container', 'desc', 'container')).toEqual({
			field: 'container',
			direction: 'asc'
		});
		expect(nextSort('container', 'desc', 'name')).toEqual({ field: 'name', direction: 'asc' });
	});
});
