// Shared filter support of the scoped module pages (character,
// collection, personal): parse the query segment, fetch the filter
// panel when a type is selected.
import { parseQueryUi, type UiSearch } from '$lib/query';
import type { FilterPanelData } from '$lib/types';

export interface PageFilterData {
	query: string;
	panel: FilterPanelData | null;
	unknownType: boolean;
}

export async function loadPageFilters(
	fetch: typeof globalThis.fetch,
	query: string
): Promise<PageFilterData> {
	const search: UiSearch = parseQueryUi(query);
	const panel =
		search.typeSlug === null
			? null
			: await fetch(`/api/filter-panel/${search.typeSlug}`)
					.then((response) =>
						response.ok ? (response.json() as Promise<FilterPanelData>) : null
					)
					.catch(() => null);
	return { query, panel, unknownType: search.typeSlug !== null && panel === null };
}
