import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const state = { page: { data: { nav: null as unknown }, url: new URL('https://mutamarket.com/') } };
vi.mock('$app/state', () => state);
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
    await render(AdSlot, { slot: '123', minHeight: 250, fullWidthResponsive: true });
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

  it('renders a fixed unit without the responsive attributes', async () => {
    await render(AdSlot, { slot: '456', width: 300, height: 600 });
    await settle();

    const ins = document.querySelector<HTMLElement>('ins.adsbygoogle');
    expect(ins?.style.display).toBe('inline-block');
    expect(ins?.style.width).toBe('300px');
    expect(ins?.style.height).toBe('600px');
    expect(ins?.hasAttribute('data-ad-format')).toBe(false);
    expect(ins?.hasAttribute('data-full-width-responsive')).toBe(false);
  });

  it('never requests a unit a breakpoint hides', async () => {
    // The test browser has no Tailwind; stand in for its `hidden`.
    const css = document.createElement('style');
    css.textContent = '.hidden { display: none }';
    document.head.append(css);
    await render(AdSlot, { slot: '456', width: 160, height: 600, class: 'hidden' });
    await settle();

    expect(document.querySelector('ins.adsbygoogle')).not.toBeNull();
    expect(window.adsbygoogle).toBeUndefined();
  });

  it('stays dormant without a unit id', async () => {
    await render(AdSlot, { slot: '' });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')).toBeNull();
  });

  it('shows nothing to premium accounts', async () => {
    state.page.data.nav = { user: { has_premium: true } };
    await render(AdSlot, { slot: '123' });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')).toBeNull();
  });

  it('labels a unit sitting among content', async () => {
    await render(AdSlot, { slot: '123', labeled: true });
    await settle();

    expect(document.querySelector('[data-testid="ad-slot"]')?.textContent?.trim()).toBe(
      'Advertisement',
    );
  });
});
