import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';

vi.mock('$app/state', () => ({
  page: {
    data: { nav: null, sidebar: null },
    url: new URL('https://mutamarket.com/'),
  },
}));
vi.mock('$app/env/public', () => ({
  PUBLIC_PATREON_LINK: 'https://www.patreon.com/join/x',
  PUBLIC_KOFI_LINK: '',
  PUBLIC_MARKEEDRAGON_URL: 'https://store.markeedragon.com/',
  PUBLIC_MARKEEDRAGON_CODE: 'mutamarket',
  PUBLIC_ADSENSE_CLIENT_ID: '',
}));
vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
  invalidate: vi.fn(),
  refreshAll: vi.fn(),
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

import { page as appPage } from '$app/state';
import { refreshAll } from '$app/navigation';
import { sidebarData, type SidebarPayload } from '#lib/sidebar.ts';
import type { NavState } from '#lib/types.ts';

function payload(overrides: Partial<SidebarPayload> = {}): SidebarPayload {
  return {
    bookmarks: null,
    advertisements: [],
    eve_referral_link: null,
    donations: {
      top_donors: [],
      recent_donations: [],
    } as unknown as SidebarPayload['donations'],
    discord_invites: [],
    premium_character: 'MutaMate',
    premium_cost: 100_000_000,
    premium_yearly_cost: 1_000_000_000,
    premium_ad_free_cost: 300_000_000,
    premium_ad_free_yearly_cost: 3_000_000_000,
    promotion_daily_cost: 25_000_000,
    ...overrides,
  };
}

// The sidebar is `hidden` until Tailwind's xl breakpoint (1280px), so at the
// default viewport it renders to nothing at all.
const DESKTOP_VIEWPORT = { width: 1440, height: 900 };

beforeAll(() => page.viewport(DESKTOP_VIEWPORT.width, DESKTOP_VIEWPORT.height));

afterEach(() => {
  sidebarData.set(null);
  appPage.data.nav = null;
});

const ADS: SidebarPayload['advertisements'] = [
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
];
describe('the ad carousel', () => {
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
      }),
    );
    const screen = await render(Sidebar);
    await expect.element(screen.getByAltText('PLEX sale')).toBeInTheDocument();

    const carousels = screen.baseElement.querySelectorAll('[aria-roledescription="carousel"]');
    expect(carousels).toHaveLength(1);
    const adSlides = carousels[0].querySelectorAll('[aria-roledescription="slide"]');
    expect(adSlides).toHaveLength(2);
    // The MarkeeDragon creative carries the coupon section as a second link.
    expect(carousels[0].querySelectorAll('a[href*="markeedragon"]')).toHaveLength(2);
  });

  it('renders no carousel without creatives', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Premium')).toBeInTheDocument();
    expect(screen.baseElement.querySelector('[aria-roledescription="carousel"]')).toBeNull();
  });
});

const REFERRAL = 'https://www.eveonline.com/signup?invc=abc';

describe('the EVE referral banner', () => {
  it('offers the referral link', async () => {
    sidebarData.set(payload({ eve_referral_link: REFERRAL }));
    const screen = await render(Sidebar);
    const banner = screen.getByTestId('eve-referral');
    await expect.element(banner).toHaveAttribute('href', REFERRAL);
    await expect.element(banner).toHaveAttribute('target', '_blank');
    await expect.element(banner).toHaveAttribute('rel', 'sponsored noopener');
    expect(banner.element().textContent).toContain('Need a new alt?');
    expect(banner.element().textContent).toContain('1,000,000');
    expect(banner.element().textContent).toContain('Free skill points');
    expect(banner.element().textContent).toContain('Create account');
  });

  it('stays hidden without a link', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.baseElement.querySelector('[data-testid="eve-referral"]')).toBeNull();
  });

  it('reaches logged-in players too, who are the ones making alts', async () => {
    appPage.data.nav = { user: { ad_free: false }, characters: [] } as unknown as NavState;
    sidebarData.set(payload({ eve_referral_link: REFERRAL }));
    const screen = await render(Sidebar);
    await expect.element(screen.getByTestId('eve-referral')).toHaveAttribute('href', REFERRAL);
  });

  it('stays hidden with ad-free premium', async () => {
    appPage.data.nav = { user: { ad_free: true }, characters: [] } as unknown as NavState;
    sidebarData.set(payload({ eve_referral_link: REFERRAL }));
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.baseElement.querySelector('[data-testid="eve-referral"]')).toBeNull();
  });
});

