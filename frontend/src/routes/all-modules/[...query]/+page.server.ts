import type { PageServerLoad } from './$types';
import { loadBrowser } from '$lib/server/browser';

// The all-modules browser includes modules not currently for sale, like
// the legacy AllModulesController.
export const load: PageServerLoad = ({ fetch, params }) => loadBrowser(fetch, params.query, true);
