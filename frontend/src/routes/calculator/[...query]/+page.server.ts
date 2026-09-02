import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { loadPageFilters } from '$lib/server/page-filters';
import type { ProbabilityRow } from '$lib/calculator';

// The mutation calculator (legacy CalculatorController): the filter
// panel plus one probability row per (mutaplasmid, source type)
// combination; null until a category is picked.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const query = params.query ?? '';
  const [filters, probability] = await Promise.all([
    loadPageFilters(fetch, query),
    apiGet<ProbabilityRow[] | null>(
      fetch,
      query === '' ? '/api/calculator' : `/api/calculator/${query}`,
    ),
  ]);
  return { ...filters, probability };
};
