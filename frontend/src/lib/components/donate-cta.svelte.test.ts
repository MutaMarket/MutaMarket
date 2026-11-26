import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import DonateCta from './donate-cta.svelte';
import { DEFAULT_PREMIUM } from '#lib/premium.ts';

const FEATURES = [
  'Historic sales',
  'Similar sold modules',
  'Search alerts',
  'Priority ordering',
  'Gold name',
  'Theme color',
];

describe('the donate call to action', () => {
  it('asks a premium donor to extend, names the price and features, and hints at ad-free', async () => {
    const screen = await render(DonateCta, {
      premium: DEFAULT_PREMIUM,
      hasDonated: true,
      hasPremium: true,
      adFree: false,
    });
    await expect
      .element(screen.getByRole('heading', { name: 'Donate again and extend your premium' }))
      .toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain(
      '100 million ISK sent in game to MutaMate buys the sending character a month of:',
    );
    expect(
      [...screen.baseElement.querySelectorAll('li')].map((item) => item.textContent?.trim()),
    ).toEqual(FEATURES);
    expect(
      screen.baseElement.querySelector('[data-testid="ad-free-hint"] span')?.textContent?.trim(),
    ).toBe(
      'Tired of ads? Premium + Ad-free removes every ad on the site for 300 million ISK a month.',
    );
    expect(screen.getByRole('link', { name: 'Go ad-free' }).element().getAttribute('href')).toBe(
      '/premium',
    );
    expect(screen.getByRole('link', { name: 'Details' }).element().getAttribute('href')).toBe(
      '/premium',
    );
  });

  it('copies the donation character', async () => {
    const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    const screen = await render(DonateCta, {
      premium: { ...DEFAULT_PREMIUM, premium_character: 'Isk Sink' },
      hasDonated: false,
      hasPremium: false,
      adFree: false,
    });
    await screen.getByRole('button', { name: 'Copy Isk Sink' }).click();
    expect(writeText).toHaveBeenCalledWith('Isk Sink');
    writeText.mockRestore();
  });

  it('words the title by donation and premium state', async () => {
    const first = await render(DonateCta, {
      premium: DEFAULT_PREMIUM,
      hasDonated: false,
      hasPremium: false,
      adFree: false,
    });
    await expect
      .element(first.getByRole('heading', { name: 'Donate and unlock premium' }))
      .toBeInTheDocument();
    first.unmount();

    const lapsed = await render(DonateCta, {
      premium: DEFAULT_PREMIUM,
      hasDonated: true,
      hasPremium: false,
      adFree: false,
    });
    await expect
      .element(lapsed.getByRole('heading', { name: 'Donate again and get premium back' }))
      .toBeInTheDocument();
  });

  it('drops the ad-free hint for an ad-free account', async () => {
    const screen = await render(DonateCta, {
      premium: DEFAULT_PREMIUM,
      hasDonated: true,
      hasPremium: true,
      adFree: true,
    });
    await expect
      .element(screen.getByRole('heading', { name: 'Donate again and extend your premium' }))
      .toBeInTheDocument();
    expect(screen.baseElement.querySelector('[data-testid="ad-free-hint"]')).toBeNull();
  });
});
