import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { AdminRafflesData } from '#lib/raffles.js';

// Guests hit the API 401 -> login; non-admins the 403 error page.
export const load: PageServerLoad = async ({ fetch, url }) => {
  const search = url.searchParams.get('type_search') ?? '';
  const query = search ? `?type_search=${encodeURIComponent(search)}` : '';
  const raffles = await apiGet<AdminRafflesData>(fetch, `/api/admin/raffles${query}`);
  return { raffles };
};
