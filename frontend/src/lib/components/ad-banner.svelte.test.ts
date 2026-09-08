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
  it('mounts exactly the legacy banner unit of the viewport width', async () => {
    await render(AdBanner);
    await new Promise((resolve) => setTimeout(resolve, 50));

    const units = [...document.querySelectorAll<HTMLElement>('ins.adsbygoogle')].map((ins) => [
      ins.getAttribute('data-ad-slot'),
      ins.style.width,
      ins.style.height,
    ]);
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
    const width = window.innerWidth / rem;
    const expected =
      width < 48
        ? ['7674742784', '300px', '100px']
        : width < 64
          ? ['1300906129', '728px', '90px']
          : ['1526043502', '970px', '90px'];
    expect(units).toEqual([expected]);
    expect(window.adsbygoogle).toEqual([{}]);
  });

  it('takes its margins only once the unit is filled', async () => {
    await render(AdBanner);
    await new Promise((resolve) => setTimeout(resolve, 50));
    const strip = document.querySelector<HTMLElement>('[data-testid="ad-banner"]');
    expect(strip?.classList).not.toContain('my-2');

    document.querySelector<HTMLElement>('ins.adsbygoogle')!.dataset.adStatus = 'filled';
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(strip?.classList).toContain('my-2');
  });

  it('leaves no gap for premium accounts', async () => {
    state.page.data.nav = { user: { has_premium: true } };
    await render(AdBanner);

    expect(document.querySelector('[data-testid="ad-banner"]')).toBeNull();
  });
});
