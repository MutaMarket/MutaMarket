import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { CharacterCardData } from '$lib/types-social';

export const load: PageServerLoad = async ({ fetch }) => ({
  characters: await apiGet<CharacterCardData[]>(fetch, '/api/characters'),
});
