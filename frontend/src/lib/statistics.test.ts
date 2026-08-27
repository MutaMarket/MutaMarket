import { describe, expect, it } from 'vitest';
import {
	filterPersonalRows,
	pageCount,
	sortPersonalRows,
	syncLabel,
	type PersonalStatRow
} from './statistics';

function row(typeName: string, creatorName: string, count: number): PersonalStatRow {
	return {
		type: { id: 1, name: typeName },
		creator: { id: 2, name: creatorName },
		count
	};
}

const rows = [
	row('Abyssal Stasis Webifier', 'Alice', 3),
	row('Abyssal Warp Scrambler', 'Bob', 5),
	row('50MN Abyssal Microwarpdrive', 'Alice', 5)
];

describe('filterPersonalRows', () => {
	it('matches type and creator names case-insensitively', () => {
		expect(filterPersonalRows(rows, 'webifier')).toHaveLength(1);
		expect(filterPersonalRows(rows, 'ALICE')).toHaveLength(2);
		expect(filterPersonalRows(rows, '  ')).toHaveLength(3);
	});
});

describe('sortPersonalRows', () => {
	it('sorts by count with stable name tie-breaks', () => {
		const sorted = sortPersonalRows(rows, 'count', false);
		expect(sorted.map((r) => r.count)).toEqual([5, 5, 3]);
		expect(sorted[0].type.name).toBe('50MN Abyssal Microwarpdrive');
	});

	it('sorts by names in both directions without mutating', () => {
		expect(sortPersonalRows(rows, 'creator', true)[0].creator.name).toBe('Alice');
		expect(sortPersonalRows(rows, 'type', false)[0].type.name).toBe('Abyssal Warp Scrambler');
		expect(rows[0].type.name).toBe('Abyssal Stasis Webifier');
	});
});

describe('pageCount', () => {
	it('rounds up and never drops below one page', () => {
		expect(pageCount(0, 15)).toBe(1);
		expect(pageCount(15, 15)).toBe(1);
		expect(pageCount(16, 15)).toBe(2);
	});
});

describe('syncLabel', () => {
	it('renders the UTC clock time of the refresh stamp', () => {
		expect(syncLabel('2026-08-28T12:45:03Z')).toBe('12:45 UTC');
		expect(syncLabel('garbage')).toBe('garbage');
	});
});
