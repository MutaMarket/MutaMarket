import { describe, expect, it } from 'vitest';
import { getLocale, message, seedLocale, segments, setLocale, t } from './i18n.svelte';

describe('t', () => {
  it('fills named params and picks the plural form by count like vue-i18n', () => {
    seedLocale('en');
    expect(t('calculator.omega.monthsLabel', { count: 1 })).toBe('1 Month');
    expect(t('calculator.omega.monthsLabel', { count: 3 })).toBe('3 Months');
    expect(t('calculator.omega.monthsLabel', { count: 0 })).toBe('0 Months');
    expect(t('modules.findAsset.countFromTop', { index: 7 })).toBe(
      'Count from the top until you reach module 7',
    );
  });

  it('falls back to English for a missing translation and to the key for a missing key', () => {
    setLocale('de');
    expect(getLocale()).toBe('de');
    expect(t('nav.localeSwitcher.label')).toBe(message('nav.localeSwitcher.label', 'de'));
    expect(t('no.such.key')).toBe('no.such.key');
    expect(document.documentElement.lang).toBe('de');
    expect(document.cookie).toContain('locale=de');
    setLocale('en');
  });

  it('splits a message into text and placeholders for markup slots', () => {
    seedLocale('en');
    expect(segments('modules.findAsset.belongsTo', { character: 'X', station: 'Y' })).toEqual([
      { text: 'This module belongs to ' },
      { slot: 'character' },
      { text: ' and is located in ' },
      { slot: 'station' },
      { text: '.' },
    ]);
  });
});
