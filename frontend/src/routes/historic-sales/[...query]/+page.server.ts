import type { PageServerLoad } from './$types';
import { loadBrowser } from '$lib/server/browser';

// The premium historic-sales browser, the legacy HistoricSaleController.
export const load: PageServerLoad = ({ fetch, params }) =>
  loadBrowser(fetch, params.query, true, true);
