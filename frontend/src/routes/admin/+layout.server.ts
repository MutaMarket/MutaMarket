import type { LayoutServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { LivePayload } from '$lib/admin-live.svelte';

// The console chrome only needs the header slice; each page loads the
// sections it draws. Guests hit the API 401 -> login, non-admins the
// 403 error page, so every admin route is gated here once.
export const load: LayoutServerLoad = async ({ fetch }) => {
  const live = await apiGet<LivePayload>(fetch, '/api/admin/live?sections=header');
  return { live };
};