describe('the premium card', () => {
  it('links its header to the premium page, like the legacy Details link', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    const link = screen.getByRole('link', { name: 'Details' });
    await expect.element(link).toHaveAttribute('href', '/premium');
  });
});

describe('ad-free premium', () => {
  it('hides every ad, the store campaigns included', async () => {
    appPage.data.nav = { user: { ad_free: true }, characters: [] } as unknown as NavState;
    sidebarData.set(payload({ advertisements: ADS }));
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Details')).toBeInTheDocument();

    // No carousel at all, no Patreon card.
    expect(screen.baseElement.querySelector('[aria-roledescription="carousel"]')).toBeNull();
    expect(screen.baseElement.querySelector('img[alt="PLEX sale"]')).toBeNull();
    expect(screen.baseElement.querySelector('img[alt="Omega"]')).toBeNull();
    expect(screen.baseElement.querySelector('a[href*="markeedragon"]')).toBeNull();
    expect(screen.baseElement.querySelector('a[href*="patreon"]')).toBeNull();
  });

  it('shows everything without it', async () => {
    appPage.data.nav = { user: { ad_free: false }, characters: [] } as unknown as NavState;
    sidebarData.set(payload({ advertisements: ADS }));
    const screen = await render(Sidebar);
    await expect.element(screen.getByAltText('Omega')).toBeInTheDocument();
    expect(screen.baseElement.querySelectorAll('[aria-roledescription="carousel"]')).toHaveLength(
      1,
    );
    expect(screen.baseElement.querySelector('a[href*="patreon"]')).not.toBeNull();
  });
});

const DAY = 86_400_000;

type NavCharacter = NavState['characters'][number];

function character(overrides: Partial<NavCharacter> = {}): NavCharacter {
  return {
    id: 42,
    name: 'Wolfgang Bunwoll',
    corporation_id: null,
    has_asset_token: false,
    active: true,
    granted_scopes: [],
    scope_warnings_muted: false,
    premium_paid_until: null,
    ...overrides,
  };
}

function navWith(
  characters: NavCharacter[],
  accountAdFree = false,
  lockedUntil: string | null = null,
): NavState {
  return {
    user: {
      ad_free: false,
      has_premium: characters.some((entry) => entry.premium_paid_until !== null),
      premium_ad_free: accountAdFree,
      premium_plan_locked_until: lockedUntil,
    },
    characters,
  } as unknown as NavState;
}

function inDays(days: number): string {
  return new Date(Date.now() + days * DAY + 3_600_000).toISOString();
}

describe('the premium card prices', () => {
  it('shows the plain prices to guests', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Monthly')).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('100 million ISK');
    expect(screen.baseElement.textContent).toContain('1 billion ISK');
    expect(screen.baseElement.textContent).not.toContain('300 million');
  });

  it("follows the account's ad-free plan", async () => {
    appPage.data.nav = navWith([character()], true);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Monthly')).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('300 million ISK');
    expect(screen.baseElement.textContent).toContain('3 billion ISK');
    expect(screen.baseElement.textContent).not.toContain('100 million');
  });
});

