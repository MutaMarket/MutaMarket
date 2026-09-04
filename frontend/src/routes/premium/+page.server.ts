import type { PageServerLoad } from './$types';
import type { ModuleDetail, PremiumGiftable } from '$lib/types';
import { premiumFromSidebar } from '$lib/premium';
import { apiGet } from '$lib/server/api';

// The premium sales page, the legacy PremiumController::index props
// plus the shared premium config of the AppData props (/api/sidebar).
export const load: PageServerLoad = async ({ fetch }) => {
  const [{ sample_modules, giftable }, sidebar] = await Promise.all([
    apiGet<{ sample_modules: ModuleDetail[]; giftable: PremiumGiftable[] | null }>(
      fetch,
      '/api/premium/page',
    ),
    fetch('/api/sidebar')
      .then((response) => (response.ok ? response.json() : null))
      .catch(() => null),
  ]);

  return {
    sampleModules: sample_modules,
    giftable: giftable ?? [],
    premium: premiumFromSidebar(sidebar),
  };
};
