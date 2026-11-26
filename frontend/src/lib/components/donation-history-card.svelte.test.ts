import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import DonationHistoryCard from './donation-history-card.svelte';

// Intl puts a no-break space between the currency code and the number.
function text(element: Element | null): string {
  return (element?.textContent ?? '').replaceAll(' ', ' ').trim();
}

describe('the donation history card', () => {
  it('lists each donation with its character, UTC day and amount, then the total', async () => {
    const screen = await render(DonationHistoryCard, {
      donations: [
        {
          id: 2,
          amount: 1_000_000_000,
          // Late UTC evening: local time must not push it to the next day.
          date: '2026-09-15T23:30:00Z',
          character: { id: 90_000_002, name: 'Settex Alt' },
        },
        {
          id: 1,
          amount: 100_000_000,
          date: '2026-08-01T12:00:00Z',
          character: { id: 90_000_001, name: 'Settex Prime' },
        },
      ],
    });
    await expect.element(screen.getByText('Your donations')).toBeInTheDocument();
    const rows = [...screen.baseElement.querySelectorAll('li')].map((row) => [
      text(row.querySelector('span')),
      text(row.querySelector('time')),
      text(row.querySelector('li > span')),
    ]);
    expect(rows).toEqual([
      ['Settex Alt', '15 Sept 2026', 'ISK 1,000,000,000'],
      ['Settex Prime', '1 Aug 2026', 'ISK 100,000,000'],
    ]);
    expect(text(screen.baseElement.querySelector('[data-testid="donation-total"]'))).toBe(
      'ISK 1,100,000,000',
    );
    expect(screen.baseElement.textContent).toContain(
      'ISK donations received from the characters on this account.',
    );
  });

  it('says so when no character has donated, and still links to the how-to', async () => {
    const screen = await render(DonationHistoryCard, { donations: [] });
    await expect.element(screen.getByText('Your donations')).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('No donations from your characters yet.');
    expect(screen.baseElement.querySelector('ul')).toBeNull();
    expect(screen.baseElement.querySelector('[data-testid="donation-total"]')).toBeNull();
    expect(screen.getByRole('link', { name: 'How to donate' }).element().getAttribute('href')).toBe(
      '/donations',
    );
  });
});
