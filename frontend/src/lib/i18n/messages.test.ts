import { describe, expect, it } from 'vitest';
import { LOCALES, MESSAGES } from './messages';

describe('the translation catalogue', () => {
  it('has every English key in every locale and nothing more', () => {
    // The legacy check-i18n script: English is the source of truth and
    // de/zh mirror its key set exactly.
    const english = Object.keys(MESSAGES.en).sort();
    expect(english.length).toBeGreaterThan(900);
    for (const { value } of LOCALES) {
      expect(Object.keys(MESSAGES[value]).sort()).toEqual(english);
    }
  });
});
