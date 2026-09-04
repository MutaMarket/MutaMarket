import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

vi.mock('$app/state', () => ({
  page: { url: new URL('https://mutamarket.com/login'), data: {} },
}));

const LoginPage = (await import('./+page.svelte')).default;

describe('the login page', () => {
  it('offers both EVE sign-in flows as full page loads', async () => {
    // Both /eve links must bypass the SvelteKit router: the proxy hands
    // them to Axum, which redirects to the EVE SSO.
    const screen = await render(LoginPage);
    await expect.element(screen.getByText('Welcome to MutaMarket')).toBeInTheDocument();

    const eve = [...screen.container.querySelectorAll('a[href^="/eve"]')];
    expect(eve.map((anchor) => anchor.getAttribute('href'))).toEqual([
      '/eve',
      '/eve?without_scopes=true',
    ]);
    expect(eve.every((anchor) => anchor.getAttribute('rel') === 'external')).toBe(true);
    // Seven feature headings plus the sign-in card, all one level under
    // the page title so the outline does not skip a level.
    expect(screen.container.querySelectorAll('h2')).toHaveLength(8);
    expect(screen.container.querySelectorAll('h3')).toHaveLength(0);
  });
});
