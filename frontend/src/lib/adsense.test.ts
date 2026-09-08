import { describe, expect, it } from 'vitest';
import { adsenseScriptUrl, showsAds } from './adsense';
import type { NavState } from './types';

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
});
