import type { Handle, HandleFetch } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { LOCALE_COOKIE, LOCALE_COOKIE_MAX_AGE, registerServerLocaleReader } from '$lib/i18n.svelte';
import { requestLocale, resolveLocale } from '$lib/server/locale';

registerServerLocaleReader(() => requestLocale.getStore() ?? 'en');

// The locale decision per request, re-queued as a cookie on every
// response like the legacy middleware, and carried through the render.
export const handle: Handle = async ({ event, resolve }) => {
  const locale = resolveLocale(
    event.cookies.get(LOCALE_COOKIE),
    event.request.headers.get('accept-language'),
  );
  event.cookies.set(LOCALE_COOKIE, locale, {
    path: '/',
    httpOnly: false,
    sameSite: 'lax',
    maxAge: LOCALE_COOKIE_MAX_AGE,
  });
  event.locals.locale = locale;
  return requestLocale.run(locale, () =>
    resolve(event, {
      transformPageChunk: ({ html }) => html.replace('%lang%', locale),
    }),
  );
};

// Server-side loads reach Axum directly (the browser goes through the
// shared-origin proxy instead). The cookie forward is what keeps SSR
// authenticated: without it every render is a guest.
export const handleFetch: HandleFetch = async ({ event, request, fetch }) => {
  const url = new URL(request.url);
  if (url.origin === event.url.origin && url.pathname.startsWith('/api')) {
    const axum = env.AXUM_URL ?? 'http://127.0.0.1:3000';
    request = new Request(new URL(url.pathname + url.search, axum), request);
    const cookie = event.request.headers.get('cookie');
    if (cookie) {
      request.headers.set('cookie', cookie);
    }
  }

  return fetch(request);
};
