import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import type { OfferThread } from '$lib/types-offers';

const invalidate = vi.fn().mockResolvedValue(undefined);
const pageState = {
  url: new URL('https://mutamarket.com/offers/7'),
  data: {} as Record<string, unknown>,
};

vi.mock('$app/navigation', () => ({ invalidate, invalidateAll: vi.fn(), goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: pageState }));
vi.mock('$lib/asset-import-stream', () => ({ subscribeUserEvent: () => () => {} }));

const ThreadPage = (await import('./+page.svelte')).default;

function thread(): OfferThread {
  return {
    id: 7,
    sender: { id: 1, name: 'Offer Buyer' },
    receiver: { id: 2, name: 'Offer Seller' },
    price: 1_000_000,
    own_character_id: 2,
    left_by_sender: false,
    left_by_receiver: false,
    module: null,
    messages: [
      {
        id: 1,
        sender: { id: 1, name: 'Offer Buyer' },
        content: 'Hello',
        created_at: '2026-09-05T09:00:00Z',
        mine: false,
      },
    ],
  };
}

describe('the offer thread page', () => {
  it('refreshes the navigation unread count once the thread is open', async () => {
    pageState.data = { nav: { unread_offers: 1 } };
    await render(ThreadPage, { props: { data: { offer: thread() } } as never });

    await vi.waitFor(() => expect(invalidate).toHaveBeenCalledWith('app:shared-props'));
  });

  it('leaves the navigation alone when nothing was unread', async () => {
    invalidate.mockClear();
    pageState.data = { nav: { unread_offers: 0 } };
    await render(ThreadPage, { props: { data: { offer: thread() } } as never });

    expect(invalidate).not.toHaveBeenCalled();
  });
});
