import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import PaginationButtons from './pagination-buttons.svelte';

describe('the pagination buttons', () => {
  it('renders nothing while everything fits on one page', async () => {
    const screen = render(PaginationButtons, {
      meta: { current_page: 1, per_page: 32, total: 10, last_page: 1 },
      onPage: vi.fn(),
    });
    await expect.element(screen.baseElement).toBeInTheDocument();
    expect(screen.baseElement.querySelector('nav')).toBeNull();
  });

  it('lists the pages and reports the one clicked', async () => {
    const onPage = vi.fn();
    const screen = render(PaginationButtons, {
      meta: { current_page: 2, per_page: 32, total: 100, last_page: 4 },
      onPage,
    });
    await expect.element(screen.getByRole('navigation')).toBeInTheDocument();
    const labels = [...screen.baseElement.querySelectorAll('[data-slot="pagination-link"]')].map(
      (link) => link.textContent?.trim(),
    );
    expect(labels).toEqual(['1', '2', '3', '4']);
    await screen.getByText('3', { exact: true }).click();
    expect(onPage).toHaveBeenCalledWith(3);
    await screen.getByLabelText('Go to last page').click();
    expect(onPage).toHaveBeenCalledWith(4);
  });
});
