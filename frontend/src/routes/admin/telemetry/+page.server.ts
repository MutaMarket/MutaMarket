import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { LivePayload } from '$lib/admin-live.svelte';

export const load: PageServerLoad = async ({ fetch }) => {
  const live = await apiGet<LivePayload>(fetch, '/api/admin/live?sections=telemetry,failures');
  return { live };
};
