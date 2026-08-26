// Shared load logic of the module browser pages (home, /modules and
// /all-modules): the card set and, when the query
// carries a type, the filter panel bounds. Search failures become error
// pages carrying the legacy message and status.

import { apiGet } from './api';
import { parseQueryUi } from '$lib/query';
import type { FilterPanelData, ModuleDetail, ModulesStats } from '$lib/types';

export interface BrowserData {
	prefix: string;
	query: string;
	modules: ModuleDetail[];
	panel: FilterPanelData | null;
	unknownType: boolean;
	/** Market/archive totals for the page header; null when the fetch
	 * degrades. */
	stats: ModulesStats | null;
}

export async function loadBrowser(
	fetch: typeof globalThis.fetch,
	query: string,
	unlisted: boolean
): Promise<BrowserData> {
	const search = parseQueryUi(query);

	const cardsPath = query === '' ? '/api/module-cards' : `/api/module-cards/${query}`;
	const [modules, panel, stats] = await Promise.all([
		apiGet<ModuleDetail[]>(fetch, unlisted ? `${cardsPath}?unlisted=true` : cardsPath),
		// The panel degrades to absent instead of failing the page.
		search.typeSlug === null
			? Promise.resolve(null)
			: fetch(`/api/filter-panel/${search.typeSlug}`)
					.then((response) =>
						response.ok ? (response.json() as Promise<FilterPanelData>) : null
					)
					.catch(() => null),
		// The header stats degrade the same way.
		fetch(`/api/module-stats?unlisted=${unlisted}`)
			.then((response) => (response.ok ? (response.json() as Promise<ModulesStats>) : null))
			.catch(() => null)
	]);

	return {
		prefix: unlisted ? 'all-modules' : 'modules',
		query,
		modules,
		panel,
		unknownType: search.typeSlug !== null && panel === null,
		stats,
	};
}
