import { describe, expect, it } from 'vitest';
import {
  CONTRACT_COLUMNS,
  compareContracts,
  contractTotals,
  matchesSearch,
  mergeContracts,
  sortContracts,
  type MergedContract,
  type PersonalContractEntry,
} from './personal-contracts';
import type { CharacterRef, ModuleDetail, TypeRef } from './types';

function character(id: number, name: string): CharacterRef {
  return {
    id,
    slug: `${name.toLowerCase()}-${id}`,
    name,
    description: null,
    has_premium: false,
    corporation_id: null,
  };
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
    average_fraction: null,
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
    ...overrides,
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
          date_accepted: '2026-08-03 10:00:00+00',
        }),
      ],
      [],
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
        entry({ id: 5, modules: [moduleCard(50, 'Abyssal Web')] }),
      ],
      [],
    );
    expect(merged[0].modules).toEqual([moduleCard(50, 'Abyssal Web')]);
  });

  it('keeps the legacy quirk: only the first entry decides accepted_by_user', () => {
    const acceptor = character(9, 'Mine');
    const groups = [
      entry({ id: 5, modules: [] }),
      entry({ id: 5, status: 'completed', acceptor, acceptor_type: 'character' }),
    ];
    // The first entry carries no acceptor, so even the owner's own
    // character does not count as accepted_by_user.
    expect(mergeContracts(groups, [9])[0].accepted_by_user).toBe(false);
    expect(mergeContracts([groups[1]], [9])[0].accepted_by_user).toBe(true);
    expect(mergeContracts([groups[1]], [8])[0].accepted_by_user).toBe(false);
  });

  it('keeps the legacy quirk: a later status-less entry resets to outstanding', () => {
    const merged = mergeContracts([entry({ id: 5, status: 'completed' }), entry({ id: 5 })], []);
    expect(merged[0].status).toBe('outstanding');
  });

  it('sorts outstanding first, then newest id', () => {
    const merged = mergeContracts(
      [
        entry({ id: 1, status: 'completed' }),
        entry({ id: 2 }),
        entry({ id: 3, status: 'failed' }),
        entry({ id: 4 }),
      ],
      [],
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
          entry({ id: 5, status: 'failed', price: 1000 }),
        ],
        [9],
      ),
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
        types: [webType],
      }),
    ],
    [],
  )[0];

  it('matches issuer, acceptor and module names, case-insensitively', () => {
    expect(matchesSearch(merged, '')).toBe(true);
    expect(matchesSearch(merged, 'ali')).toBe(true);
    expect(matchesSearch(merged, 'BUYER')).toBe(true);
    expect(matchesSearch(merged, 'abyssal web')).toBe(true);
    expect(matchesSearch(merged, 'gyrostabilizer')).toBe(false);
  });
});

/** A merged row with only the fields the comparators read. */
function row(overrides: Partial<MergedContract> & { id: number }): MergedContract {
  return {
    type: 'item_exchange',
    price: 0,
    asking_for_items: false,
    plex_count: 0,
    non_abyssal_modules_count: 0,
    abyssal_modules_count: 0,
    issuer: null,
    acceptor: null,
    date_issued: '2026-08-01 10:00:00+00',
    date_expired: '2026-09-01 10:00:00+00',
    date_accepted: null,
    status: null,
    modules: [],
    accepted_by_user: false,
    found_modules: false,
    ...overrides,
  } as MergedContract;
}

describe('CONTRACT_COLUMNS', () => {
  it('keeps the legacy column order, with modules unsortable', () => {
    expect(CONTRACT_COLUMNS.map((column) => column.key)).toEqual([
      'issuer',
      'acceptor',
      'date_issued',
      'date_accepted',
      'date_expired',
      'status',
      'modules',
      'price',
    ]);
    expect(CONTRACT_COLUMNS.find((column) => column.key === 'modules')?.sortable).toBe(false);
  });
});

