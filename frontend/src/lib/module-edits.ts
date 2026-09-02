// Bulk editing of notes, collection notes and asking prices, the legacy
// useNote / useCollectionNote / useAskingPrice trio: a menu entry or a
// page button turns one mode on, every card then renders its own field,
// and a single floating bar posts every change at once.
//
// Two divergences from legacy, which kept three independent editing
// flags: only one mode runs at a time (legacy pinned all three floating
// bars to the same spot, where they overlapped), and only fields the
// user actually changed are posted (legacy re-sent every rendered card).
import { get, writable } from 'svelte/store';
import type { CollectionNoteRef, ModuleDetail, NavState, NoteRef } from './types';

export type EditMode = 'note' | 'collection-note' | 'price';

export interface EditSession {
  mode: EditMode;
  /** The collection a `collection-note` session writes into. */
  collectionId: number | null;
  /** module id → what the user typed next to what was there when the
   * field first rendered. Absent until they touch a field, which is
   * what keeps an untouched card out of the request. */
  drafts: Record<number, { value: string; stored: string }>;
}

/** The running session; null means nothing is being edited. */
export const editSession = writable<EditSession | null>(null);

/** The collection page currently open, so the menu knows whether a
 * collection note is possible and which collection it belongs to. */
export const openCollection = writable<{
  id: number;
  characterId: number;
} | null>(null);

export function startEdit(mode: EditMode, collectionId: number | null = null) {
  editSession.set({ mode, collectionId, drafts: {} });
}

export function cancelEdit() {
  editSession.set(null);
}

export function setDraft(module: ModuleDetail, value: string) {
  editSession.update((session) =>
    session === null
      ? session
      : {
          ...session,
          drafts: {
            ...session.drafts,
            [module.id]: { value, stored: storedValue(session.mode, module) },
          },
        },
  );
}

/** What the module already has stored for the mode being edited. */
export function storedValue(mode: EditMode, module: ModuleDetail): string {
  switch (mode) {
    case 'note':
      return note(module)?.content ?? '';
    case 'collection-note':
      return collectionNote(module)?.content ?? '';
    case 'price': {
      // A listing with no asking price comes back as 0, not null: the
      // legacy resource casts the missing correlated subselect with
      // `(float) null`. Legacy then reads it with plain truthiness
      // everywhere, so 0 means "no price".
      const price = module.public_asset?.price ?? 0;
      return price > 0 ? String(Math.round(price)) : '';
    }
  }
}

/** What a field shows: the draft if the user has typed, else the stored
 * value. Editing a module, navigating away and coming back keeps the
 * draft, the reason legacy held these in a module-level store. */
export function draftValue(session: EditSession, module: ModuleDetail): string {
  return session.drafts[module.id]?.value ?? storedValue(session.mode, module);
}

export function note(module: ModuleDetail): NoteRef | null {
  return module.note ?? null;
}

export function collectionNote(module: ModuleDetail): CollectionNoteRef | null {
  return module.collection_note ?? null;
}

/** The character ids on the nav payload, which `page.data` carries
 * untyped, so every caller does not repeat the narrowing. */
export function navCharacterIds(nav: unknown): number[] {
  return ((nav as NavState | null | undefined)?.characters ?? []).map((character) => character.id);
}

/** The legacy `can_set_price`: only the character who owns the listing. */
export function canSetPrice(module: ModuleDetail, characterIds: number[]): boolean {
  const owner = module.public_asset?.owner.id;
  return owner !== undefined && characterIds.includes(owner);
}

/** The legacy collection-note `can_edit`: only the collection's owner,
 * even though the endpoint itself lets any signed-in user write. */
export function canEditCollectionNote(
  collection: { characterId: number } | null,
  characterIds: number[],
): boolean {
  return collection !== null && characterIds.includes(collection.characterId);
}

/** Whether a card renders a row for this mode: something is stored to
 * show, or the running session is editing this mode and the viewer is
 * allowed to. Shared with the card's masonry row-span math, which has to
 * reach the same answer. */
export function showsEditRow(
  mode: EditMode,
  module: ModuleDetail,
  session: EditSession | null,
  allowed: boolean,
): boolean {
  if (session !== null && session.mode === mode) {
    return allowed;
  }
  switch (mode) {
    case 'note':
      return note(module) !== null;
    case 'collection-note':
      return collectionNote(module) !== null;
    case 'price':
      // The card's public-asset row already shows the asking price,
      // so unlike legacy there is no separate read-only price row.
      return false;
  }
}

export interface EditRequest {
  path: string;
  body: unknown;
}

/** The request a session's changed drafts add up to, or null when
 * nothing changed. Prices below zero are clamped: the backend reads any
 * non-positive number as "clear the asking price". */
export function editRequest(session: EditSession): EditRequest | null {
  const changed = Object.entries(session.drafts).filter(
    ([, draft]) => draft.value !== draft.stored,
  );
  if (changed.length === 0) {
    return null;
  }

  if (session.mode === 'price') {
    const module_pricing = changed.map(([id, draft]) => ({
      module_id: Number(id),
      price: Math.max(0, parsePrice(draft.value) ?? 0),
    }));
    return { path: '/module-pricing', body: { module_pricing } };
  }

  const notes = changed.map(([id, draft]) => ({
    module_id: Number(id),
    content: draft.value.trim(),
  }));
  return session.mode === 'note'
    ? { path: '/notes', body: { notes } }
    : {
        path: '/collection-notes',
        body: { collection_id: session.collectionId, notes },
      };
}

/** Ceiling for an asking price, one trillion ISK, matching the server's
 * MAX_ASKING_PRICE in `src/server/pricing.rs`. Anything above it is a
 * typo; the endpoint rejects it, so the save button blocks it first. */
export const MAX_ASKING_PRICE = 1_000_000_000_000;

/** A price field's value as a number: digits with optional separators,
 * `''` meaning "clear it". Anything else is not a price. */
export function parsePrice(value: string): number | null {
  const cleaned = value.replaceAll(/[,\s_]/g, '').trim();
  if (cleaned === '') {
    return 0;
  }
  const parsed = Number(cleaned);
  return Number.isFinite(parsed) ? parsed : null;
}

/** A price the endpoint will take: a number, and within the ceiling. */
export function validPrice(value: string): boolean {
  const price = parsePrice(value);
  return price !== null && price <= MAX_ASKING_PRICE;
}

/** Every draft in the session parses. Guards the save button. */
export function isValid(session: EditSession): boolean {
  return (
    session.mode !== 'price' ||
    Object.values(session.drafts).every((draft) => validPrice(draft.value))
  );
}

export async function saveEdits(): Promise<'saved' | 'unchanged' | 'failed'> {
  const session = get(editSession);
  if (session === null) {
    return 'unchanged';
  }
  const request = editRequest(session);
  if (request === null) {
    cancelEdit();
    return 'unchanged';
  }

  // The endpoints answer the legacy referer redirect, which fetch
  // follows back into a page load we do not want; `manual` turns that
  // into an opaque success.
  const response = await fetch(request.path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(request.body),
    redirect: 'manual',
  });
  if (!response.ok && response.type !== 'opaqueredirect' && response.status !== 0) {
    return 'failed';
  }

  cancelEdit();
  return 'saved';
}
