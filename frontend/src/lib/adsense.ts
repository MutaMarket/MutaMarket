// Google AdSense. The page carries the loader script (which also runs
// the Auto ads formats enabled in the AdSense console) and manual ad
// units, the legacy Advertisement.vue slots, in the spots below. Manual
// units matter on this SPA: Auto ads only place on full page loads,
// while a manual unit is re-requested on every client-side navigation.
// The legacy `useAdvertisement` gate is kept: guests and free accounts
// see ads, premium accounts do not. Empty client id means no AdSense
// at all (development, forks without an account).
import { env } from '$env/dynamic/public';
import { parseQueryUi, splitQueryPath } from './query';
import type { DisplayEntry, NavState } from './types';

export const ADSENSE_CLIENT_ID = env.PUBLIC_ADSENSE_CLIENT_ID ?? '';

/** Ad unit ids of the MutaMarket AdSense account. An empty id keeps
 * that spot dormant until the unit exists in the AdSense console. */
export const AD_SLOTS = {
  /** The inline banner of every page (legacy InlineAd.vue): 970x90 from
   * lg until the rails take over at 3xl, 728x90 from md, 300x100 below. */
  bannerWide: '1526043502',
  bannerMedium: '1300906129',
  bannerMobile: '7674742784',
  /** 300x600 sticky rails beside the page at 4xl+ (legacy LeftColumn/RightColumn.vue). */
  railWideLeft: '8936091240',
  railWideRight: '4343778538',
  /** 160x600 sticky rails beside the page at 3xl (legacy LeftColumn/RightColumn.vue). */
  railNarrowLeft: '8800122194',
  railNarrowRight: '8223313794',
  /** The fluid in-feed unit inside the module grid. */
  inFeed: '4816129786',
} satisfies Record<string, string>;

/** The AdSense layout key of the in-feed unit, the template chosen in
 * the AdSense console. */
export const IN_FEED_LAYOUT_KEY = '+2a+rx+1+2-3';

/** Grid positions (number of module cards before the ad) of the in-feed
 * units: one right around the fold on desktop (a four-column grid), one
 * halfway down a 40-module page. */
export const IN_FEED_POSITIONS = [4, 20] as const;

/** What AdSense reported for a unit: `data-ad-status` on the `<ins>`,
 * `pending` until the request answers. */
export type AdStatus = 'pending' | 'filled' | 'unfilled';

export type GridItem = { kind: 'module'; entry: DisplayEntry } | { kind: 'ad'; position: number };

/** The masonry rows the in-feed card spans: those of the common
 * four-attribute module card (about 320px), which holds the in-feed
 * template at a card's width. */
export const IN_FEED_ROW_SPAN = 6;

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

/** What counts as a new page for the ad units: the page itself, the
 * module type and the page number of a list. Changing any other filter
 * keeps the units in place. (The legacy re-keyed on the full Inertia
 * URL, so every filter change re-requested the ads.) */
export function adRouteKey(pathname: string): string {
  const { base, query } = splitQueryPath(pathname);
  const search = parseQueryUi(query);
  const parts = [base];
  if (search.typeSlug !== null) {
    parts.push(`type/${search.typeSlug}`);
  }
  if (search.page > 1) {
    parts.push(`page/${search.page}`);
  }
  return parts.join('/');
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

/** `showsAds` for the configured account, or every unit's development
 * placeholder. */
export function adsVisible(nav: NavState | null | undefined, dev: boolean): boolean {
  return dev || showsAds(nav ?? null, ADSENSE_CLIENT_ID);
}
