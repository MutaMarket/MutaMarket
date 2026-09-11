import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { LivePayload } from '$lib/admin-live.svelte';
import type { ServiceCharacter } from '$lib/admin-types';

// The database counts are deliberately not loaded here. They are count
// scans over the largest tables, and waiting for them held a client-side
// navigation to the console for seconds; the poll fills them in instead.
export const load: PageServerLoad = async ({ fetch }) => {
  const [live, service] = await Promise.all([
    apiGet<LivePayload>(fetch, '/api/admin/live?sections=system,jobs'),
    apiGet<ServiceCharacter>(fetch, '/api/admin/service-character'),
  ]);

  return { live, service };
};