describe('the premium status', () => {
  it('is absent for guests', async () => {
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.baseElement.querySelector('[data-testid="premium-status"]')).toBeNull();
  });

  it('shows the plan, the days left and what a switch leaves', async () => {
    appPage.data.nav = navWith([character({ premium_paid_until: inDays(30) })]);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    const status = screen.getByTestId('premium-status');
    await expect.element(status).toBeInTheDocument();
    expect(status.element().textContent).toContain('Wolfgang Bunwoll');
    expect(status.element().textContent).toContain('30 days left');
    expect(status.element().textContent).toContain(
      'Switching to Premium + Ad-free leaves 10 days.',
    );
    await expect
      .element(screen.getByRole('radio', { name: 'Premium' }))
      .toHaveAttribute('aria-checked', 'true');
    await expect
      .element(screen.getByRole('radio', { name: '+ Ad-free' }))
      .toHaveAttribute('aria-checked', 'false');
    // With live premium the upsell gives way to the status.
    expect(screen.baseElement.textContent).not.toContain('Unlock historic sales');
  });

  it('shows the linked character holding the most premium', async () => {
    appPage.data.nav = navWith([
      character({ premium_paid_until: inDays(5) }),
      character({ id: 43, name: 'Alt Hauler', active: false, premium_paid_until: inDays(40) }),
      character({ id: 44, name: 'No Time', active: false }),
    ]);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    const status = screen.getByTestId('premium-status');
    await expect.element(status).toBeInTheDocument();
    expect(status.element().textContent).toContain('Alt Hauler');
    expect(status.element().textContent).toContain('40 days left');
    expect(status.element().textContent).not.toContain('Wolfgang Bunwoll');
  });

  it('says so without premium and keeps the upsell', async () => {
    appPage.data.nav = navWith([character()], true);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    const status = screen.getByTestId('premium-status');
    await expect.element(status).toBeInTheDocument();
    expect(status.element().textContent).toContain('No premium');
    expect(status.element().textContent).not.toContain('Switching to');
    await expect
      .element(screen.getByRole('radio', { name: '+ Ad-free' }))
      .toHaveAttribute('aria-checked', 'true');
    expect(screen.baseElement.textContent).toContain('Unlock historic sales');
  });

  it('asks before going ad-free, then switches the account', async () => {
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ ad_free: true, locked_until: null, characters: [] }), {
        status: 200,
      }),
    );
    appPage.data.nav = navWith([
      character({ premium_paid_until: inDays(9) }),
      character({ id: 43, name: 'Alt Hauler', active: false, premium_paid_until: inDays(30) }),
    ]);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await screen.getByRole('radio', { name: '+ Ad-free' }).click();

    // The warning shows the longest time before and after, the prices
    // side by side and the lock; nothing is sent before it is confirmed.
    await expect.element(screen.getByRole('dialog')).toBeInTheDocument();
    const text = (id: string) =>
      (screen.getByTestId(id).element().textContent ?? '').replace(/\s+/g, ' ').trim();
    expect(text('ad-free-time')).toBe('Premium 30 days Premium + Ad-free 10 days');
    expect(text('ad-free-price')).toBe(
      'PremiumPremium + Ad-free1 month100 million ISK300 million ISK' +
        '12 months1 billion ISK3 billion ISK',
    );
    expect(text('ad-free-lock')).toMatch(
      /^Locked for 30 days\. You can switch back to premium on \d{4}-\d{2}-\d{2} at the earliest\./,
    );
    expect(fetchMock).not.toHaveBeenCalled();

    await screen.getByRole('button', { name: 'Go ad-free' }).click();
    await vi.waitFor(() => expect(refreshAll).toHaveBeenCalled());
    expect(fetchMock).toHaveBeenCalledWith('/premium/plan', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ ad_free: true }),
    });

    // The chosen plan does nothing.
    fetchMock.mockClear();
    await screen.getByRole('radio', { name: 'Premium' }).click();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('cancelling the warning keeps the plan', async () => {
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockClear();
    appPage.data.nav = navWith([character({ premium_paid_until: inDays(9) })]);
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await screen.getByRole('radio', { name: '+ Ad-free' }).click();
    await screen.getByRole('button', { name: 'Cancel' }).click();
    await expect.element(screen.getByRole('dialog')).not.toBeInTheDocument();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('locks the way back while the 30 days run', async () => {
    appPage.data.nav = navWith(
      [character({ premium_paid_until: inDays(9) })],
      true,
      '2026-12-01T12:00:00Z',
    );
    sidebarData.set(payload());
    const screen = await render(Sidebar);
    await expect.element(screen.getByRole('radio', { name: 'Premium' })).toBeDisabled();
    expect(screen.getByTestId('premium-status').element().textContent).toContain(
      'Ad-free until at least 2026-12-01.',
    );
  });
});