describe('compareContracts', () => {
  it('sorts names, treating a missing one as empty', () => {
    const alice = row({ id: 1, issuer: character(1, 'Alice') });
    const bob = row({ id: 2, issuer: character(2, 'Bob') });
    const nobody = row({ id: 3 });
    expect(compareContracts('issuer', alice, bob)).toBeLessThan(0);
    expect(compareContracts('issuer', nobody, alice)).toBeLessThan(0);
    expect(compareContracts('acceptor', nobody, nobody)).toBe(0);
  });

  it('sorts dates oldest first, with a null date as the epoch', () => {
    const early = row({ id: 1, date_issued: '2026-08-01 10:00:00+00' });
    const late = row({ id: 2, date_issued: '2026-08-09 10:00:00+00' });
    expect(compareContracts('date_issued', early, late)).toBeLessThan(0);
    expect(compareContracts('date_expired', early, late)).toBe(0);
  });

  it('puts the newest acceptance first and the unaccepted last', () => {
    // The legacy quirk: this column alone sorts descending.
    const older = row({ id: 1, date_accepted: '2026-08-01 10:00:00+00' });
    const newer = row({ id: 2, date_accepted: '2026-08-09 10:00:00+00' });
    const never = row({ id: 3, date_accepted: null });
    expect(compareContracts('date_accepted', newer, older)).toBeLessThan(0);
    expect(compareContracts('date_accepted', never, older)).toBeGreaterThan(0);
    expect(compareContracts('date_accepted', never, never)).toBe(0);
  });

  it('keeps the legacy status comparator, non-transitive and all', () => {
    const outstanding = row({ id: 1, status: 'outstanding' });
    const other = row({ id: 2, status: 'outstanding' });
    const completed = row({ id: 3, status: 'completed' });
    const failed = row({ id: 4, status: 'failed' });

    expect(compareContracts('status', outstanding, completed)).toBe(-1);
    expect(compareContracts('status', completed, outstanding)).toBe(1);
    // Both outstanding: `a` wins either way round. Ported deliberately.
    expect(compareContracts('status', outstanding, other)).toBe(-1);
    expect(compareContracts('status', other, outstanding)).toBe(-1);
    expect(compareContracts('status', completed, failed)).toBe(1);
    // Neither outstanding nor completed: the id breaks the tie.
    expect(compareContracts('status', failed, row({ id: 9, status: 'failed' }))).toBe(-5);
  });

  it('sorts price with a missing one as zero, and ignores an unknown key', () => {
    const cheap = row({ id: 1, price: 10 });
    const free = row({ id: 2, price: null });
    expect(compareContracts('price', free, cheap)).toBeLessThan(0);
    expect(compareContracts('modules', cheap, free)).toBe(0);
    expect(compareContracts(null, cheap, free)).toBe(0);
  });
});

describe('sortContracts', () => {
  it('returns the merge order untouched when no column is chosen', () => {
    const rows = [row({ id: 3 }), row({ id: 1 })];
    expect(sortContracts(rows, null, false)).toBe(rows);
  });

  it('keeps ties in merge order in both directions', () => {
    // Reversing an ascending sort would flip these, and the legacy
    // table does not: the direction belongs inside the comparator.
    const rows = [row({ id: 1, price: 50 }), row({ id: 2, price: 50 }), row({ id: 3, price: 10 })];
    expect(sortContracts(rows, 'price', false).map((r) => r.id)).toEqual([3, 1, 2]);
    expect(sortContracts(rows, 'price', true).map((r) => r.id)).toEqual([1, 2, 3]);
  });

  it('inverts the order of rows the comparator does separate', () => {
    const rows = [row({ id: 1, price: 10 }), row({ id: 2, price: 30 })];
    expect(sortContracts(rows, 'price', false).map((r) => r.id)).toEqual([1, 2]);
    expect(sortContracts(rows, 'price', true).map((r) => r.id)).toEqual([2, 1]);
  });

  it('does not mutate the rows it was given', () => {
    const rows = [row({ id: 2, price: 30 }), row({ id: 1, price: 10 })];
    sortContracts(rows, 'price', false);
    expect(rows.map((r) => r.id)).toEqual([2, 1]);
  });
});

describe('the types fallback', () => {
  it('keeps repeated type ids, one entry per item row', () => {
    // A contract holding two of the same module type yields two
    // entries with that type id. The table must not key on it: doing
    // so crashed the page with each_key_duplicate.
    const merged = mergeContracts(
      [
        entry({
          id: 500,
          modules: undefined,
          types: [
            { id: 49730, name: 'Abyssal Warp Scrambler' },
            { id: 49730, name: 'Abyssal Warp Scrambler' },
            { id: 47702, name: 'Abyssal Heat Sink' },
          ],
        }),
      ],
      [1],
    );

    expect(merged).toHaveLength(1);
    const ids = merged[0].modules.map((module) => module.id);
    expect(ids).toEqual([49730, 49730, 47702]);
    // The merge must not dedupe: the count of each type is the point,
    // and it is what makes keying the table cell on the id wrong.
    expect(new Set(ids).size).toBeLessThan(ids.length);
  });
});
