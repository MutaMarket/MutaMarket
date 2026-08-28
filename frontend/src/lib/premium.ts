// Helpers of the premium sales page, the legacy
// Premium/ShowPremiumPage.vue computed props.

import type { ModuleDetail } from './types';
import { PREMIUM_MONTHLY_ISK, PREMIUM_YEARLY_ISK } from './sidebar';

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
export function yearlySavings(): number {
	return PREMIUM_MONTHLY_ISK * 12 - PREMIUM_YEARLY_ISK;
}
