// Google AdSense. The page carries the loader script (which also runs
// the Auto ads formats enabled in the AdSense console) and manual ad
// units, the legacy Advertisement.vue slots, in the spots below. Manual
// units matter on this SPA: Auto ads only place on full page loads,
// while a manual unit is re-requested on every client-side navigation.
// The legacy `useAdvertisement` gate is kept: guests and free accounts
// see ads, premium accounts do not. Empty client id means no AdSense
// at all (development, forks without an account).
import { env } from '$env/dynamic/public';
import type { DisplayEntry, NavState } from './types';

export const ADSENSE_CLIENT_ID = env.PUBLIC_ADSENSE_CLIENT_ID ?? '';

/** Ad unit ids of the MutaMarket AdSense account. An empty id keeps
 * that spot dormant until the unit exists in the AdSense console. */
export const AD_SLOTS = {
  /** Responsive unit in the sidebar (legacy Sidebar.vue). */
  sidebar: '5480032744',
  /** 300x600 sticky rails beside the page at 4xl+ (legacy LeftColumn/RightColumn.vue). */
  railWideLeft: '8936091240',
  railWideRight: '4343778538',
  /** 160x600 sticky rails beside the page at 3xl (legacy LeftColumn/RightColumn.vue). */
  railNarrowLeft: '8800122194',
  railNarrowRight: '8223313794',
  /** New: a card-sized responsive unit inside the module grid. */
  inFeed: '',
  /** New: a responsive banner under the header below xl, where the
   * sidebar and rails do not exist. */
  contentTop: '',
} as const;

/** Grid positions (number of module cards before the ad) of the in-feed
 * units: one right around the fold on desktop (a four-column grid), one
 * halfway down a 40-module page. */
export const IN_FEED_POSITIONS = [4, 20] as const;

export type GridItem = { kind: 'module'; entry: DisplayEntry } | { kind: 'ad'; position: number };

/** Interleaves the in-feed ad cards with the module cards. An ad only
 * lands when the page has at least that many modules; a short page
 * never ends in an ad. */
export function withInFeedAds(entries: DisplayEntry[], slot: string): GridItem[] {
  const items: GridItem[] = entries.map((entry) => ({ kind: 'module', entry }));
  if (slot === '') {
    return items;
  }
  const positions = IN_FEED_POSITIONS.filter((position) => position < entries.length);
  for (const [inserted, position] of positions.entries()) {
    items.splice(position + inserted, 0, { kind: 'ad', position });
  }
  return items;
}

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
