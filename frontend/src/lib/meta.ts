// Page metadata, ported from the legacy
// resources/js/Components/Meta/Headers.vue. The tag set, its order, the
// `name`/`property` split and the default card are mirrored from that
// component. Page titles and descriptions come from the meta.* keys of
// the translation catalogue, as they did upstream.

import { attributeFormattedValue } from './attributes';
import { toIskCompact } from './format-number';
import { t } from './i18n.svelte';
import type { EstimatorStatistic, ModuleAttributeView, ModuleDetail } from './types';

/** Prefixed onto every page's own keywords, as in the legacy component. */
const DEFAULT_KEYWORDS =
  'mutamarket, mutaplasmid, modules, contracts, deals, offers, eve, online, gaming, appraise, abyssal';

/** The site-wide card for pages that declare no image. The dimensions are
 * the real pixel size of static/img/mutamarket-og.png. */
const DEFAULT_IMAGE: MetaImage = {
  url: '/img/mutamarket-og.png',
  width: 1280,
  height: 800,
};

/** Every OG endpoint and the static fallback serve PNG. */
const IMAGE_TYPE = 'image/png';

/** The amber brand accent browsers tint the UI with. */
const THEME_COLOR = '#f59f0a';

const SITE_NAME = 'mutamarket.com';

/** Legacy mapped the active i18n locale here; the rewrite is English-only. */
const OG_LOCALE = 'en_US';

/** The 1.91:1 card the type, character and collection OG endpoints render. */
const OG_CARD_WIDTH = 600;
const OG_CARD_HEIGHT = 315;

/** The module card is a portrait list: one 50px row per rolled attribute
 * on top of 72px of chrome (the renderer's 50px header, 2px rule and two
 * 10px paddings). Mirrors the legacy OpenGraphController's own math. */
const MODULE_CARD_WIDTH = 350;
const MODULE_CARD_ROW_HEIGHT = 50;
const MODULE_CARD_CHROME_HEIGHT = 72;

/** Below this R² the estimate is labelled low confidence, as in the
 * legacy ShowModulePage description. */
const LOW_CONFIDENCE_R2 = 0.1;

export interface MetaImage {
  url: string;
  width: number;
  height: number;
}

/** One rendered `<meta>` tag. Legacy emits the OpenGraph and Twitter
 * content tags with `property` and the rest with `name`; the split is
 * inconsistent upstream (og:site_name, og:locale and og:type use `name`)
 * and is reproduced verbatim so the emitted markup matches. */
export interface MetaTag {
  attr: 'name' | 'property';
  key: string;
  content: string;
}

export interface MetaInput {
  /** Absolute page origin, e.g. https://mutamarket.com. */
  origin: string;
  /** The current path, e.g. /modules/foo-123. */
  path: string;
  title: string;
  description: string;
  image?: MetaImage;
  keywords?: string | string[];
}

function trimSlashes(value: string): string {
  return value.replace(/^\/+|\/+$/g, '');
}

/** Scrapers reject relative og:image and og:url values, so everything we
 * emit is absolute. Legacy hardcoded https://mutamarket.com for the page
 * URL and left the image relative; the rewrite derives both from the
 * request origin so previews also resolve on staging and in dev. */
export function absoluteUrl(origin: string, path: string): string {
  if (/^https?:\/\//i.test(path)) {
    return path;
  }
  return `${origin.replace(/\/+$/, '')}/${trimSlashes(path)}`;
}

export function combineKeywords(keywords?: string | string[]): string {
  const own = (Array.isArray(keywords) ? keywords.join(',') : (keywords ?? '')).trim();
  return own ? `${DEFAULT_KEYWORDS}, ${own}` : DEFAULT_KEYWORDS;
}

