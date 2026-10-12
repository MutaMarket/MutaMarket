import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { LocationsData } from '#lib/location-tree.js';

// The asset-locations tree (legacy ShowLocationsPage); guests bounce to
// the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => ({
  tree: await apiGet<LocationsData>(fetch, '/api/locations'),
});
