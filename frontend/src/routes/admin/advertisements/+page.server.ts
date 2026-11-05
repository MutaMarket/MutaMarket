import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';

export interface AdminAdvertisement {
  id: number;
  name: string;
  description: string | null;
  image_url: string | null;
  link: string | null;
  size: string;
  active: boolean;
  priority: number;
  starts_at: string | null;
  expires_at: string | null;
  status: 'live' | 'scheduled' | 'expired' | 'inactive';
}

// Guests hit the API 401 -> login; non-admins the 403 error page.
export const load: PageServerLoad = async ({ fetch }) => {
  const [advertisements, referral] = await Promise.all([
    apiGet<AdminAdvertisement[]>(fetch, '/api/admin/advertisements'),
    apiGet<{ link: string | null }>(fetch, '/api/admin/referral-link'),
  ]);
  return { advertisements, referralLink: referral.link };
};
