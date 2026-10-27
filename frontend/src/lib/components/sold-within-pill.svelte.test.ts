import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { userEvent } from 'vitest/browser';

const goto = vi.fn();
vi.mock('$app/navigation', () => ({ goto }));

const SoldWithinPill = (await import('./sold-within-pill.svelte')).default;

import { defaultUiSearch } from '#lib/query.ts';

const BASE = { ...defaultUiSearch(), typeSlug: 'x', page: 3 };

afterEach(() => {
  goto.mockReset();
});

describe('sold-within-pill.svelte', () => {
  it('applies the typed day count from page 1', async () => {
    const screen = await render(SoldWithinPill, {
      props: { prefix: 'historic-sales', search: BASE },
    });
    const input = screen.getByRole('textbox', { name: 'Sold in the last days' });
    await input.fill('14');
    await userEvent.keyboard('{Enter}');

    expect(goto).toHaveBeenCalledExactlyOnceWith('/historic-sales/type/x/sold-within/14', {
      reset: false,
    });
    // The search prop never followed the navigation, so the field's
    // later blur would apply again.
    screen.unmount();
  });

  it('drops the window when cleared or not positive, and skips unchanged values', async () => {
    const screen = await render(SoldWithinPill, {
      props: { prefix: 'historic-sales', search: { ...BASE, soldWithin: 7 } },
    });
    const input = screen.getByRole('textbox', { name: 'Sold in the last days' });
    await expect.element(input).toHaveValue('7');

    await input.fill('7');
    await userEvent.keyboard('{Enter}');
    expect(goto).not.toHaveBeenCalled();

    await input.fill('0');
    await userEvent.keyboard('{Enter}');
    expect(goto).toHaveBeenCalledExactlyOnceWith('/historic-sales/type/x', { reset: false });
  });
});
