// Translation runtime, the counterpart of the legacy vue-i18n plugin:
// the same keys and message syntax (named {params}, "one | other"
// plural forms chosen by count), English as the fallback locale, and a
// locale that switches in place without a reload. In the browser the
// locale is one reactive value per session; on the server every request
// reads its own through the reader hooks.server.ts installs, so locales
// never bleed between requests.
import { browser } from '$app/environment';
import { DEFAULT_LOCALE, MESSAGES, type Locale } from './i18n/messages';

export { DEFAULT_LOCALE, LOCALES, isLocale, type Locale } from './i18n/messages';

/** The legacy cookie: readable by JavaScript, so the switcher can write it. */
export const LOCALE_COOKIE = 'locale';
/** A year, like the legacy SetLocale middleware. */
export const LOCALE_COOKIE_MAX_AGE = 60 * 60 * 24 * 365;

let current: Locale = $state(DEFAULT_LOCALE);
let serverReader: (() => Locale) | null = null;

export function getLocale(): Locale {
  if (browser) {
    return current;
  }
  return serverReader?.() ?? DEFAULT_LOCALE;
}

/** Installed once by the server hooks; returns the current request's locale. */
export function registerServerLocaleReader(reader: () => Locale): void {
  serverReader = reader;
}

/** Seeds the browser locale from the server's decision; no-op on the server. */
export function seedLocale(locale: Locale): void {
  if (browser) {
    current = locale;
  }
}

/** The switcher: swaps messages in place and writes the cookie for the next request. */
export function setLocale(locale: Locale): void {
  if (!browser) {
    return;
  }
  current = locale;
  document.cookie = `${LOCALE_COOKIE}=${locale};path=/;max-age=${LOCALE_COOKIE_MAX_AGE};samesite=lax`;
  document.documentElement.lang = locale;
}

export type Params = Record<string, string | number>;

/** The vue-i18n default plural rule: which "|" form a count selects. */
function pluralIndex(choice: number, forms: number): number {
  const count = Math.abs(choice);
  if (forms === 2) {
    return count === 1 ? 0 : 1;
  }
  return count === 0 ? 0 : Math.min(count, 2);
}

/** The raw message for a key in the current locale, English when the locale lacks it. */
export function message(key: string, locale: Locale = getLocale()): string | undefined {
  return MESSAGES[locale][key] ?? MESSAGES[DEFAULT_LOCALE][key];
}

/** The message with its plural form picked and its {params} filled in. */
export function t(key: string, params: Params = {}): string {
  const raw = message(key);
  if (raw === undefined) {
    return key;
  }
  const choice = params.count ?? params.n;
  const forms = raw.split(' | ');
  const chosen =
    forms.length > 1 && typeof choice === 'number' ? forms[pluralIndex(choice, forms.length)] : raw;
  return chosen.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

/** The message split into literal text and {placeholder} names, for the Trans component. */
export function segments(key: string, params: Params = {}): { text?: string; slot?: string }[] {
  const raw = message(key) ?? key;
  const choice = params.count ?? params.n;
  const forms = raw.split(' | ');
  const chosen =
    forms.length > 1 && typeof choice === 'number' ? forms[pluralIndex(choice, forms.length)] : raw;
  const parts: { text?: string; slot?: string }[] = [];
  let last = 0;
  for (const match of chosen.matchAll(/\{(\w+)\}/g)) {
    if (match.index > last) {
      parts.push({ text: chosen.slice(last, match.index) });
    }
    parts.push({ slot: match[1] });
    last = match.index + match[0].length;
  }
  if (last < chosen.length) {
    parts.push({ text: chosen.slice(last) });
  }
  return parts;
}
