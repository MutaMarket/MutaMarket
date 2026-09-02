// The legacy Static/Abyssals list: every abyssal output type by id and
// name. The legacy QueryBuilder turned the id into the name slug for
// the type URL segment; `abyssalSlug`/`abyssalBySlug` port that mapping.

import abyssals from './abyssals.json';

export interface AbyssalType {
  id: number;
  name: string;
}

export const ABYSSALS: AbyssalType[] = abyssals;

function slugify(name: string): string {
  return name.replace(/ /g, '-').toLowerCase();
}

/** The legacy URL slug for an abyssal type id ("abyssal-stasis-webifier"),
 * or the bare id for a type missing from the static list (the backend
 * resolves both). */
export function abyssalSlug(typeId: number): string {
  const type = ABYSSALS.find((abyssal) => abyssal.id === typeId);
  return type === undefined ? String(typeId) : slugify(type.name);
}

/** Resolves a URL type segment (name slug or bare id) back to the
 * abyssal type, for showing the current selection without a server
 * round-trip. */
export function abyssalBySlug(slug: string | null): AbyssalType | null {
  if (slug === null || slug === '') {
    return null;
  }
  const id = Number(slug);
  if (Number.isInteger(id)) {
    return ABYSSALS.find((abyssal) => abyssal.id === id) ?? null;
  }
  const needle = slug.toLowerCase();
  return ABYSSALS.find((abyssal) => slugify(abyssal.name) === needle) ?? null;
}
