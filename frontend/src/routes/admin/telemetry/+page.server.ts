import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';
import type { LivePayload } from '#lib/admin-live.svelte.ts';

export const load: PageServerLoad = async ({ fetch }) => {
  const live = await apiGet<LivePayload>(fetch, '/api/admin/live?sections=telemetry,failures');
  return { live };
};
