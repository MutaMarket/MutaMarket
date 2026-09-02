import { describe, expect, it } from 'vitest';
import { filterRows, oneIn, sortRows, toPercentage, type ProbabilityRow } from './calculator';

function row(
  typeName: string,
  mutaplasmidName: string,
  probability: number,
  cost: number | null,
): ProbabilityRow {
  return {
    type: { id: 1, name: typeName },
    mutaplasmid: { id: 2, name: mutaplasmidName },
    probability,
    cost,
    cost_mutaplasmid: 0,
    cost_type: null,
  };
}

describe('toPercentage', () => {
  it('formats the legacy percent readout', () => {
    expect(toPercentage(0.5)).toBe('50%');
    expect(toPercentage(0.42345)).toBe('42.35%');
    expect(toPercentage(1)).toBe('100%');
  });
});

describe('oneIn', () => {
  it('rounds the expected attempt count', () => {
    expect(oneIn(0.5)).toBe('1 in 2');
    expect(oneIn(0.3)).toBe('1 in 3');
    expect(oneIn(0.0001)).toBe('1 in 10000');
  });
});

describe('filterRows', () => {
  const rows = [
    row('Khanid Navy Stasis Webifier', 'Decayed Stasis Webifier Mutaplasmid', 1, null),
    row('Fleeting Compact Stasis Webifier', 'Gravid Stasis Webifier Mutaplasmid', 1, null),
  ];

  it('matches type and mutaplasmid names case-insensitively', () => {
    expect(filterRows(rows, 'khanid')).toHaveLength(1);
    expect(filterRows(rows, 'GRAVID')).toHaveLength(1);
    expect(filterRows(rows, 'webifier')).toHaveLength(2);
  });

  it('returns everything for a blank needle', () => {
    expect(filterRows(rows, '  ')).toHaveLength(2);
  });
});

describe('sortRows', () => {
  const rows = [
    row('B-type', 'M1', 0.2, 100),
    row('A-type', 'M2', 0, null),
    row('C-type', 'M3', 0.8, 50),
  ];

  it('sorts names in both directions', () => {
    expect(sortRows(rows, 'type', true).map((r) => r.type.name)).toEqual([
      'A-type',
      'B-type',
      'C-type',
    ]);
    expect(sortRows(rows, 'type', false)[0].type.name).toBe('C-type');
  });

  it('sinks impossible rolls and unknown costs to the end', () => {
    expect(sortRows(rows, 'probability', true).map((r) => r.probability)).toEqual([0.2, 0.8, 0]);
    expect(sortRows(rows, 'probability', false).map((r) => r.probability)).toEqual([0.8, 0.2, 0]);
    expect(sortRows(rows, 'cost', true).map((r) => r.cost)).toEqual([50, 100, null]);
    expect(sortRows(rows, 'cost', false).map((r) => r.cost)).toEqual([100, 50, null]);
  });

  it('does not mutate the input', () => {
    const before = rows.map((r) => r.type.name);
    sortRows(rows, 'type', true);
    expect(rows.map((r) => r.type.name)).toEqual(before);
  });
});
