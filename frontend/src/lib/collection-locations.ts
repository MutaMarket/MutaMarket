// Sorting and parent resolution for the collection manage-modules
// dialog's location grid, the legacy CollectionLocationSettings.vue
// computed helpers plus the SortFunctions couldBeContainer check.

import type { CharacterLocationView } from './types-social';

export type SortField = 'name' | 'type' | 'station' | 'modules' | 'visibility' | 'container';
export type SortDirection = 'asc' | 'desc';

export interface LocationWithParent extends CharacterLocationView {
	parent?: CharacterLocationView;
}

/** Attaches each row's containing row (matching `location_id` against
 * the set's `item_id`s), like the legacy locations_with_parent. */
export function withParents(locations: CharacterLocationView[]): LocationWithParent[] {
	return locations.map((location) => ({
		...location,
		parent: locations.find((l) => l.item_id === location.location_id)
	}));
}

/** The legacy couldBeContainer: the type name mentions "container". */
export function couldBeContainer(location: CharacterLocationView): boolean {
	return (location.type_name ?? '').toLowerCase().includes('container');
}

/** One comparison of the legacy sorted_locations computed, quirks kept
 * (the visibility branch answers -1 whenever `a` has no public asset). */
export function compareLocations(
	a: LocationWithParent,
	b: LocationWithParent,
	field: SortField
): number {
	switch (field) {
		case 'container': {
			const aIsContainer = couldBeContainer(a);
			const bIsContainer = couldBeContainer(b);
			if (aIsContainer && !bIsContainer) return -1;
			if (!aIsContainer && bIsContainer) return 1;
			return 0;
		}
		case 'name':
			return a.name?.localeCompare(b.name ?? '') ?? 0;
		case 'type':
			return a.type_name?.localeCompare(b.type_name ?? '') ?? 0;
		case 'modules':
			return a.modules_count - b.modules_count;
		case 'visibility':
			if (a.public_asset_id && b.public_asset_id) return 0;
			return a.public_asset_id ? 1 : -1;
		case 'station':
		default:
			return a.station?.name.localeCompare(b.station?.name ?? '') ?? 0;
	}
}

/** The legacy sorted_locations: sort by the field, then reverse the
 * whole list for descending (exactly like the Vue computed did). */
export function sortLocations(
	locations: LocationWithParent[],
	field: SortField,
	direction: SortDirection
): LocationWithParent[] {
	const sorted = [...locations].sort((a, b) => compareLocations(a, b, field));
	return direction === 'desc' ? sorted.reverse() : sorted;
}

/** The legacy handleSort: same field flips direction, a new field
 * starts ascending. */
export function nextSort(
	field: SortField,
	direction: SortDirection,
	clicked: SortField
): { field: SortField; direction: SortDirection } {
	if (field === clicked) {
		return { field, direction: direction === 'asc' ? 'desc' : 'asc' };
	}
	return { field: clicked, direction: 'asc' };
}
