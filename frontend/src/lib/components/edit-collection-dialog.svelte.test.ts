import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const goto = vi.fn();
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn() }));

const EditCollectionDialog = (await import('./edit-collection-dialog.svelte')).default;

import type { CollectionCardData } from '$lib/types-social';

const collection: CollectionCardData = {
  id: 7,
  slug: 'shiny-rolls-7',
  name: 'Shiny Rolls',
  description: 'The good ones',
  visibility: 'private',
  character_id: 91,
  character_name: 'Pilot',
  character_has_premium: false,
  modules_count: 3,
  type_ids: [47408],
  types_count: 1,
};

function redirected(pathname: string) {
  return { redirected: true, url: `http://localhost${pathname}` } as Response;
}

afterEach(() => {
  goto.mockReset();
  vi.restoreAllMocks();
});

describe('edit-collection-dialog.svelte', () => {
  it('seeds the form from the collection', async () => {
    const screen = await render(EditCollectionDialog, {
      open: true,
      collection,
    });

    await expect.element(screen.getByLabelText('Name')).toHaveValue('Shiny Rolls');
    await expect.element(screen.getByLabelText('Description')).toHaveValue('The good ones');
    await expect.element(screen.getByRole('radio', { name: 'Private' })).toBeChecked();
  });

  it('puts the edited fields and follows the renamed slug', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(redirected('/collections/best-rolls-7'));
    const screen = await render(EditCollectionDialog, {
      open: true,
      collection,
    });

    await screen.getByLabelText('Name').fill('Best Rolls');
    await screen.getByLabelText('Description').fill('');
    await screen.getByRole('radio', { name: 'Public' }).click();
    await screen.getByRole('button', { name: 'Save' }).click();

    expect(fetchMock).toHaveBeenCalledWith('/collections/shiny-rolls-7', {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        name: 'Best Rolls',
        description: null,
        visibility: 'public',
      }),
      redirect: 'follow',
    });
    await vi.waitFor(() =>
      expect(goto).toHaveBeenCalledWith('/collections/best-rolls-7', {
        invalidateAll: true,
      }),
    );
  });

  it('shows the field errors of a rejected update', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'The given data was invalid.',
          errors: {
            description: ['The description field must not be greater than 255 characters.'],
          },
        }),
        { status: 422, headers: { 'content-type': 'application/json' } },
      ),
    );
    const screen = await render(EditCollectionDialog, {
      open: true,
      collection,
    });

    await screen.getByLabelText('Description').fill('x'.repeat(300));
    await screen.getByRole('button', { name: 'Save' }).click();

    await expect
      .element(screen.getByText('The description field must not be greater than 255 characters.'))
      .toBeInTheDocument();
    expect(goto).not.toHaveBeenCalled();
  });

  it('cannot save an empty name', async () => {
    const screen = await render(EditCollectionDialog, {
      open: true,
      collection,
    });

    await screen.getByLabelText('Name').fill('');
    await expect.element(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });
});
