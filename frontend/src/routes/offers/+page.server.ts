import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';
import type { OfferListItem } from '#lib/types-offers.ts';

// Guests are sent to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => {
  const offers = await apiGet<OfferListItem[]>(fetch, '/api/offers');
  return { offers };
};
