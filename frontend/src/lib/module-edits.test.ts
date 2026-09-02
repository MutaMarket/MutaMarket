import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  canEditCollectionNote,
  canSetPrice,
  cancelEdit,
  draftValue,
  editRequest,
  editSession,
  isValid,
  MAX_ASKING_PRICE,
  navCharacterIds,
  parsePrice,
  saveEdits,
  setDraft,
  showsEditRow,
  startEdit,
  storedValue,
  validPrice,
  type EditSession,
} from './module-edits';
import type { ModuleDetail } from './types';

function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id: 1,
    slug: 'a-module',
    type: { id: 47_808, name: 'Abyssal Something' },
    mutated_attributes: [],
    ...overrides,
  } as unknown as ModuleDetail;
}

function session(overrides: Partial<EditSession> = {}): EditSession {
  return { mode: 'note', collectionId: null, drafts: {}, ...overrides };
}

beforeEach(() => {
  cancelEdit();
});

describe('storedValue', () => {
  it('reads the note, the collection note and the asking price', () => {
    const withNote = module({ note: { id: 7, content: 'keep' } });
    expect(storedValue('note', withNote)).toBe('keep');
    expect(storedValue('note', module())).toBe('');

    const withCollectionNote = module({
      collection_note: {
        id: 3,
        content: 'shared',
        collection: { id: 5 },
      } as ModuleDetail['collection_note'],
    });
    expect(storedValue('collection-note', withCollectionNote)).toBe('shared');

    const listed = module({
      public_asset: {
        owner: { id: 90, name: 'Seller' },
        price: 1_500_000_000.4,
      },
    });
    expect(storedValue('price', listed)).toBe('1500000000');
    expect(storedValue('price', module())).toBe('');
  });

  it("reads an unpriced listing's zero as no price", () => {
    // The API sends 0, not null, for a listing nobody priced: the legacy
    // resource casts the missing subselect with `(float) null`.
    const unpriced = module({
      public_asset: { owner: { id: 90, name: 'Seller' }, price: 0 },
    });
    expect(storedValue('price', unpriced)).toBe('');
  });
});

describe('drafts', () => {
  it('starts from the stored value and keeps what the user typed', () => {
    const target = module({ note: { id: 7, content: 'keep' } });
    startEdit('note');
    expect(draftValue(get(editSession)!, target)).toBe('keep');

    setDraft(target, 'changed');
    expect(draftValue(get(editSession)!, target)).toBe('changed');
    expect(get(editSession)!.drafts[1]).toEqual({
      value: 'changed',
      stored: 'keep',
    });
  });
});

describe('editRequest', () => {
  it('is null when nothing changed', () => {
    expect(editRequest(session())).toBeNull();
    expect(editRequest(session({ drafts: { 1: { value: 'a', stored: 'a' } } }))).toBeNull();
  });

  it('posts only the changed notes, trimmed', () => {
    const request = editRequest(
      session({
        drafts: {
          1: { value: '  fresh  ', stored: '' },
          2: { value: 'same', stored: 'same' },
        },
      }),
    );
    expect(request).toEqual({
      path: '/notes',
      body: { notes: [{ module_id: 1, content: 'fresh' }] },
    });
  });

  it('carries the collection id for collection notes', () => {
    const request = editRequest(
      session({
        mode: 'collection-note',
        collectionId: 42,
        drafts: { 9: { value: 'shared', stored: '' } },
      }),
    );
    expect(request).toEqual({
      path: '/collection-notes',
      body: { collection_id: 42, notes: [{ module_id: 9, content: 'shared' }] },
    });
  });

  it('sends prices as numbers, clearing with a zero', () => {
    const request = editRequest(
      session({
        mode: 'price',
        drafts: {
          1: { value: '1,500,000,000', stored: '' },
          2: { value: '', stored: '900' },
          3: { value: '-5', stored: '900' },
        },
      }),
    );
    expect(request).toEqual({
      path: '/module-pricing',
      body: {
        module_pricing: [
          { module_id: 1, price: 1_500_000_000 },
          { module_id: 2, price: 0 },
          { module_id: 3, price: 0 },
        ],
      },
    });
  });
});

