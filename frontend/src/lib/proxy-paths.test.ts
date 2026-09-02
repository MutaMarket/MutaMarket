// The backend-owned URL prefixes live in three hand-synced places: the
// dev proxy's proxy-paths.ts and the @axum matchers of the compose and
// production Caddyfiles in deploy/. This suite parses all three and
// fails on drift.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import { axumPrefixes, axumWebsocketPrefix } from '../../proxy-paths.ts';

function caddyfile(relative: string): string {
  return readFileSync(fileURLToPath(new URL(relative, import.meta.url)), 'utf8');
}

// The @axum path matcher's tokens, line continuations included.
function axumMatcherTokens(caddy: string): string[] {
  const match = caddy.match(/@axum path((?:[^\n\\]*\\\n)*[^\n]*)/);
  if (!match) throw new Error('no "@axum path" matcher found');
  return match[1]
    .replaceAll('\\\n', ' ')
    .split(/\s+/)
    .filter((token) => token.length > 0);
}

// A matcher token names a prefix: "/api/*" and "/api" both mean /api.
function prefixesOf(tokens: string[]): string[] {
  const prefixes = new Set(tokens.map((token) => token.replace(/\/\*$/, '')));
  return [...prefixes].sort();
}

const expected = [...new Set([...axumPrefixes, axumWebsocketPrefix])].sort();
const docker = caddyfile('../../../deploy/Caddyfile.dev');
const deploy = caddyfile('../../../deploy/Caddyfile');

describe('backend path prefixes', () => {
  it('docker Caddyfile matches proxy-paths.ts', () => {
    expect(prefixesOf(axumMatcherTokens(docker))).toEqual(expected);
  });

  it('deploy Caddyfile matches proxy-paths.ts', () => {
    expect(prefixesOf(axumMatcherTokens(deploy))).toEqual(expected);
  });

  it('both Caddyfiles use identical matcher tokens', () => {
    expect(axumMatcherTokens(deploy)).toEqual(axumMatcherTokens(docker));
  });
});
