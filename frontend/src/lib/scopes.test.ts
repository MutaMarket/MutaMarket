import { describe, expect, it } from 'vitest';
import {
  charactersNeedingScopes,
  grantUrl,
  missingScopes,
  requiredScopes,
  warnsAboutScopes,
  type ScopeInfo,
} from './scopes';
import type { AccountCharacter } from './types';

const CATALOGUE: ScopeInfo[] = [
  { id: 'assets', label: 'Asset import', description: '', optional: false },
  { id: 'contracts', label: 'Personal contracts', description: '', optional: false },
  { id: 'corp', label: 'Corporation assets', description: '', optional: true },
];

function character(overrides: Partial<AccountCharacter> = {}): AccountCharacter {
  return {
    id: 1,
    name: 'Pilot',
    corporation_id: null,
    has_asset_token: true,
    active: false,
    granted_scopes: ['assets', 'contracts'],
    scope_warnings_muted: false,
    ...overrides,
  };
}

describe('requiredScopes', () => {
  it('leaves out the optional ones', () => {
    expect(requiredScopes(CATALOGUE).map((scope) => scope.id)).toEqual(['assets', 'contracts']);
  });
});

describe('missingScopes', () => {
  it('is empty when every required scope is granted', () => {
    expect(missingScopes(character(), CATALOGUE)).toEqual([]);
  });

  it('never counts an ungranted optional scope', () => {
    const missing = missingScopes(character({ granted_scopes: ['assets'] }), CATALOGUE);
    expect(missing.map((scope) => scope.id)).toEqual(['contracts']);
  });

  it('lists everything for a character with no tokens left', () => {
    const missing = missingScopes(character({ granted_scopes: [] }), CATALOGUE);
    expect(missing.map((scope) => scope.id)).toEqual(['assets', 'contracts']);
  });
});

describe('warnsAboutScopes', () => {
  it('stays quiet once the user mutes the character', () => {
    const muted = character({ granted_scopes: [], scope_warnings_muted: true });
    expect(missingScopes(muted, CATALOGUE)).toHaveLength(2);
    expect(warnsAboutScopes(muted, CATALOGUE)).toBe(false);
  });

  it('warns about an unmuted character that lacks a scope', () => {
    expect(warnsAboutScopes(character({ granted_scopes: [] }), CATALOGUE)).toBe(true);
  });
});

describe('charactersNeedingScopes', () => {
  it('collects only the warning ones', () => {
    const characters = [
      character({ id: 1 }),
      character({ id: 2, granted_scopes: [] }),
      character({ id: 3, granted_scopes: [], scope_warnings_muted: true }),
    ];
    expect(charactersNeedingScopes(characters, CATALOGUE).map((c) => c.id)).toEqual([2]);
  });
});

describe('grantUrl', () => {
  it('asks for the granted scopes again so re-auth never narrows a token', () => {
    const url = grantUrl(character({ granted_scopes: ['corp'] }), CATALOGUE);
    const scopes = decodeURIComponent(url.split('scopes=')[1]).split(' ');
    expect(scopes.sort()).toEqual(['assets', 'contracts', 'corp']);
    expect(url).toContain('add_to_account=true');
  });
});
