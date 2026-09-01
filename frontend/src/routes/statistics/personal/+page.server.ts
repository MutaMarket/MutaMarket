import type { PageServerLoad } from './$types';
import type { PersonalStats } from '$lib/statistics';

// The personal statistics tab (legacy /personal/stats); guests see the
// sign-in invitation instead of a redirect.
export const load: PageServerLoad = async ({ fetch }) => ({
	personal: await fetch('/api/personal/stats')
		.then((response) => (response.ok ? (response.json() as Promise<PersonalStats>) : null))
		.catch(() => null),
});
