import type { PageServerLoad } from './$types';
import { loadBrowser } from '#lib/server/browser.js';

// The home page: the unfiltered for-sale module browser.
export const load: PageServerLoad = (event) => loadBrowser(event, '', false);
