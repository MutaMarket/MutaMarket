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
vi.mock('$lib/adsense', async (original) => ({
  ...(await original<typeof import('$lib/adsense')>()),
  AD_SLOTS: { bannerWide: '123', railWideLeft: '456', railNarrowLeft: '789', inFeed: '' },
}));
vi.mock('$env/dynamic/public', () => ({ env: { PUBLIC_ADSENSE_CLIENT_ID: 'ca-pub-1' } }));

const AdSlot = (await import('./ad-slot.svelte')).default;

async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 50));
}

afterEach(() => {
  delete window.adsbygoogle;
  state.page.data.nav = null;
});

describe('ad-slot.svelte', () => {
  it('renders a responsive unit and requests it', async () => {
    await render(AdSlot, { unit: 'bannerWide', minHeight: 250, fullWidthResponsive: true });
    await settle();

    const ins = document.querySelector<HTMLElement>('ins.adsbygoogle');
    expect(ins?.getAttribute('data-ad-client')).toBe('ca-pub-1');
    expect(ins?.getAttribute('data-ad-slot')).toBe('123');
    expect(ins?.getAttribute('data-ad-format')).toBe('auto');
    expect(ins?.getAttribute('data-full-width-responsive')).toBe('true');
    expect(ins?.style.display).toBe('block');
    expect(ins?.style.width).toBe('100%');
    expect(ins?.style.minHeight).toBe('250px');
    expect(window.adsbygoogle).toEqual([{}]);
  });

  it('passes the layout key of a fluid in-feed unit', async () => {
    await render(AdSlot, { unit: 'bannerWide', format: 'fluid', layoutKey: '+2t+rl+2h-1m-4u' });
    await settle();

    const ins = document.querySelector('ins.adsbygoogle');
    expect(ins?.getAttribute('data-ad-format')).toBe('fluid');
    expect(ins?.getAttribute('data-ad-layout-key')).toBe('+2t+rl+2h-1m-4u');
  });

  it('renders a fixed unit without the responsive attributes', async () => {
    await render(AdSlot, { unit: 'railWideLeft', width: 300, height: 600 });
    await settle();

    const ins = document.querySelector<HTMLElement>('ins.adsbygoogle');
    expect(ins?.style.display).toBe('inline-block');
    expect(ins?.style.width).toBe('300px');
    expect(ins?.style.height).toBe('600px');
    expect(ins?.hasAttribute('data-ad-format')).toBe(false);
    expect(ins?.hasAttribute('data-ad-layout-key')).toBe(false);
    expect(ins?.hasAttribute('data-full-width-responsive')).toBe(false);
  });

  it('does not exist while its media query fails', async () => {
    // A push serves the first unprocessed unit in the DOM, so a hidden
    // unit must not be there at all.
    await render(AdSlot, {
      unit: 'railNarrowLeft',
      width: 160,
      height: 600,
      media: '(width >= 99999px)',
    });
    await settle();

    expect(document.querySelector('ins.adsbygoogle')).toBeNull();
    expect(window.adsbygoogle).toBeUndefined();
  });

  it('exists while its media query matches', async () => {
    await render(AdSlot, { unit: 'railNarrowLeft', width: 160, height: 600, media: '(width > 0)' });
    await settle();

    expect(document.querySelector('ins.adsbygoogle')).not.toBeNull();
    expect(window.adsbygoogle).toEqual([{}]);
  });

  it('stays dormant without a unit id', async () => {
    await render(AdSlot, { unit: 'inFeed' });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')).toBeNull();
  });

  it('shows nothing to Patreon backers', async () => {
    state.page.data.nav = { user: { is_patreon_member: true } };
    await render(AdSlot, { unit: 'bannerWide' });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')).toBeNull();
  });

  it('reports the fill status AdSense stamps on the unit', async () => {
    const onstatus = vi.fn();
    await render(AdSlot, { unit: 'bannerWide', onstatus });
    await settle();
    expect(onstatus).toHaveBeenLastCalledWith('pending');

    const slot = document.querySelector<HTMLElement>('[data-testid="ad-slot"]');
    expect(slot?.dataset.status).toBe('pending');
    expect(slot?.classList).toContain('invisible');

    const ins = document.querySelector<HTMLElement>('ins.adsbygoogle');
    ins!.dataset.adStatus = 'filled';
    await settle();
    expect(onstatus).toHaveBeenLastCalledWith('filled');
    expect(slot?.dataset.status).toBe('filled');
    expect(slot?.classList).not.toContain('invisible');
    expect(slot?.classList).not.toContain('hidden');
  });

  it('collapses a unit AdSense leaves unfilled', async () => {
    await render(AdSlot, { unit: 'bannerWide' });
    await settle();

    const ins = document.querySelector<HTMLElement>('ins.adsbygoogle');
    ins!.dataset.adStatus = 'unfilled';
    await settle();
    expect(document.querySelector('[data-testid="ad-slot"]')?.classList).toContain('hidden');
  });

  it('labels a unit sitting among content', async () => {
    await render(AdSlot, { unit: 'bannerWide', labeled: true });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')?.textContent?.trim()).toBe(
      'Advertisement',
    );
  });
});
