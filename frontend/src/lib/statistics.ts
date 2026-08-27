// The unified statistics page's data shapes and pure logic: the
// market overview, the top-creators leaderboard and the personal
// creation stats (client-side search/sort like the legacy BaseTable).

import type { ModulesStats } from './types';

export interface StatisticsOverview {
	stats: ModulesStats;
	total_value: number;
	average_value: number;
	creators_count: number;
	characters_count: number;
}

export interface TopCharacterRow {
	id: number;
	name: string;
	modules_created_count: number;
	rank_number: number;
}

export interface TopCharacters {
	data: TopCharacterRow[];
	meta: { current_page: number; per_page: number; total: number };
}

export interface PersonalStatRow {
	type: { id: number; name: string };
	creator: { id: number; name: string };
	count: number;
}

export interface PersonalStats {
	stats: PersonalStatRow[];
	total_modules: number;
	total_value: number;
	total_spent: number;
}

export type PersonalSortKey = 'type' | 'creator' | 'count';

/** Case-insensitive match on the type or creator name. */
export function filterPersonalRows(rows: PersonalStatRow[], needle: string): PersonalStatRow[] {
	const query = needle.trim().toLowerCase();
	if (query === '') {
		return rows;
	}
	return rows.filter(
		(row) =>
			row.type.name.toLowerCase().includes(query) ||
			row.creator.name.toLowerCase().includes(query)
	);
}

/** Sorts a copy by the column; count ties fall back to the names so the
 * order stays stable. */
export function sortPersonalRows(
	rows: PersonalStatRow[],
	key: PersonalSortKey,
	ascending: boolean
): PersonalStatRow[] {
	const direction = ascending ? 1 : -1;
	return [...rows].sort((a, b) => {
		if (key === 'count') {
			return (
				direction * (a.count - b.count) ||
				a.type.name.localeCompare(b.type.name) ||
				a.creator.name.localeCompare(b.creator.name)
			);
		}
		return direction * a[key].name.localeCompare(b[key].name);
	});
}

/** Total pages of the leaderboard pagination. */
export function pageCount(total: number, perPage: number): number {
	return Math.max(1, Math.ceil(total / perPage));
}