describe('parsePrice', () => {
  it('accepts separators and reads empty as clearing the price', () => {
    expect(parsePrice('1500000000')).toBe(1_500_000_000);
    expect(parsePrice('1,500,000,000')).toBe(1_500_000_000);
    expect(parsePrice('  ')).toBe(0);
    expect(parsePrice('')).toBe(0);
    expect(parsePrice('1.5b')).toBeNull();
    expect(parsePrice('abc')).toBeNull();
  });
});

describe('isValid', () => {
  it('rejects a price above the ceiling', () => {
    expect(validPrice(String(MAX_ASKING_PRICE))).toBe(true);
    expect(validPrice(String(MAX_ASKING_PRICE + 1))).toBe(false);
    expect(validPrice('abc')).toBe(false);
    expect(
      isValid(
        session({
          mode: 'price',
          drafts: { 1: { value: '2000000000000', stored: '' } },
        }),
      ),
    ).toBe(false);
  });

  it('only guards price drafts', () => {
    expect(isValid(session({ drafts: { 1: { value: 'anything', stored: '' } } }))).toBe(true);
    expect(isValid(session({ mode: 'price', drafts: { 1: { value: 'abc', stored: '' } } }))).toBe(
      false,
    );
    expect(isValid(session({ mode: 'price', drafts: { 1: { value: '10', stored: '' } } }))).toBe(
      true,
    );
  });
});

describe('showsEditRow', () => {
  it('shows a stored note outside a session', () => {
    expect(showsEditRow('note', module({ note: { id: 1, content: 'x' } }), null, true)).toBe(true);
    expect(showsEditRow('note', module(), null, true)).toBe(false);
  });

  it('never shows a read-only price row', () => {
    const listed = module({
      public_asset: { owner: { id: 90, name: 'S' }, price: 100 },
    });
    expect(showsEditRow('price', listed, null, true)).toBe(false);
  });

  it('shows the edited mode only when the viewer is allowed', () => {
    const running = session({ mode: 'price' });
    expect(showsEditRow('price', module(), running, true)).toBe(true);
    expect(showsEditRow('price', module(), running, false)).toBe(false);
    expect(showsEditRow('note', module(), running, true)).toBe(false);
  });
});

describe('permissions', () => {
  it('lets only the listing owner set a price', () => {
    const listed = module({
      public_asset: { owner: { id: 90, name: 'S' }, price: null },
    });
    expect(canSetPrice(listed, [90])).toBe(true);
    expect(canSetPrice(listed, [91])).toBe(false);
    expect(canSetPrice(module(), [90])).toBe(false);
  });

  it('lets only the collection owner edit its notes', () => {
    expect(canEditCollectionNote({ characterId: 5 }, [5])).toBe(true);
    expect(canEditCollectionNote({ characterId: 5 }, [6])).toBe(false);
    expect(canEditCollectionNote(null, [5])).toBe(false);
  });

  it('reads character ids off an untyped nav payload', () => {
    expect(navCharacterIds({ characters: [{ id: 1 }, { id: 2 }] })).toEqual([1, 2]);
    expect(navCharacterIds(null)).toEqual([]);
    expect(navCharacterIds(undefined)).toEqual([]);
  });
});

describe('saveEdits', () => {
  it('posts the request and ends the session', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, type: 'basic', status: 200 });
    vi.stubGlobal('fetch', fetchMock);

    const target = module();
    startEdit('note');
    setDraft(target, 'written');
    await expect(saveEdits()).resolves.toBe('saved');

    expect(fetchMock).toHaveBeenCalledWith('/notes', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ notes: [{ module_id: 1, content: 'written' }] }),
      redirect: 'manual',
    });
    expect(get(editSession)).toBeNull();
    vi.unstubAllGlobals();
  });

  it('treats the endpoints referer redirect as success', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: false, type: 'opaqueredirect', status: 0 }),
    );
    startEdit('note');
    setDraft(module(), 'written');
    await expect(saveEdits()).resolves.toBe('saved');
    vi.unstubAllGlobals();
  });

  it('keeps the session open when the post fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, type: 'basic', status: 500 }));
    startEdit('note');
    setDraft(module(), 'written');
    await expect(saveEdits()).resolves.toBe('failed');
    expect(get(editSession)).not.toBeNull();
    vi.unstubAllGlobals();
  });

  it('closes without posting when nothing changed', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    startEdit('note');
    await expect(saveEdits()).resolves.toBe('unchanged');
    expect(fetchMock).not.toHaveBeenCalled();
    expect(get(editSession)).toBeNull();
    vi.unstubAllGlobals();
  });
});
