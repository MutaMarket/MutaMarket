import type { LayoutServerLoad } from './$types';
import { settingsFromCookies } from '#lib/display.ts';
import { SHARED_PROPS_DEPENDENCY } from '#lib/invalidation.ts';
import { loadSharedProps } from '#lib/server/shared-props.ts';

export const load: LayoutServerLoad = async ({ fetch, cookies, locals, depends }) => {
  depends(SHARED_PROPS_DEPENDENCY);
  return {
    locale: locals.locale,
    ...(await loadSharedProps(fetch)),
    displaySettings: settingsFromCookies((name) => cookies.get(name)),
  };
};
