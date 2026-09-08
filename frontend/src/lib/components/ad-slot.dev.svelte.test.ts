import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The development placeholder, split from ad-slot.svelte.test.ts because
// `dev` is fixed per module mock.
vi.mock('$app/state', () => ({
  page: { data: { nav: null }, url: new URL('https://mutamarket.com/') },
}));
vi.mock('$app/environment', () => ({ browser: true, building: false, dev: true, version: 'test' }));
vi.mock('$env/dynamic/public', () => ({ env: { PUBLIC_ADSENSE_CLIENT_ID: '' } }));

const AdSlot = (await import('./ad-slot.svelte')).default;

describe('ad-slot.svelte in development', () => {
  it('draws a labeled placeholder of every unit, dormant ones included', async () => {
    await render(AdSlot, { unit: 'inFeed', minHeight: 250 });
    await new Promise((resolve) => setTimeout(resolve, 50));

    const box = document.querySelector<HTMLElement>('[data-testid="ad-placeholder"]');
    expect(box?.style.minHeight).toBe('250px');
    expect(box?.textContent).toContain('inFeed');
    expect(box?.textContent).toContain('responsive, min 250px');
    expect(box?.textContent).toContain('no unit id');
    expect(document.querySelector('ins.adsbygoogle')).toBeNull();
    expect(window.adsbygoogle).toBeUndefined();
  });
});
