import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const state = { page: { data: { nav: null as unknown }, url: new URL('https://mutamarket.com/') } };
vi.mock('$app/state', () => state);
vi.mock('$app/environment', () => ({
  browser: true,
  building: false,
  dev: false,
  version: 'test',
}));
vi.mock('$env/dynamic/public', () => ({ env: { PUBLIC_ADSENSE_CLIENT_ID: 'ca-pub-1' } }));

const AdBanner = (await import('./ad-banner.svelte')).default;

afterEach(() => {
  delete window.adsbygoogle;
  state.page.data.nav = null;
});

describe('ad-banner.svelte', () => {
  it('carries the three legacy banner units, one per breakpoint', async () => {
    await render(AdBanner);
    await new Promise((resolve) => setTimeout(resolve, 50));

    const units = [...document.querySelectorAll<HTMLElement>('ins.adsbygoogle')].map((ins) => [
      ins.getAttribute('data-ad-slot'),
      ins.style.width,
      ins.style.height,
      ins.parentElement?.className.trim(),
    ]);
    expect(units).toEqual([
      ['1526043502', '970px', '90px', 'hidden lg:block 3xl:hidden'],
      ['1300906129', '728px', '90px', 'hidden md:block lg:hidden'],
      ['7674742784', '300px', '100px', 'md:hidden'],
    ]);
  });

  it('leaves no gap for premium accounts', async () => {
    state.page.data.nav = { user: { has_premium: true } };
    await render(AdBanner);

    expect(document.querySelector('[data-testid="ad-banner"]')).toBeNull();
  });
});
