import type { LayoutServerLoad } from './$types';
import type { NavState } from '$lib/types';
import { settingsFromCookies } from '$lib/display';

export const load: LayoutServerLoad = async ({ fetch, cookies }) => {
  // Guests get a JSON null; an unreachable API renders as guest rather
  // than failing every page.
  const nav: NavState | null = await fetch('/api/nav-state')
    .then((response) => (response.ok ? response.json() : null))
    .catch(() => null);

  return {
    nav,
    displaySettings: settingsFromCookies((name) => cookies.get(name)),
  };
};