export function buildMetaTags(input: MetaInput): MetaTag[] {
  const image = input.image ?? DEFAULT_IMAGE;
  const imageUrl = absoluteUrl(input.origin, image.url);
  const pageUrl = absoluteUrl(input.origin, input.path);

  const tags: MetaTag[] = [
    { attr: 'name', key: 'description', content: input.description },
    { attr: 'name', key: 'keywords', content: combineKeywords(input.keywords) },
    { attr: 'property', key: 'og:image', content: imageUrl },
    { attr: 'property', key: 'og:image:type', content: IMAGE_TYPE },
    { attr: 'property', key: 'og:description', content: input.description },
    { attr: 'property', key: 'og:title', content: input.title },
    { attr: 'property', key: 'og:url', content: pageUrl },
    { attr: 'property', key: 'twitter:image', content: imageUrl },
    { attr: 'property', key: 'twitter:description', content: input.description },
    { attr: 'property', key: 'twitter:title', content: input.title },
    { attr: 'property', key: 'twitter:url', content: pageUrl },
    { attr: 'name', key: 'twitter:card', content: 'summary_large_image' },
  ];

  // Legacy declares the dimensions only for a page-supplied image, not
  // for the default card.
  if (input.image) {
    tags.push(
      { attr: 'property', key: 'og:image:width', content: String(input.image.width) },
      { attr: 'property', key: 'og:image:height', content: String(input.image.height) },
    );
  }

  tags.push(
    { attr: 'name', key: 'og:site_name', content: SITE_NAME },
    { attr: 'name', key: 'theme-color', content: THEME_COLOR },
    { attr: 'name', key: 'twitter:site', content: SITE_NAME },
    { attr: 'name', key: 'og:locale', content: OG_LOCALE },
    { attr: 'name', key: 'og:type', content: 'website' },
  );

  return tags;
}

// The OG card endpoints live in the Rust API (src/server/social.rs).
// Legacy appended ".png" to the module URL for scrapers that want an
// image extension; those handlers parse the segment as an integer, so the
// rewrite points at the bare path instead.

export function typeOgImage(typeId: number): MetaImage {
  return { url: `/og/type/${typeId}`, width: OG_CARD_WIDTH, height: OG_CARD_HEIGHT };
}

export function characterOgImage(characterId: number): MetaImage {
  return { url: `/og/character/${characterId}`, width: OG_CARD_WIDTH, height: OG_CARD_HEIGHT };
}

export function collectionOgImage(collectionId: number): MetaImage {
  return { url: `/og/collection/${collectionId}`, width: OG_CARD_WIDTH, height: OG_CARD_HEIGHT };
}

export function moduleOgImage(moduleId: number, attributes: ModuleAttributeView[]): MetaImage {
  const rows = attributes.filter((attribute) => !attribute.is_virtual).length;
  return {
    url: `/og/module/${moduleId}`,
    width: MODULE_CARD_WIDTH,
    height: MODULE_CARD_CHROME_HEIGHT + rows * MODULE_CARD_ROW_HEIGHT,
  };
}

/** The module card's description: every mutated attribute on its own
 * line, then the estimate. Ported from the legacy ShowModulePage. */
export function moduleMetaDescription(
  module: ModuleDetail,
  statistic: EstimatorStatistic | null,
): string {
  const lines = module.mutated_attributes.map(
    (attribute) => `${attribute.display_name}: ${attributeFormattedValue(attribute)}`,
  );

  let value = toIskCompact(module.estimated_value);
  if (typeof statistic?.r2 === 'number' && statistic.r2 < LOW_CONFIDENCE_R2) {
    value += ` ${t('meta.modulesShow.lowConfidence')}`;
  }

  lines.push(t('meta.modulesShow.estimatedValue', { value }));

  return lines.join('\n');
}

/** The brand every document title ends in, as the legacy translations
 * did ("All modules | MutaMarket"). */
export const SITE_TITLE = 'MutaMarket';

/** The document title of a page: its own title, a bar, the brand. */
export function documentTitle(title: string): string {
  return `${title} | ${SITE_TITLE}`;
}

/** The module page title: "{creator}'s {type}". */
export function moduleMetaTitle(module: ModuleDetail): string {
  return t('meta.modulesShow.title', {
    creator: module.creator?.name ?? t('common.labels.unknown'),
    type: module.type.name,
  });
}
