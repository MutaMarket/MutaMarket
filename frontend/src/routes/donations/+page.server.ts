import type { PageServerLoad } from './$types';
import type { DonationLists } from '$lib/donations';
import { EMPTY_DONATIONS } from '$lib/donations';

// The donations page reads the legacy shared `donations` prop, which
// lives in the sidebar payload here. Like the legacy page, an absent
// prop degrades to empty lists instead of failing the page.
export const load: PageServerLoad = async ({ fetch }) => {
	const donations: DonationLists = await fetch('/api/sidebar')
		.then((response) => (response.ok ? response.json() : null))
		.then((payload) => payload?.donations ?? EMPTY_DONATIONS)
		.catch(() => EMPTY_DONATIONS);

	return { donations };
};
