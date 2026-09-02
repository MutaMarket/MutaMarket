import { describe, expect, it } from 'vitest';
import {
  absoluteUrl,
  buildMetaTags,
  characterOgImage,
  collectionOgImage,
  combineKeywords,
  moduleMetaDescription,
  moduleMetaTitle,
  moduleOgImage,
  typeOgImage,
  type MetaTag,
} from './meta';
import type { EstimatorStatistic, ModuleAttributeView, ModuleDetail } from './types';

const ORIGIN = 'https://mutamarket.com';

const DEFAULT_KEYWORDS =
  'mutamarket, mutaplasmid, modules, contracts, deals, offers, eve, online, gaming, appraise, abyssal';

/** The tags as "attr:key=content", in emission order. */
function rendered(tags: MetaTag[]): string[] {
  return tags.map((tag) => `${tag.attr}:${tag.key}=${tag.content}`);
}

function contentOf(tags: MetaTag[], key: string): string | undefined {
  return tags.find((tag) => tag.key === key)?.content;
}

describe('combineKeywords', () => {
  it('returns the defaults alone when the page supplies none', () => {
    expect(combineKeywords()).toBe(DEFAULT_KEYWORDS);
  });

  it('appends a keyword string after the defaults', () => {
    expect(combineKeywords('contracts, public, search, find')).toBe(
      `${DEFAULT_KEYWORDS}, contracts, public, search, find`,
    );
  });

  it('joins an array with commas, as the legacy Array.toString did', () => {
    expect(combineKeywords(['Nicolas Kion', 'character', 'modules'])).toBe(
      `${DEFAULT_KEYWORDS}, Nicolas Kion,character,modules`,
    );
  });

  it('drops the separator for an empty array or blank string', () => {
    expect(combineKeywords([])).toBe(DEFAULT_KEYWORDS);
    expect(combineKeywords('   ')).toBe(DEFAULT_KEYWORDS);
  });
});

describe('absoluteUrl', () => {
  it('prefixes the origin onto a root-relative path', () => {
    expect(absoluteUrl(ORIGIN, '/og/type/47800')).toBe('https://mutamarket.com/og/type/47800');
  });

  it('collapses the slashes between origin and path', () => {
    expect(absoluteUrl('https://mutamarket.com/', '///modules/foo')).toBe(
      'https://mutamarket.com/modules/foo',
    );
  });

  it('keeps the trailing slash form for the site root', () => {
    expect(absoluteUrl(ORIGIN, '/')).toBe('https://mutamarket.com/');
  });

  it('leaves an already absolute URL untouched', () => {
    expect(absoluteUrl(ORIGIN, 'https://images.evetech.net/types/1/icon')).toBe(
      'https://images.evetech.net/types/1/icon',
    );
  });

  it('resolves against a non-production origin', () => {
    expect(absoluteUrl('http://localhost:5100', '/og/module/12')).toBe(
      'http://localhost:5100/og/module/12',
    );
  });
});

describe('buildMetaTags without a page image', () => {
  const tags = buildMetaTags({
    origin: ORIGIN,
    path: '/donations',
    title: 'Donations',
    description: 'Support MutaMarket and help us keep the site running!',
    keywords: 'donations, support, isk',
  });

  it('emits the legacy tag set in order, with the default card', () => {
    expect(rendered(tags)).toEqual([
      'name:description=Support MutaMarket and help us keep the site running!',
      `name:keywords=${DEFAULT_KEYWORDS}, donations, support, isk`,
      'property:og:image=https://mutamarket.com/img/mutamarket-og.png',
      'property:og:image:type=image/png',
      'property:og:description=Support MutaMarket and help us keep the site running!',
      'property:og:title=Donations',
      'property:og:url=https://mutamarket.com/donations',
      'property:twitter:image=https://mutamarket.com/img/mutamarket-og.png',
      'property:twitter:description=Support MutaMarket and help us keep the site running!',
      'property:twitter:title=Donations',
      'property:twitter:url=https://mutamarket.com/donations',
      'name:twitter:card=summary_large_image',
      'name:og:site_name=mutamarket.com',
      'name:theme-color=#f59f0a',
      'name:twitter:site=mutamarket.com',
      'name:og:locale=en_US',
      'name:og:type=website',
    ]);
  });

  it('declares no image dimensions for the default card', () => {
    expect(tags.some((tag) => tag.key === 'og:image:width')).toBe(false);
    expect(tags.some((tag) => tag.key === 'og:image:height')).toBe(false);
  });
});

