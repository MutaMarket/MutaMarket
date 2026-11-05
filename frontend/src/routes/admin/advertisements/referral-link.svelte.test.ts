import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

vi.mock('$app/navigation', () => ({ refreshAll: vi.fn() }));

const AdvertisementsPage = (await import('./+page.svelte')).default;

const LINK = 'https://www.eveonline.com/signup?invc=abc';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('the referral link field', () => {
  it('starts from the stored link and saves what the admin typed', async () => {
    const fetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(JSON.stringify({ link: LINK }), { status: 200 }));
    const screen = await render(AdvertisementsPage, {
      data: { advertisements: [], referralLink: 'https://old.example' },
    } as never);

    const input = screen.getByLabelText('EVE referral link');
    await expect.element(input).toHaveValue('https://old.example');
    await input.fill(LINK);
    await screen.getByRole('button', { name: 'Save' }).click();

    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    expect(fetch).toHaveBeenCalledWith('/api/admin/referral-link', {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ link: LINK }),
    });
  });

  it('starts empty without a stored link', async () => {
    const screen = await render(AdvertisementsPage, {
      data: { advertisements: [], referralLink: null },
    } as never);
    await expect.element(screen.getByLabelText('EVE referral link')).toHaveValue('');
  });
});
