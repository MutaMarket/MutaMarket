import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { get } from 'svelte/store';

import ModuleEditRow from './module-edit-row.svelte';
import { t } from '$lib/i18n.svelte';
import { cancelEdit, editSession, setDraft, startEdit } from '$lib/module-edits';
import type { ModuleDetail } from '$lib/types';

function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id: 7,
    slug: 'a-module-7',
    type: { id: 47_800, name: 'Abyssal Heat Sink' },
    mutated_attributes: [],
    ...overrides,
  } as unknown as ModuleDetail;
}

afterEach(() => cancelEdit());

describe('the note row', () => {
  it('renders nothing without a note and without a session', async () => {
    const screen = render(ModuleEditRow, {
      module: module(),
      mode: 'note',
      allowed: true,
    });
    await expect.element(screen.baseElement).toBeInTheDocument();
    expect(screen.baseElement.querySelector('textarea')).toBeNull();
    expect(screen.baseElement.textContent?.trim()).toBe('');
  });

  it('shows a stored note as text', async () => {
    const screen = render(ModuleEditRow, {
      module: module({ note: { id: 1, content: 'watch this roll' } }),
      mode: 'note',
      allowed: true,
    });
    await expect.element(screen.getByText('watch this roll')).toBeInTheDocument();
  });

  it('swaps to a field seeded with the stored note while editing', async () => {
    startEdit('note');
    const screen = render(ModuleEditRow, {
      module: module({ note: { id: 1, content: 'watch this roll' } }),
      mode: 'note',
      allowed: true,
    });
    const field = screen.getByLabelText(t('modules.card.note'));
    await expect.element(field).toHaveValue('watch this roll');

    await field.fill('changed');
    expect(get(editSession)?.drafts[7]).toEqual({
      value: 'changed',
      stored: 'watch this roll',
    });
  });

  it('stays out of the way when the viewer may not edit', async () => {
    startEdit('note');
    const screen = render(ModuleEditRow, {
      module: module(),
      mode: 'note',
      allowed: false,
    });
    expect(screen.baseElement.querySelector('textarea')).toBeNull();
  });

  it('ignores a session for another mode', async () => {
    startEdit('price');
    const screen = render(ModuleEditRow, {
      module: module(),
      mode: 'note',
      allowed: true,
    });
    expect(screen.baseElement.querySelector('textarea')).toBeNull();
  });
});

describe('the asking-price row', () => {
  it('only appears while prices are being edited', async () => {
    const listed = module({
      public_asset: { owner: { id: 90, name: 'Seller' }, price: 1_500_000_000 },
    });
    const screen = render(ModuleEditRow, {
      module: listed,
      mode: 'price',
      allowed: true,
    });
    expect(screen.baseElement.querySelector('input')).toBeNull();

    startEdit('price');
    // A number field reports a number, not the string it was seeded with.
    await expect.element(screen.getByLabelText('Asking price')).toHaveValue(1_500_000_000);
  });

  it('floats the short form of what was typed beside the field', async () => {
    const listed = module({
      public_asset: { owner: { id: 90, name: 'Seller' }, price: null },
    });
    startEdit('price');
    const screen = render(ModuleEditRow, {
      module: listed,
      mode: 'price',
      allowed: true,
    });

    setDraft(listed, '1500000000');
    await expect.element(screen.getByText('1.5B')).toBeInTheDocument();

    setDraft(listed, '');
    await expect.element(screen.getByText('No price specified')).toBeInTheDocument();
  });
});
