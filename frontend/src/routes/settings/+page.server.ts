import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { SettingsData } from '$lib/settings';

// The account settings page (legacy ShowSettingsPage); guests bounce
// to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => ({
	settings: await apiGet<SettingsData>(fetch, '/api/settings'),
});
