import type { LayoutServerLoad } from './$types';
import { settingsFromCookies } from '$lib/display';
import { loadSharedProps } from '$lib/server/shared-props';

export const load: LayoutServerLoad = async ({ fetch, cookies, locals }) => {
  return {
    locale: locals.locale,
    ...(await loadSharedProps(fetch)),
    displaySettings: settingsFromCookies((name) => cookies.get(name)),
  };
};
