import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';

export interface AdminGearItem {
  id: number;
  name: string;
  description: string | null;
  image_url: string | null;
  link: string;
  active: boolean;
  priority: number;
}

// Guests hit the API 401 -> login; non-admins the 403 error page.
export const load: PageServerLoad = async ({ fetch }) => {
  const gearItems = await apiGet<AdminGearItem[]>(fetch, '/api/admin/gear-items');
  return { gearItems };
};
