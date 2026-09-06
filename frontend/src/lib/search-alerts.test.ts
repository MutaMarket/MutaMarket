import { describe, expect, it } from 'vitest';

import { toIsk } from './format-number';
import { defaultUiSearch } from './query';
import { alertCriteria, alertOptions, alertQuery, alertSearchPath } from './search-alerts';

describe('alertQuery', () => {
  it('drops the page, the sort and the viewer-relative options like the server', () => {
    expect(
      alertQuery({
        ...defaultUiSearch(),
        typeSlug: '47408',
        goldbar: true,
        sort: ['price', false],
        withPersonalModules: true,
        withoutFitted: true,
        page: 3,
      }),
    ).toBe('type/47408/goldbar');
    expect(alertQuery(defaultUiSearch())).toBe('');
    expect(
      alertQuery({
        ...defaultUiSearch(),
        typeSlug: '47408',
        attributes: [{ name: 'cpu', lower: 10, upper: 20 }],
        price: [1_000_000, null],
        inJita: true,
      }),
    ).toBe('type/47408/attributes/cpu/10-20/contract-price/1000000.00/in-jita');
  });
});

describe('alert helpers', () => {
  const alert = {
    id: 1,
    query: 'type/47408/goldbar/in-jita',
    type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
    created_at: '2026-09-06T10:00:00Z',
    last_notified_at: null,
    notified_count: 0,
  };

  it('links back to the browser page, newest listing first', () => {
    expect(alertSearchPath(alert)).toBe('/modules/type/47408/sort/date-added/desc/goldbar/in-jita');
    expect(alertSearchPath({ ...alert, query: '' })).toBe('/modules/sort/date-added/desc');
  });

  it('lists the options after the type', () => {
    expect(alertOptions(alert)).toEqual(['goldbar', 'in-jita']);
    expect(alertOptions({ ...alert, query: 'type/47408' })).toEqual([]);
  });
});

describe('alertCriteria', () => {
  const base = {
    id: 1,
    query: '',
    type: null,
    created_at: '2026-09-06T10:00:00Z',
    last_notified_at: null,
    notified_count: 0,
  };

  it('spells out every option with the server bound semantics', () => {
    expect(
      alertCriteria({
        ...base,
        query:
          'type/47408/meta-group/faction/meta-level/8/attributes/cpu/10-20/mass/5/auction' +
          '/contract-price/1000000.00-5000000.00/estimated-value/250000000.00' +
          '/no-multi-item-contracts/contracts-only/goldbar/without-other-items/in-jita/search/gist',
      }),
    ).toEqual([
      { key: 'metaGroup', params: { name: 'Faction' } },
      { key: 'metaLevel', params: { level: '8' } },
      { key: 'attributeRange', params: { name: 'cpu', min: 10, max: 20 } },
      { key: 'attributeMin', params: { name: 'mass', min: 5 } },
      { key: 'auction' },
      { key: 'priceRange', params: { min: toIsk(1_000_000), max: toIsk(5_000_000) } },
      { key: 'valueMin', params: { min: toIsk(250_000_000) } },
      { key: 'singleItem' },
      { key: 'onlyContracts' },
      { key: 'goldbar' },
      { key: 'withoutOtherItems' },
      { key: 'inJita' },
      { key: 'searchTerm', params: { term: 'gist' } },
    ]);
  });

  it('treats a lone contract price as a maximum and a bare type as no criteria', () => {
    expect(alertCriteria({ ...base, query: 'type/47408/contract-price/900.00' })).toEqual([
      { key: 'priceMax', params: { max: toIsk(900) } },
    ]);
    expect(alertCriteria({ ...base, query: 'type/47408' })).toEqual([]);
  });
});
