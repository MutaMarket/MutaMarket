import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { ModuleDetail } from '$lib/types';

// Public like the legacy invitation page.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const modules = await apiGet<ModuleDetail[]>(fetch, `/api/workbench-page/${params.modules}`);
  return { modules, shared: params.modules };
};
