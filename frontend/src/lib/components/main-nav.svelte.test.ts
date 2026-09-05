import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The nav highlights the current path, so it needs one.
vi.mock('$app/state', () => ({
  page: { url: new URL('https://mutamarket.com/') },
}));

const MainNav = (await import('./main-nav.svelte')).default;

import { t } from '$lib/i18n.svelte';
import type { NavState } from '$lib/types';

function nav(overrides: Partial<NavState> = {}): NavState {
  return {
    user: {
      name: 'Wolfgang Bunwoll',
      active_character_id: 42,
      is_admin: false,
      has_premium: false,
    },
    characters: [
      {
        id: 42,
        name: 'Wolfgang Bunwoll',
        corporation_id: null,
        has_asset_token: false,
        active: true,
        granted_scopes: [],
        scope_warnings_muted: false,
      },
    ],
    raffle: null,
    scope_catalogue: [],
    ...overrides,
  } as NavState;
}

/** Every href the nav renders, menus included. The More menu is a
 * CSS-only disclosure whose panel is always in the DOM, so its links are
 * present without opening it. */
function links(container: HTMLElement): string[] {
  return [...container.querySelectorAll('a')].map((anchor) => anchor.getAttribute('href') ?? '');
}

describe('main-nav', () => {
  it('collapses into a drawer below the desktop breakpoint', async () => {
    // The desktop link row only exists from xl up; the hamburger opens
    // the same destinations, More groups included, in a left sheet.
    const screen = await render(MainNav, { nav: nav() });
    expect(screen.container.querySelector('nav')?.className).toContain('hidden');

    await screen.getByRole('button', { name: t('nav.mobile.openMenu') }).click();
    await expect.element(screen.getByRole('dialog')).toBeInTheDocument();

    const drawer = links(screen.baseElement as HTMLElement);
    for (const href of ['/', '/sell/modules', '/offers', '/calculator', '/personal/contracts']) {
      expect(drawer).toContain(href);
    }
    await screen.getByRole('dialog').getByRole('link', { name: 'Calculator' }).click();
    await expect.element(screen.getByRole('dialog')).not.toBeInTheDocument();
  });

  it('names the logo link and labels the navigation and More menu', async () => {
    // The logo is an icon-only link; the More disclosure is a labelled
    // button, and its panel renders in the nav landmark.
    const screen = await render(MainNav, { nav: nav() });
    await expect
      .element(screen.getByRole('link', { name: t('nav.logo.home') }))
      .toHaveAttribute('href', '/');

    const more = [...screen.container.querySelectorAll('button')].find((button) =>
      button.textContent?.includes(t('nav.desktop.more')),
    );
    expect(more?.getAttribute('aria-haspopup')).toBe('true');
    expect(screen.container.querySelector('nav')?.getAttribute('aria-label')).toBe(
      t('nav.ariaLabel'),
    );
  });

  it('offers a signed-in account its contracts', async () => {
    // The page is reachable only from here; without the entry it has
    // no route into it from the UI at all.
    const { container } = await render(MainNav, { nav: nav() });

    expect(links(container)).toContain('/personal/contracts');
    expect(container.textContent).toContain('My contracts');
  });

  it('shows a guest no account entries', async () => {
    const { container } = await render(MainNav, { nav: null });

    expect(links(container)).not.toContain('/personal/contracts');
  });

  it('starts EVE SSO from the guest character button', async () => {
    const { container } = await render(MainNav, { nav: null });
    const login = container.querySelector<HTMLAnchorElement>('a[href="/eve"]');

    expect(login?.getAttribute('rel')).toBe('external');
    expect(login?.textContent).toContain(t('nav.auth.login'));
    expect(links(container)).not.toContain('/login');
  });

  it('keeps the account entries in the legacy order', async () => {
    const { container } = await render(MainNav, { nav: nav() });
    const account = links(container).filter(
      (href) => href.startsWith('/characters/') || href === '/personal/contracts',
    );

    expect(account[0]).toMatch(/^\/characters\//);
    expect(account[1]).toBe('/personal/contracts');
  });
});
