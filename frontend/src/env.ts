import { defineEnvVars } from '@sveltejs/kit/env';

/** Where SSR reaches Axum when AXUM_URL is unset: the API's default BIND_ADDR. */
const DEFAULT_AXUM_URL = 'http://127.0.0.1:3000';

/** The plain Markee Dragon store, without an affiliate id. */
const DEFAULT_MARKEEDRAGON_URL = 'https://store.markeedragon.com/';

// Unset (or empty) variables read as the empty string unless a default
// is given; empty partner links hide their card, button or coupon strip.
export const variables = defineEnvVars({
  AXUM_URL: { schema: (input) => input || DEFAULT_AXUM_URL },
  PUBLIC_ADSENSE_CLIENT_ID: { public: true, schema: (input) => input ?? '' },
  PUBLIC_KOFI_LINK: { public: true, schema: (input) => input ?? '' },
  PUBLIC_PATREON_LINK: { public: true, schema: (input) => input ?? '' },
  PUBLIC_MARKEEDRAGON_URL: { public: true, schema: (input) => input || DEFAULT_MARKEEDRAGON_URL },
  PUBLIC_MARKEEDRAGON_CODE: { public: true, schema: (input) => input ?? '' },
});
