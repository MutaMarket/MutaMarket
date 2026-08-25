// Shared load logic of the module browser pages (home, /modules and
// /all-modules): the card set, the market stats and, when the query
// carries a type, the filter panel bounds. Search failures become error
// pages carrying the legacy message and status.

import { apiGet } from './api';
import { parseQueryUi } from '$lib/query';
import type { FilterPanelData, ModuleDetail, ModulesStats } from '$lib/types';

export interface BrowserData {
	prefix: string;
	query: string;
	modules: ModuleDetail[];
	stats: ModulesStats | null;
	panel: FilterPanelData | null;
	unknownType: boolean;
}

export async function loadBrowser(
	fetch: typeof globalThis.fetch,
	query: string,
	unlisted: boolean
): Promise<BrowserData> {
	const search = parseQueryUi(query);

	const cardsPath = query === '' ? '/api/module-cards' : `/api/module-cards/${query}`;
	const [modules, stats, panel] = await Promise.all([
		apiGet<ModuleDetail[]>(fetch, unlisted ? `${cardsPath}?unlisted=true` : cardsPath),
		// The strip and the panel degrade to absent instead of failing the
		// page.
		fetch('/api/module-stats')
			.then((response) => (response.ok ? (response.json() as Promise<ModulesStats>) : null))
			.catch(() => null),
		search.typeSlug === null
			? Promise.resolve(null)
			: fetch(`/api/filter-panel/${search.typeSlug}`)
					.then((response) =>
						response.ok ? (response.json() as Promise<FilterPanelData>) : null
					)
					.catch(() => null)
	]);

	return {
		prefix: unlisted ? 'all-modules' : 'modules',
		query,
		modules,
		stats,
		panel,
		unknownType: search.typeSlug !== null && panel === null,
	};
}
