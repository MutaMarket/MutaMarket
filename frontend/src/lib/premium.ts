// Helpers of the premium sales page (the legacy
// Premium/ShowPremiumPage.vue computed props) and the shared premium
// config the legacy AppData middleware exposed globally, served here
// through /api/sidebar.

import type { ModuleDetail } from './types';

/** The legacy AppData shared props: config app.premium_character /
 * app.premium_cost / app.premium_yearly_cost (env-overridable on the
 * backend). */
export interface PremiumConfig {
	premium_character: string;
	premium_cost: number;
	premium_yearly_cost: number;
}

/** The backend config defaults, used only when /api/sidebar is
 * unreachable (the pages degrade like the donations lists). */
export const DEFAULT_PREMIUM: PremiumConfig = {
	premium_character: 'MutaMate',
	premium_cost: 100_000_000,
	premium_yearly_cost: 1_000_000_000
};

/** The shared premium config out of a sidebar payload. */
export function premiumFromSidebar(
	payload: Partial<PremiumConfig> | null | undefined
): PremiumConfig {
	return {
		premium_character: payload?.premium_character ?? DEFAULT_PREMIUM.premium_character,
		premium_cost: payload?.premium_cost ?? DEFAULT_PREMIUM.premium_cost,
		premium_yearly_cost: payload?.premium_yearly_cost ?? DEFAULT_PREMIUM.premium_yearly_cost
	};
}

/** Columns of the falling hero backdrop. */
export const HERO_COLUMNS = 3;

/** The legacy hero_columns computed: the sample modules dealt
 * round-robin into three columns, empty columns dropped. */
export function heroColumns(modules: ModuleDetail[]): ModuleDetail[][] {
	const columns: ModuleDetail[][] = Array.from({ length: HERO_COLUMNS }, () => []);
	modules.forEach((module, index) => {
		columns[index % HERO_COLUMNS].push(module);
	});
	return columns.filter((column) => column.length > 0);
}

/** The legacy yearly_savings computed: two free months. */
export function yearlySavings(premium: PremiumConfig): number {
	return premium.premium_cost * 12 - premium.premium_yearly_cost;
}
