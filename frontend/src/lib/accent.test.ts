import { describe, expect, it } from 'vitest';
import { accentForeground, accentThemeCss, normalizeAccent } from './accent';

describe('normalizeAccent', () => {
  it('accepts only a six-digit hex, lowercased and trimmed', () => {
    expect(normalizeAccent('#A6E600')).toBe('#a6e600');
    expect(normalizeAccent('  #12ab34 ')).toBe('#12ab34');
    expect(normalizeAccent('a6e600')).toBeNull();
    expect(normalizeAccent('#a6e60')).toBeNull();
    expect(normalizeAccent('#a6e600; }')).toBeNull();
    expect(normalizeAccent('red')).toBeNull();
    expect(normalizeAccent(null)).toBeNull();
    expect(normalizeAccent(undefined)).toBeNull();
    expect(normalizeAccent('')).toBeNull();
  });
});

describe('accentForeground', () => {
  it('picks dark text on a light accent and light text on a dark one', () => {
    expect(accentForeground('#a6e600')).toBe('#0a0a0a');
    expect(accentForeground('#3b82f6')).toBe('#fafafa');
    expect(accentForeground('#000000')).toBe('#fafafa');
    expect(accentForeground('#ffffff')).toBe('#0a0a0a');
  });
});

describe('accentThemeCss', () => {
  it('is null for an absent or invalid color', () => {
    expect(accentThemeCss(null)).toBeNull();
    expect(accentThemeCss('nonsense')).toBeNull();
  });

  it('overrides the accent tokens with the normalized hex', () => {
    const css = accentThemeCss('#3b82f6');
    expect(css).toContain('--primary:#3b82f6!important');
    expect(css).toContain('--accent:#3b82f6!important');
    expect(css).toContain('--primary-foreground:#fafafa!important');
    expect(css).toContain('--ring:color-mix(in oklab,#3b82f6 60%,transparent)!important');
    // A would-be injection never reaches the string: it fails validation.
    expect(accentThemeCss('#abc123;} body{display:none}')).toBeNull();
    expect(accentThemeCss('red;}')).toBeNull();
  });
});