describe('buildMetaTags with a page image', () => {
  const tags = buildMetaTags({
    origin: ORIGIN,
    path: '/characters/nicolas-kion-42',
    title: 'Nicolas Kion',
    description: "Browse Nicolas Kion's abyssal modules on MutaMarket.",
    image: characterOgImage(42),
    keywords: ['Nicolas Kion', 'character', 'modules'],
  });

  it('emits the legacy tag set in order, dimensions included', () => {
    expect(rendered(tags)).toEqual([
      "name:description=Browse Nicolas Kion's abyssal modules on MutaMarket.",
      `name:keywords=${DEFAULT_KEYWORDS}, Nicolas Kion,character,modules`,
      'property:og:image=https://mutamarket.com/og/character/42',
      'property:og:image:type=image/png',
      "property:og:description=Browse Nicolas Kion's abyssal modules on MutaMarket.",
      'property:og:title=Nicolas Kion',
      'property:og:url=https://mutamarket.com/characters/nicolas-kion-42',
      'property:twitter:image=https://mutamarket.com/og/character/42',
      "property:twitter:description=Browse Nicolas Kion's abyssal modules on MutaMarket.",
      'property:twitter:title=Nicolas Kion',
      'property:twitter:url=https://mutamarket.com/characters/nicolas-kion-42',
      'name:twitter:card=summary_large_image',
      'property:og:image:width=600',
      'property:og:image:height=315',
      'name:og:site_name=mutamarket.com',
      'name:theme-color=#f59f0a',
      'name:twitter:site=mutamarket.com',
      'name:og:locale=en_US',
      'name:og:type=website',
    ]);
  });

  it('makes the OG image absolute against a dev origin too', () => {
    const local = buildMetaTags({
      origin: 'http://localhost:5100',
      path: '/characters/nicolas-kion-42',
      title: 'Nicolas Kion',
      description: 'x',
      image: characterOgImage(42),
    });
    expect(contentOf(local, 'og:image')).toBe('http://localhost:5100/og/character/42');
    expect(contentOf(local, 'twitter:image')).toBe('http://localhost:5100/og/character/42');
  });

  it('honours an explicit canonical path override', () => {
    const overridden = buildMetaTags({
      origin: ORIGIN,
      path: '/modules',
      title: 'All modules',
      description: 'x',
    });
    expect(contentOf(overridden, 'og:url')).toBe('https://mutamarket.com/modules');
    expect(contentOf(overridden, 'twitter:url')).toBe('https://mutamarket.com/modules');
  });
});

describe('OG image endpoints', () => {
  it('points the type, character and collection cards at 600x315', () => {
    expect(typeOgImage(47800)).toEqual({ url: '/og/type/47800', width: 600, height: 315 });
    expect(characterOgImage(90)).toEqual({ url: '/og/character/90', width: 600, height: 315 });
    expect(collectionOgImage(7)).toEqual({ url: '/og/collection/7', width: 600, height: 315 });
  });

  it('sizes the module card by its rolled attribute rows', () => {
    const attributes = [
      { is_virtual: false },
      { is_virtual: false },
      { is_virtual: false },
    ] as ModuleAttributeView[];
    expect(moduleOgImage(12, attributes)).toEqual({
      url: '/og/module/12',
      width: 350,
      height: 72 + 3 * 50,
    });
  });

  it('excludes virtual attributes from the row count', () => {
    const attributes = [
      { is_virtual: false },
      { is_virtual: true },
      { is_virtual: false },
    ] as ModuleAttributeView[];
    expect(moduleOgImage(12, attributes).height).toBe(72 + 2 * 50);
  });

  it('is just the chrome when nothing rolled', () => {
    expect(moduleOgImage(12, []).height).toBe(72);
  });
});

function attribute(display_name: string, value: number): ModuleAttributeView {
  return { display_name, value, unit: null } as ModuleAttributeView;
}

function moduleWith(estimated_value: number | null): ModuleDetail {
  return {
    id: 12,
    type: { name: 'Abyssal Heavy Assault Missile Launcher' },
    creator: { name: 'Nicolas Kion' },
    estimated_value,
    mutated_attributes: [attribute('Rate of fire', 8.5), attribute('CPU usage', 30)],
  } as unknown as ModuleDetail;
}

describe('moduleMetaTitle', () => {
  it("reads {creator}'s {type}", () => {
    expect(moduleMetaTitle(moduleWith(1))).toBe(
      "Nicolas Kion's Abyssal Heavy Assault Missile Launcher",
    );
  });

  it('falls back to Unknown for a module with no creator', () => {
    const orphan = { ...moduleWith(1), creator: null } as ModuleDetail;
    expect(moduleMetaTitle(orphan)).toBe("Unknown's Abyssal Heavy Assault Missile Launcher");
  });
});

describe('moduleMetaDescription', () => {
  it('lists each mutated attribute then the estimate', () => {
    expect(moduleMetaDescription(moduleWith(142_000_000), null)).toBe(
      'Rate of fire: 8.5\nCPU usage: 30\nEst. value: 142 million ISK',
    );
  });

  it('reads N/A when the module has no estimate', () => {
    expect(moduleMetaDescription(moduleWith(null), null)).toBe(
      'Rate of fire: 8.5\nCPU usage: 30\nEst. value: N/A',
    );
  });

  it('flags a low-confidence estimate below the R2 threshold', () => {
    const statistic = { r2: 0.05 } as EstimatorStatistic;
    expect(moduleMetaDescription(moduleWith(142_000_000), statistic)).toBe(
      'Rate of fire: 8.5\nCPU usage: 30\nEst. value: 142 million ISK (Low confidence)',
    );
  });

  it('leaves a confident estimate unflagged', () => {
    const statistic = { r2: 0.9 } as EstimatorStatistic;
    expect(moduleMetaDescription(moduleWith(142_000_000), statistic)).toBe(
      'Rate of fire: 8.5\nCPU usage: 30\nEst. value: 142 million ISK',
    );
  });

  it('leaves the estimate unflagged when R2 is unknown', () => {
    const statistic = { r2: null } as EstimatorStatistic;
    expect(moduleMetaDescription(moduleWith(142_000_000), statistic)).toBe(
      'Rate of fire: 8.5\nCPU usage: 30\nEst. value: 142 million ISK',
    );
  });
});
