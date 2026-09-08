import { describe, expect, it } from 'vitest';
import {
  IN_FEED_ROW_SPAN,
  adRouteKey,
  IN_FEED_POSITIONS,
  adsenseScriptUrl,
  showsAds,
  withInFeedAds,
} from './adsense';
import type { DisplayEntry, NavState } from './types';

function nav(is_patreon_member: boolean, has_premium = false): NavState {
  return { user: { is_patreon_member, has_premium } } as NavState;
}

describe('adsense', () => {
  it('serves ads to everyone but Patreon backers', () => {
    // The legacy useAdvertisement gate: patreon.is_premium, not the ISK
    // premium.
    expect(showsAds(null, 'ca-pub-1')).toBe(true);
    expect(showsAds(nav(false), 'ca-pub-1')).toBe(true);
    expect(showsAds(nav(false, true), 'ca-pub-1')).toBe(true);
    expect(showsAds(nav(true), 'ca-pub-1')).toBe(false);
  });

  it('stays off entirely without a client id', () => {
    expect(showsAds(null, '')).toBe(false);
    expect(showsAds(nav(false), '')).toBe(false);
  });

  it('points the loader at the client id', () => {
    expect(adsenseScriptUrl('ca-pub-1')).toBe(
      'https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=ca-pub-1',
    );
  });

  describe('adRouteKey', () => {
    it('changes with the page, the module type and the page number only', () => {
      expect(adRouteKey('/modules')).toBe('/modules');
      expect(adRouteKey('/modules/goldbar/sort/price')).toBe('/modules');
      expect(adRouteKey('/modules/page/1')).toBe('/modules');
      expect(adRouteKey('/modules/type/47408/goldbar')).toBe('/modules/type/47408');
      expect(adRouteKey('/modules/goldbar/page/2')).toBe('/modules/page/2');
      expect(adRouteKey('/all-modules/type/47408/page/3')).toBe('/all-modules/type/47408/page/3');
      expect(adRouteKey('/collections/shiny-rolls-7/type/47408/search/x')).toBe(
        '/collections/shiny-rolls-7/type/47408',
      );
    });

    it('keeps a page without a query path as it is', () => {
      expect(adRouteKey('/modules/medium-abyssal-shield-extender-1055564662093')).toBe(
        '/modules/medium-abyssal-shield-extender-1055564662093',
      );
      expect(adRouteKey('/')).toBe('/');
    });
  });

  describe('withInFeedAds', () => {
    const entries = (count: number): DisplayEntry[] =>
      Array.from({ length: count }, (_, i) => ({ module: { id: i + 1 } }) as DisplayEntry);
    const shape = (count: number, slot = '42') =>
      withInFeedAds(entries(count), slot).map((item) =>
        item.kind === 'ad' ? `ad@${item.position}` : item.entry.module.id,
      );

    it('puts an ad card after the configured module counts', () => {
      expect(IN_FEED_POSITIONS).toEqual([4, 20]);
      // The span of the common four-attribute card.
      expect(IN_FEED_ROW_SPAN).toBe(6);
      const items = shape(40);
      expect(items.slice(0, 6)).toEqual([1, 2, 3, 4, 'ad@4', 5]);
      expect(items.slice(20, 23)).toEqual([20, 'ad@20', 21]);
      expect(items).toHaveLength(42);
    });

    it('never ends a short page with an ad', () => {
      expect(shape(4)).toEqual([1, 2, 3, 4]);
      expect(shape(5)).toEqual([1, 2, 3, 4, 'ad@4', 5]);
      expect(shape(0)).toEqual([]);
    });

    it('is a plain module list while the unit has no id', () => {
      expect(shape(40, '')).toHaveLength(40);
    });
  });
});
