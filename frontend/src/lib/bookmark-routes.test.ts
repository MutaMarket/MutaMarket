import { describe, expect, it } from 'vitest';
import { routePriority, sortBookmarks } from './bookmark-routes';

describe('sortBookmarks', () => {
	it('orders by the legacy category priority, then name', () => {
		const sorted = sortBookmarks([
			{ id: 1, name: 'Zed settings', query: '/settings', type_id: null },
			{ id: 2, name: 'Webs', query: '/modules/type/47702', type_id: 47702 },
			{ id: 3, name: 'Alpha chars', query: '/characters', type_id: null },
			{ id: 4, name: 'Afterburners', query: '/modules/type/47749', type_id: 47749 },
			{ id: 5, name: 'Somewhere odd', query: '/unknown-path', type_id: null },
		]);
		expect(sorted.map((bookmark) => bookmark.id)).toEqual([4, 2, 3, 1, 5]);
	});

	it('falls back to last place for unknown routes', () => {
		expect(routePriority('/modules/type/x')).toBe(0);
		expect(routePriority('/personal/contracts')).toBe(9);
		expect(routePriority('/whatever')).toBe(99);
	});
});
