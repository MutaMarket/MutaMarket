import type { PageServerLoad } from './$types';
import type { DonationLists } from '$lib/donations';
import { EMPTY_DONATIONS } from '$lib/donations';
import { premiumFromSidebar } from '$lib/premium';

// The donations page reads the legacy shared `donations` and premium
// AppData props, which live in the sidebar payload here. Like the
// legacy page, an absent payload degrades to empty lists and the
// premium defaults instead of failing the page.
export const load: PageServerLoad = async ({ fetch }) => {
  const payload = await fetch('/api/sidebar')
    .then((response) => (response.ok ? response.json() : null))
    .catch(() => null);

  const donations: DonationLists = payload?.donations ?? EMPTY_DONATIONS;
  return { donations, premium: premiumFromSidebar(payload) };
};
