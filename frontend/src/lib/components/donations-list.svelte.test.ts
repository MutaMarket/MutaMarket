import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import DonationsList from './donations-list.svelte';

function donation(id: number, count: number) {
	return {
		id,
		amount: 250_000_000,
		donation_count: count,
		character: {
			id: 90_000_000 + id,
			name: `Donor ${id}`,
			slug: `donor-${id}`,
			description: null,
			has_premium: false,
			corporation_id: null,
		},
	};
}

describe('donations-list', () => {
	// A tooltip without a Tooltip.Provider above it throws during SSR, and
	// this component renders on /donations and in the sidebar with nothing
	// providing one. It has to bring its own.
	it('renders a repeat donor, whose row carries a tooltip', () => {
		const { container } = render(DonationsList, {
			donations: [donation(1, 5)],
			showRank: true,
		});

		expect(container.textContent).toContain('Donor 1');
	});

	it('renders a one-off donor, whose row has no tooltip', () => {
		const { container } = render(DonationsList, {
			donations: [donation(2, 1)],
			showRank: false,
		});

		expect(container.textContent).toContain('Donor 2');
	});

	it('renders an empty list without complaint', () => {
		const { container } = render(DonationsList, { donations: [], showRank: true });

		expect(container.querySelectorAll('img')).toHaveLength(0);
	});
});
