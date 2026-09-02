import type { LayoutServerLoad } from './$types';
import { settingsFromCookies } from '$lib/display';
import { loadSharedProps } from '$lib/server/shared-props';

export const load: LayoutServerLoad = async ({ fetch, cookies }) => {
  return {
    ...(await loadSharedProps(fetch)),
    displaySettings: settingsFromCookies((name) => cookies.get(name)),
  };
};
