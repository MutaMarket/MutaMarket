import type { PageServerLoad } from './$types';
import type { ModuleDetail, PremiumAccountPlan, PremiumGiftable } from '#lib/types.ts';
import { premiumFromSidebar } from '#lib/premium.ts';
import { apiGet } from '#lib/server/api.ts';

// The premium sales page, the legacy PremiumController::index props
// plus the shared premium config of the AppData props (/api/sidebar),
// the gifting balances and the account's plan with every character's time.
export const load: PageServerLoad = async ({ fetch }) => {
  const [{ sample_modules, giftable, plan }, sidebar] = await Promise.all([
    apiGet<{
      sample_modules: ModuleDetail[];
      giftable: PremiumGiftable[] | null;
      plan: PremiumAccountPlan | null;
    }>(fetch, '/api/premium/page'),
    fetch('/api/sidebar')
      .then((response) => (response.ok ? response.json() : null))
      .catch(() => null),
  ]);

  return {
    sampleModules: sample_modules,
    giftable: giftable ?? [],
    plan,
    premium: premiumFromSidebar(sidebar),
  };
};
