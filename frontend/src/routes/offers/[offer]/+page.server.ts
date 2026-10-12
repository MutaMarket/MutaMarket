import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { OfferThread } from '#lib/types-offers.js';

// Guests are sent to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const offer = await apiGet<OfferThread>(fetch, `/api/offers/${params.offer}`);
  return { offer };
};
