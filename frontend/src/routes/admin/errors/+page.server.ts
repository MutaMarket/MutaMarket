import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { failuresQuery, type StatusClass } from '$lib/admin-request-failures';
import type { RequestFailuresPayload } from '$lib/admin-types';

const CLASSES: StatusClass[] = ['client', 'server'];

// The filters arrive in the URL so the activity page's error counts can
// link straight at one route; an unknown class is dropped rather than
// handed to the API, which would answer the whole page with a 422.
export const load: PageServerLoad = async ({ fetch, url }) => {
  const route = url.searchParams.get('route');
  const asked = url.searchParams.get('class');
  const statusClass = CLASSES.find((known) => known === asked) ?? null;

  return {
    route,
    statusClass,
    failures: await apiGet<RequestFailuresPayload>(
      fetch,
      failuresQuery({ route, class: statusClass }),
    ),
  };
};
