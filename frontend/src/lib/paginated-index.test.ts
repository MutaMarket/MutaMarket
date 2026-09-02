import { describe, expect, it } from 'vitest';

import { indexQuery, pageParam } from './paginated-index';

describe('the index query helpers', () => {
  it('reads a page number and falls back to the first page', () => {
    expect(pageParam(new URLSearchParams('page=3'), 'page')).toBe(3);
    expect(pageParam(new URLSearchParams('page=0'), 'page')).toBe(1);
    expect(pageParam(new URLSearchParams('page=abc'), 'page')).toBe(1);
    expect(pageParam(new URLSearchParams(''), 'page_public')).toBe(1);
  });

  it('omits the first page and empty searches like the legacy paginators', () => {
    expect(indexQuery({ search: '', page: 1 })).toBe('');
    expect(indexQuery({ search: 'Col', page_public: 2, page: 1 })).toBe(
      '?search=Col&page_public=2',
    );
    expect(indexQuery({ personal: 'true', page: 3 })).toBe('?personal=true&page=3');
  });
});
