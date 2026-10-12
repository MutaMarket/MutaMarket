import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { LivePayload } from '#lib/admin-live.svelte.js';
import type { ActivityHistory } from '#lib/admin-types.js';

export const load: PageServerLoad = async ({ fetch }) => {
  const [live, history] = await Promise.all([
    apiGet<LivePayload>(fetch, '/api/admin/live?sections=activity'),
    apiGet<ActivityHistory>(fetch, '/api/admin/activity?window=24h'),
  ]);

  return { live, history };
};
