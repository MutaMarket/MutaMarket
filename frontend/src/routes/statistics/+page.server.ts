import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { StatisticsOverview } from '#lib/statistics.js';

// The statistics overview tab: the materialized market-wide totals.
export const load: PageServerLoad = async ({ fetch }) => ({
  overview: await apiGet<StatisticsOverview>(fetch, '/api/statistics/overview'),
});
