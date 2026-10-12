import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';
import type { SearchAlert } from '#lib/search-alerts.ts';
import type { SettingsData } from '#lib/settings.ts';

// The account settings page (legacy ShowSettingsPage); guests bounce
// to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => {
  const [settings, searchAlerts] = await Promise.all([
    apiGet<SettingsData>(fetch, '/api/settings'),
    apiGet<SearchAlert[]>(fetch, '/api/search-alerts'),
  ]);
  return { settings, searchAlerts };
};
