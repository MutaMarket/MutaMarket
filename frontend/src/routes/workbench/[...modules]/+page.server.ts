import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import type { ModuleDetail } from '#lib/types.js';

// Public like the legacy invitation page.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const modules = await apiGet<ModuleDetail[]>(fetch, `/api/workbench-page/${params.modules}`);
  return { modules, shared: params.modules };
};
