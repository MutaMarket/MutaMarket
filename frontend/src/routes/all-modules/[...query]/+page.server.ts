import type { PageServerLoad } from './$types';
import { loadBrowser } from '#lib/server/browser.ts';

// The all-modules browser includes modules not currently for sale, like
// the legacy AllModulesController.
export const load: PageServerLoad = (event) => loadBrowser(event, event.params.query, true);
