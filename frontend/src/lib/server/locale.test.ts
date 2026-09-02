import { describe, expect, it } from 'vitest';
import { preferredLocale, resolveLocale } from './locale';

describe('resolveLocale', () => {
  it('prefers the cookie, then the weighted Accept-Language, then English', () => {
    expect(resolveLocale('zh', 'de')).toBe('zh');
    expect(resolveLocale('fr', 'de-AT,de;q=0.9,en;q=0.8')).toBe('de');
    expect(resolveLocale(undefined, 'ru-RU,ru;q=0.9,en;q=0.8')).toBe('ru');
    expect(resolveLocale(undefined, 'fr-FR,fr;q=0.9')).toBe('en');
    expect(resolveLocale(undefined, null)).toBe('en');
  });

  it('ranks by quality, not by position', () => {
    expect(preferredLocale('en;q=0.5, zh-CN;q=0.9')).toBe('zh');
    expect(preferredLocale('*')).toBeNull();
  });
});
