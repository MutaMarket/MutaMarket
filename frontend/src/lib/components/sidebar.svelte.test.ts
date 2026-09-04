import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

vi.mock('$app/state', () => ({
  page: { data: { nav: null, sidebar: null }, url: new URL('https://mutamarket.com/') },
}));
vi.mock('$env/dynamic/public', () => ({
  env: {
    PUBLIC_PATREON_LINK: 'https://www.patreon.com/join/x',
    PUBLIC_KOFI_LINK: '',
    PUBLIC_MARKEEDRAGON_CODE: 'mutamarket',
  },
}));
vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
  invalidate: vi.fn(),
  invalidateAll: vi.fn(),
  afterNavigate: vi.fn(),
  beforeNavigate: vi.fn(),
  onNavigate: vi.fn(),
}));

// Nothing here should reach the API.
vi.stubGlobal(
  'fetch',
  vi.fn(async () => new Response(null, { status: 503 })),
);

const Sidebar = (await import('./sidebar.svelte')).default;

import { sidebarData, type SidebarPayload } from '$lib/sidebar';

function payload(overrides: Partial<SidebarPayload> = {}): SidebarPayload {
  return {
    bookmarks: null,
    advertisements: [],
    gear_items: [],
    donations: { top_donors: [], recent_donations: [] } as unknown as SidebarPayload['donations'],
    discord_invites: [],
    premium_character: 'MutaMate',
    premium_cost: 100_000_000,
    premium_yearly_cost: 1_000_000_000,
    ...overrides,
  };
}

afterEach(() => sidebarData.set(null));

describe('the ad and gear carousels', () => {
  it('puts every creative on a looping carousel slide', async () => {
    sidebarData.set(
      payload({
        advertisements: [
          {
            id: 1,
            name: 'PLEX sale',
            description: null,
            image_url: '/img/ads/a.png',
            link: 'https://store.markeedragon.com/x',
            size: 'sidebar',
          },
          {
            id: 2,
            name: 'Omega',
            description: null,
            image_url: '/img/ads/b.png',
            link: 'https://example.com',
            size: 'sidebar',
          },
          {
            id: 3,
            name: 'No image',
            description: null,
            image_url: null,
            link: null,
            size: 'sidebar',
          },
        ],
        gear_items: [
          {
            id: 7,
            name: 'HS80',
            description: null,
            image_url: '/img/gear/hs80.png',
            link: 'https://geni.us/hs80',
          },
        ],
      }),
    );
    const screen = await render(Sidebar);
    await expect.element(screen.getByAltText('PLEX sale')).toBeInTheDocument();

    const carousels = screen.baseElement.querySelectorAll('[aria-roledescription="carousel"]');
    expect(carousels).toHaveLength(2);
    const adSlides = carousels[0].querySelectorAll('[aria-roledescription="slide"]');
    expect(adSlides).toHaveLength(2);
    // The MarkeeDragon creative carries the coupon section as a second link.
    expect(carousels[0].querySelectorAll('a[href*="markeedragon"]')).toHaveLength(2);
    expect(carousels[1].querySelectorAll('[aria-roledescription="slide"]')).toHaveLength(1);
    expect(screen.baseElement.textContent).toContain('Affiliate links');
  });

  it('renders neither carousel without creatives', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Premium')).toBeInTheDocument();
    expect(screen.baseElement.querySelector('[aria-roledescription="carousel"]')).toBeNull();
  });
});
