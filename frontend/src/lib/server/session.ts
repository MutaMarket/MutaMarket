// Whether an SSR load can expect an account-only API call to work.
//
// The API answers a request without a session with 401, which the admin
// console's route roll-up counts as an error like any other: a loader
// that calls an account endpoint for everyone turns every guest render
// into a logged failure. Checking the cookie costs nothing and skips the
// call instead. An expired cookie still ends in a 401, which is rare
// enough to be noise.
import type { Cookies } from '@sveltejs/kit';

/** Mirrors SESSION_COOKIE in src/auth/session.rs, the authority. */
export const SESSION_COOKIE = 'mm_session';

export function hasSession(cookies: Pick<Cookies, 'get'>): boolean {
  return cookies.get(SESSION_COOKIE) !== undefined;
}
