// The search carried over when switching types, the exact prop subset of
// the legacy `TypeCategory.getTypeLink`: contract type and the boolean
// flags survive; attributes, sort, meta and price bounds reset. Clicking
// the already-selected type clears the type.

import { defaultUiSearch, type UiSearch } from './query';

export function typeSwitchSearch(
	current: UiSearch,
	currentTypeId: number | null,
	target: number
): UiSearch {
	return {
		...defaultUiSearch(),
		typeSlug: currentTypeId === target ? null : String(target),
		contractType: current.contractType,
		onlyContracts: current.onlyContracts,
		noMultiItemContracts: current.noMultiItemContracts,
		goldbar: current.goldbar,
		brownbar: current.brownbar,
		diamondbar: current.diamondbar,
		withPersonalModules: current.withPersonalModules,
		inJita: current.inJita,
		created: current.created,
		withoutFitted: current.withoutFitted,
		withoutAssets: current.withoutAssets
	};
}
