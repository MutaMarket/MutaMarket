import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// The pre-tabs URLs carried the leaderboard query directly under
// /statistics; send them to the characters tab.
export const load: PageServerLoad = ({ params, url }) => {
	const query = params.query ?? '';
	redirect(301, `/statistics/characters${query === '' ? '' : `/${query}`}${url.search}`);
};
