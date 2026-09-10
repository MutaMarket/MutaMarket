import type { PageServerLoad } from './$types';
import { loadBrowser } from '$lib/server/browser';

// The premium historic-sales browser, the legacy HistoricSaleController.
export const load: PageServerLoad = (event) => loadBrowser(event, event.params.query, true, true);
