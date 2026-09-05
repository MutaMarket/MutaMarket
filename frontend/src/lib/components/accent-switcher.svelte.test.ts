import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const goto = vi.fn();
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn() }));

const AccentSwitcher = (await import('./accent-switcher.svelte')).default;

import { DEFAULT_ACCENT_SWATCH, LEGACY_ORANGE, PREMIUM_ACCENTS } from '$lib/accent';

async function openPicker(hasPremium: boolean, accent: string | null = null) {
  const screen = await render(AccentSwitcher, { props: { accent, hasPremium } });
  await screen.getByRole('button', { name: 'Change theme color' }).click();
  return screen;
}

function swatches(): HTMLButtonElement[] {
  return Array.from(document.querySelectorAll<HTMLButtonElement>('[role="group"] button'));
}

function swatchColor(button: HTMLButtonElement): string {
  return button.style.backgroundColor;
}

function rgb(hex: string): string {
  const value = parseInt(hex.slice(1), 16);
  return `rgb(${value >> 16}, ${(value >> 8) & 255}, ${value & 255})`;
}

afterEach(() => {
  goto.mockReset();
  vi.restoreAllMocks();
});

describe('accent-switcher.svelte', () => {
  it('offers the default and the legacy orange free and locks the rest without premium', async () => {
    await openPicker(false);

    const all = swatches();
    expect(all.map(swatchColor)).toEqual(
      [DEFAULT_ACCENT_SWATCH, LEGACY_ORANGE, ...PREMIUM_ACCENTS].map(rgb),
    );
    expect(all.slice(0, 2).map((button) => button.dataset.locked)).toEqual([undefined, undefined]);
    expect(all.slice(2).every((button) => button.dataset.locked === 'true')).toBe(true);
    expect(document.querySelector('a[href="/premium"]')?.textContent).toContain(
      'Unlock every color with premium',
    );
  });

  it('unlocks everything with premium and marks the saved color', async () => {
    await openPicker(true, PREMIUM_ACCENTS[1]);

    const all = swatches();
    expect(all.every((button) => button.dataset.locked === undefined)).toBe(true);
    expect(all.map((button) => button.getAttribute('aria-pressed'))).toEqual(
      all.map((button) => (swatchColor(button) === rgb(PREMIUM_ACCENTS[1]) ? 'true' : 'false')),
    );
    expect(document.querySelector('a[href="/premium"]')).toBeNull();
  });

  it('sends a locked pick to the premium page', async () => {
    const fetchMock = vi
      .spyOn(window, 'fetch')
      .mockResolvedValue(new Response('{}', { status: 200 }));
    await openPicker(false);

    swatches()[2].click();
    expect(goto).toHaveBeenCalledWith('/premium');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('saves a free pick through the API', async () => {
    const fetchMock = vi
      .spyOn(window, 'fetch')
      .mockResolvedValue(new Response('{}', { status: 200 }));
    await openPicker(false);

    swatches()[1].click();
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(fetchMock.mock.calls[0][0]).toBe('/settings/accent');
    expect(JSON.parse(String(fetchMock.mock.calls[0][1]?.body))).toEqual({
      accent_color: LEGACY_ORANGE,
    });
  });

  it('previews a hovered swatch in the head and drops it on leave', async () => {
    await openPicker(false);
    const orange = swatches()[1];

    orange.dispatchEvent(new MouseEvent('mouseenter'));
    await vi.waitFor(() =>
      expect(document.head.innerHTML).toContain(`--primary:${LEGACY_ORANGE}!important`),
    );
    orange.dispatchEvent(new FocusEvent('blur'));
    await vi.waitFor(() =>
      expect(document.head.innerHTML).not.toContain(`--primary:${LEGACY_ORANGE}!important`),
    );
  });
});
