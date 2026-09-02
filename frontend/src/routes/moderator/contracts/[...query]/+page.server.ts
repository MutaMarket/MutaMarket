import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { ReviewPageData } from '$lib/contract-review';

// The moderator contract review, the legacy
// ModeratorContractController::index — public like the legacy route, with
// the optional filter query as the rest segment.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const path =
    params.query === '' ? '/api/moderator/contracts' : `/api/moderator/contracts/${params.query}`;
  const review = await apiGet<ReviewPageData>(fetch, path);
  return { review, query: params.query };
};
