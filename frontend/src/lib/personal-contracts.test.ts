import { describe, expect, it } from 'vitest';
import {
	contractTotals,
	matchesSearch,
	mergeContracts,
	type PersonalContractEntry
} from './personal-contracts';
import type { CharacterRef, ModuleDetail, TypeRef } from './types';

function character(id: number, name: string): CharacterRef {
	return { id, slug: `${name.toLowerCase()}-${id}`, name, description: null, has_premium: false, corporation_id: null };
}

function moduleCard(id: number, typeName: string): ModuleDetail {
	return {
		id,
		type: { id: 47702, name: typeName },
		creator: null,
		mutated_attributes: [],
		source_type: null,
		mutaplasmid: null,
		contract: null,
		estimated_value: null,
		estimated_value_updated_at: null,
		public_asset: null,
		slug: `${typeName.toLowerCase()}-${id}`,
		average_fraction: null
	} as ModuleDetail;
}

function entry(overrides: Partial<PersonalContractEntry> & { id: number }): PersonalContractEntry {
	return {
		type: 'item_exchange',
		price: 100,
		asking_for_items: false,
		plex_count: 0,
		non_abyssal_modules_count: 0,
		abyssal_modules_count: 1,
		issuer: character(1, 'Alice'),
		date_issued: '2026-08-01 10:00:00+00',
		date_expired: '2026-08-15 10:00:00+00',
		...overrides
	};
}

const webType: TypeRef = { id: 47702, name: 'Abyssal Web' };

describe('mergeContracts', () => {
	it('merges entries sharing an id across the sources', () => {
		const acceptor = character(9, 'Buyer');
		const merged = mergeContracts(
			[
				entry({ id: 5, modules: [moduleCard(50, 'Abyssal Web')] }),
				entry({
					id: 5,
					status: 'completed',
					types: [webType],
					is_private: true,
					acceptor,
					acceptor_type: 'character',
					date_accepted: '2026-08-03 10:00:00+00'
				})
			],
			[]
		);
		expect(merged).toHaveLength(1);
		expect(merged[0].status).toBe('completed');
		expect(merged[0].is_private).toBe(true);
		expect(merged[0].acceptor).toEqual(acceptor);
		expect(merged[0].date_accepted).toBe('2026-08-03 10:00:00+00');
		// The public entry's full cards win over the bare types.
		expect(merged[0].modules).toEqual([moduleCard(50, 'Abyssal Web')]);
	});

	it('keeps the legacy quirk: a types fallback never marks modules found', () => {
		const merged = mergeContracts(
			[
				entry({ id: 5, types: [webType], status: 'completed' }),
				entry({ id: 5, modules: [moduleCard(50, 'Abyssal Web')] })
			],
			[]
		);
		expect(merged[0].modules).toEqual([moduleCard(50, 'Abyssal Web')]);
	});

	it('keeps the legacy quirk: only the first entry decides accepted_by_user', () => {
		const acceptor = character(9, 'Mine');
		const groups = [
			entry({ id: 5, modules: [] }),
			entry({ id: 5, status: 'completed', acceptor, acceptor_type: 'character' })
		];
		// The first entry carries no acceptor, so even the owner's own
		// character does not count as accepted_by_user.
		expect(mergeContracts(groups, [9])[0].accepted_by_user).toBe(false);
		expect(mergeContracts([groups[1]], [9])[0].accepted_by_user).toBe(true);
		expect(mergeContracts([groups[1]], [8])[0].accepted_by_user).toBe(false);
	});

	it('keeps the legacy quirk: a later status-less entry resets to outstanding', () => {
		const merged = mergeContracts(
			[entry({ id: 5, status: 'completed' }), entry({ id: 5 })],
			[]
		);
		expect(merged[0].status).toBe('outstanding');
	});

	it('sorts outstanding first, then newest id', () => {
		const merged = mergeContracts(
			[
				entry({ id: 1, status: 'completed' }),
				entry({ id: 2 }),
				entry({ id: 3, status: 'failed' }),
				entry({ id: 4 })
			],
			[]
		);
		expect(merged.map((contract) => contract.id)).toEqual([4, 2, 3, 1]);
	});
});

describe('contractTotals', () => {
	it('splits completed contracts into earnings and spendings', () => {
		const acceptor = character(9, 'Buyer');
		const totals = contractTotals(
			mergeContracts(
				[
					entry({ id: 1, status: 'completed', price: 300 }),
					entry({ id: 2, status: 'completed', price: 100, acceptor, acceptor_type: 'character' }),
					entry({ id: 3, price: 40 }),
					entry({ id: 4, price: 60 }),
					entry({ id: 5, status: 'failed', price: 1000 })
				],
				[9]
			)
		);
		expect(totals.earnings).toBe(300);
		expect(totals.spent).toBe(100);
		expect(totals.profit).toBe(200);
		expect(totals.outstandingValue).toBe(100);
		expect(totals.outstandingCount).toBe(2);
	});

	it('treats null prices as zero', () => {
		const totals = contractTotals(mergeContracts([entry({ id: 1, price: null })], []));
		expect(totals.outstandingValue).toBe(0);
		expect(totals.outstandingCount).toBe(1);
	});
});

describe('matchesSearch', () => {
	const merged = mergeContracts(
		[
			entry({
				id: 5,
				issuer: character(1, 'Alice'),
				acceptor: character(9, 'Buyer'),
				acceptor_type: 'character',
				status: 'completed',
				types: [webType]
			})
		],
		[]
	)[0];

	it('matches issuer, acceptor and module names, case-insensitively', () => {
		expect(matchesSearch(merged, '')).toBe(true);
		expect(matchesSearch(merged, 'ali')).toBe(true);
		expect(matchesSearch(merged, 'BUYER')).toBe(true);
		expect(matchesSearch(merged, 'abyssal web')).toBe(true);
		expect(matchesSearch(merged, 'gyrostabilizer')).toBe(false);
	});
});
