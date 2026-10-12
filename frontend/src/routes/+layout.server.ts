import type { LayoutServerLoad } from './$types';
import { settingsFromCookies } from '#lib/display.js';
import { SHARED_PROPS_DEPENDENCY } from '#lib/invalidation.js';
import { loadSharedProps } from '#lib/server/shared-props.js';

export const load: LayoutServerLoad = async ({ fetch, cookies, locals, depends }) => {
  depends(SHARED_PROPS_DEPENDENCY);
  return {
    locale: locals.locale,
    ...(await loadSharedProps(fetch)),
    displaySettings: settingsFromCookies((name) => cookies.get(name)),
  };
};
