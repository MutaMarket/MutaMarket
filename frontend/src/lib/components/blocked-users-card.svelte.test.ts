import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import BlockedUsersCard from './blocked-users-card.svelte';

describe('the blocked users card', () => {
  it('lists each blocked account and reports the one to unblock', async () => {
    const onUnblock = vi.fn();
    const screen = render(BlockedUsersCard, {
      blocked: [
        {
          user_id: 7,
          name: 'Offer Buyer',
          character_id: 90_000_001,
          blocked_at: '2026-09-01T10:00:00Z',
        },
        {
          user_id: 8,
          name: 'Deleted User',
          character_id: null,
          blocked_at: '2026-08-01T10:00:00Z',
        },
      ],
      onUnblock,
    });
    await expect.element(screen.getByText('Offer Buyer')).toBeInTheDocument();
    expect(screen.baseElement.querySelectorAll('img')).toHaveLength(1);
    expect(screen.baseElement.textContent).toContain('Blocked on 1 Sept 2026');
    await screen.getByRole('button', { name: 'Unblock' }).first().click();
    expect(onUnblock).toHaveBeenCalledWith(7);
  });

  it('explains where blocks come from when there are none', async () => {
    const screen = render(BlockedUsersCard, { blocked: [], onUnblock: vi.fn() });
    await expect.element(screen.getByText('Blocked users')).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('Block a user from an offer thread');
    expect(screen.baseElement.querySelector('button')).toBeNull();
  });
});
