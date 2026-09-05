import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const invalidateAll = vi.fn().mockResolvedValue(undefined);
vi.mock('$app/navigation', () => ({ invalidateAll }));

// A mutable page state each test shapes before rendering.
const pageState = {
  status: 404,
  error: { message: 'Not Found' } as { message: string } | null,
  url: new URL('https://mutamarket.com/modules/does-not-exist'),
  data: {} as Record<string, unknown>,
};
vi.mock('$app/state', () => ({ page: pageState }));

const ErrorPage = (await import('./+error.svelte')).default;

function show(status: number, message: string, data: Record<string, unknown> = {}) {
  pageState.status = status;
  pageState.error = { message };
  pageState.data = data;
  return render(ErrorPage);
}

function text(container: HTMLElement): string {
  return (container.textContent ?? '').replace(/\s+/g, ' ').trim();
}

function links(container: HTMLElement): string[] {
  return [...container.querySelectorAll<HTMLAnchorElement>('nav a')].map((a) =>
    a.getAttribute('href'),
  ) as string[];
}

afterEach(() => {
  invalidateAll.mockClear();
  vi.useRealTimers();
});

describe('the error page', () => {
  it('names a missing page with its path and the legacy links', async () => {
    const { container } = await show(404, 'Not Found');

    expect(text(container)).toContain('404 Page not found');
    expect(container.querySelector('p strong')?.textContent).toBe('/modules/does-not-exist');
    expect(text(container)).toContain(
      'The page with the URL /modules/does-not-exist does not exist.',
    );
    expect(links(container)).toEqual(['/', '/characters', '/collections']);
    expect(container.querySelector('button')?.textContent).toContain('Go back');
    expect(text(container)).not.toContain('Try again');
  });

  it('adds the settings link for a signed-in user', async () => {
    const { container } = await show(403, 'Forbidden', { nav: { user: { name: 'Pilot' } } });

    expect(text(container)).toContain('403 Unauthorized');
    expect(text(container)).toContain('bribe the gatekeeper');
    expect(links(container)).toEqual(['/', '/characters', '/collections', '/settings']);
  });

  it('quotes the API message on a bad request', async () => {
    const { container } = await show(400, 'The query is malformed.');

    expect(text(container)).toContain('invalid syntax: The query is malformed.');
  });

  it('offers a retry on a server error that reloads the page data', async () => {
    const screen = await show(500, 'Internal Error');

    expect(text(screen.container)).toContain('500 Something went wrong');
    await screen.getByRole('button', { name: 'Try again' }).click();
    expect(invalidateAll).toHaveBeenCalledTimes(1);
  });

  it('falls back to the message and a retry for an unlisted status', async () => {
    const { container } = await show(429, 'Too many requests.');

    expect(text(container)).toContain('429 Too many requests.');
    expect(text(container)).toContain('The request could not be completed.');
    expect(text(container)).toContain('Try again');
  });

  it('polls during maintenance and links the Discord invite', async () => {
    vi.useFakeTimers();
    const { container } = await show(503, 'Database unavailable.', {
      sidebar: {
        discord_invites: [
          { name: 'Closed', url: null },
          { name: 'MutaMarket', url: 'https://discord.gg/mutamarket' },
        ],
      },
    });

    expect(text(container)).toContain('503');
    expect(text(container)).toContain('Maintenance in progress');
    expect(container.querySelector('a[href="https://discord.gg/mutamarket"]')).not.toBeNull();
    expect(container.querySelector('nav')).toBeNull();

    await vi.advanceTimersByTimeAsync(5000);
    expect(invalidateAll).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(5000);
    expect(invalidateAll).toHaveBeenCalledTimes(2);
  });
});
