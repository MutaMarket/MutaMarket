// The request's locale for server rendering, the legacy SetLocale
// middleware: the locale cookie, else the Accept-Language preference
// among the supported locales, else English. AsyncLocalStorage carries
// it through the render so t() works anywhere without a component
// context.
import { AsyncLocalStorage } from 'node:async_hooks';
import { DEFAULT_LOCALE, type Locale, isLocale } from '$lib/i18n/messages';

export const requestLocale = new AsyncLocalStorage<Locale>();

/** The best supported locale from an Accept-Language header, by weight. */
export function preferredLocale(acceptLanguage: string | null): Locale | null {
  if (!acceptLanguage) {
    return null;
  }
  const ranked = acceptLanguage
    .split(',')
    .map((entry, index) => {
      const [tag, ...options] = entry.trim().split(';');
      const q = options.map((option) => option.trim()).find((option) => option.startsWith('q='));
      return { tag: tag.toLowerCase(), q: q ? Number(q.slice(2)) : 1, index };
    })
    .filter((entry) => entry.tag && !Number.isNaN(entry.q) && entry.q > 0)
    .sort((a, b) => b.q - a.q || a.index - b.index);
  for (const { tag } of ranked) {
    const language = tag.split('-')[0];
    if (isLocale(language)) {
      return language;
    }
  }
  return null;
}

export function resolveLocale(cookie: string | undefined, acceptLanguage: string | null): Locale {
  if (isLocale(cookie)) {
    return cookie;
  }
  return preferredLocale(acceptLanguage) ?? DEFAULT_LOCALE;
}
