// Google AdSense with Auto ads: the page carries only the loader script
// and Google places the ads itself, so there are no slot components.
// The legacy `Advertisement.vue` gate is kept: guests and free accounts
// see ads, premium accounts do not. Empty client id means no AdSense
// at all (development, forks without an account).
import { env } from '$env/dynamic/public';
import type { NavState } from './types';

export const ADSENSE_CLIENT_ID = env.PUBLIC_ADSENSE_CLIENT_ID ?? '';

/** The AdSense loader, which also enables Auto ads for the page. */
export function adsenseScriptUrl(clientId: string): string {
  return `https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=${encodeURIComponent(clientId)}`;
}

/** Whether this visitor gets the AdSense loader at all. */
export function showsAds(nav: NavState | null, clientId: string): boolean {
  if (clientId === '') {
    return false;
  }
  return !nav?.user.has_premium;
}
