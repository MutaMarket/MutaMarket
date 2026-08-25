// Lazy client-side cache of `/api/abyssal-type-statistics`, standing in
// for the legacy client-bundled AbyssalTypeAttributes static: the module
// menus need per-type roll extremes wherever a card renders, not just on
// the show page.
import type { AbyssalTypeStatistic } from './types';

type ApiRow = AbyssalTypeStatistic & { type_id: number };

let cache: Promise<ApiRow[]> | null = null;

function allStatistics(): Promise<ApiRow[]> {
	cache ??= fetch('/api/abyssal-type-statistics')
		.then((response) => response.json() as Promise<ApiRow[]>)
		.catch(() => {
			cache = null;
			return [];
		});
	return cache;
}

export async function typeStatistics(typeId: number): Promise<AbyssalTypeStatistic[]> {
	const rows = await allStatistics();
	return rows.filter((row) => row.type_id === typeId);
}
