// Shared load logic of the module browser pages (home, /modules,
// /all-modules and the premium /historic-sales): the card set and, when
// the query carries a type, the filter panel bounds. Search failures
// become error pages carrying the legacy message and status; the
// historic page's premium 403 becomes the legacy /premium redirect.

import { redirect } from '@sveltejs/kit';
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
	unlisted: boolean,
	historic = false
): Promise<BrowserData> {
	const search = parseQueryUi(query);

	const base = historic ? '/api/historic-sales-cards' : '/api/module-cards';
	const cardsPath = query === '' ? base : `${base}/${query}`;
	const loadCards = async (): Promise<ModuleDetail[]> => {
		if (!historic) {
			return apiGet<ModuleDetail[]>(fetch, unlisted ? `${cardsPath}?unlisted=true` : cardsPath);
		}
		// The legacy PremiumMiddleware sends guests to the login page and
		// everyone else without premium to the premium page.
		const response = await fetch(cardsPath);
		if (response.status === 401) {
			redirect(303, '/login');
		}
		if (response.status === 403) {
			redirect(303, '/premium');
		}
		if (!response.ok) {
			return apiGet<ModuleDetail[]>(fetch, cardsPath);
		}
		return response.json();
	};
	const [modules, panel, stats] = await Promise.all([
		loadCards(),
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
		prefix: historic ? 'historic-sales' : unlisted ? 'all-modules' : 'modules',
		query,
		modules,
		panel,
		unknownType: search.typeSlug !== null && panel === null,
		stats,
	};
}
