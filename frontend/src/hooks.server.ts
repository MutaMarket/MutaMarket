import type { HandleFetch } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

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
