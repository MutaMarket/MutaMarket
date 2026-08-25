import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { CharacterPageData } from '$lib/types-social';

export const load: PageServerLoad = async ({ fetch, params }) => ({
	page: await apiGet<CharacterPageData>(fetch, `/api/characters/${params.character}`)
});
