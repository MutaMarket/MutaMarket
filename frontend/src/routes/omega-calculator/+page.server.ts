import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';

// The legacy OmegaCalculatorController props. Legacy quirk, ported: the
// page declares the sales prop but never uses it in its calculations.
export interface OmegaSales {
  sales: {
    markeedragon: string | null;
    evestore: string | null;
  };
}

export const load: PageServerLoad = async ({ fetch }) => {
  const { sales } = await apiGet<OmegaSales>(fetch, '/api/omega-calculator');
  return { sales };
};
