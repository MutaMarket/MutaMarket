import { existsSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

import abyssals from './abyssals.json';
import { CATALOG, SECTION_KEYS, catalogTypeIds, iconForType } from './catalog';
import en from './i18n/locales/en/misc.json';

// `abyssals.json` and the type icons are written by `sde_import` from the
// seeded mutaplasmid output types; the catalogue below them is the one
// hand-kept piece, so these pin it to what the import produced. A type
// CCP adds fails here until it has a place in the picker.

/** The icon directory the API serves at `/img/icons`. */
const ICON_DIR = '../assets/img/icons';

/** The `misc.typeDialog` namespace the section keys resolve against. */
const TYPE_DIALOG_KEYS = new Set(Object.keys(en.typeDialog));

describe('catalog coverage', () => {
  it('links every abyssal type exactly once', () => {
    const linked = catalogTypeIds();

    expect(new Set(linked).size).toBe(linked.length);
    expect([...linked].sort((a, b) => a - b)).toEqual(
      abyssals.map((abyssal) => abyssal.id).sort((a, b) => a - b),
    );
  });

  it('resolves an icon stem for every linked type', () => {
    for (const typeId of catalogTypeIds()) {
      expect(iconForType(typeId), `icon stem for ${typeId}`).not.toBeNull();
    }
  });

  it('ships an icon file for every stem it renders', () => {
    const stems = CATALOG.flat().flatMap((section) => section.entries.map((entry) => entry.icon));

    for (const stem of stems) {
      expect(existsSync(`${ICON_DIR}/${stem}.png`), `${stem}.png`).toBe(true);
    }
  });

  it('translates every section title', () => {
    for (const section of CATALOG.flat()) {
      const key = SECTION_KEYS[section.title];
      expect(key, `section key for ${section.title}`).toBeDefined();
      expect(TYPE_DIALOG_KEYS, section.title).toContain(key.split('.').at(-1));
    }
  });
});
