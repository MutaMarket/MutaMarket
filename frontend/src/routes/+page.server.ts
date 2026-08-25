import type { PageServerLoad } from './$types';
import { loadBrowser } from '$lib/server/browser';

// The home page: the unfiltered for-sale module browser.
export const load: PageServerLoad = ({ fetch }) => loadBrowser(fetch, '', false);
