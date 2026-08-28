// The donation lists shared by the sidebar's top-donors card and the
// /donations page (the legacy shared `donations` Inertia prop with its
// DonationResource entries), plus the display helpers of the legacy
// DonationRankBadge / RepeatDonorBadge components.

import type { CharacterRef } from './types';

export interface DonationEntry {
	id: number;
	amount: number;
	/** Present on the latest and recent lists; the all-time aggregate
	 * carries no date (the legacy `whenHas`). */
	date?: string;
	character: CharacterRef;
	donation_count?: number;
}

export interface DonationLists {
	latest: DonationEntry[];
	highest: DonationEntry[];
	recent: DonationEntry[];
}

/** The legacy DonationsPage fallback when the shared prop is absent. */
export const EMPTY_DONATIONS: DonationLists = { latest: [], highest: [], recent: [] };

/** The legacy DonationRankBadge gradients; ranks past the podium render
 * as plain muted numbers (null). */
export function rankGradient(rank: number): string | null {
	switch (rank) {
		case 1:
			return 'from-yellow-400 to-amber-600';
		case 2:
			return 'from-slate-300 to-slate-500';
		case 3:
			return 'from-amber-600 to-orange-700';
		default:
			return null;
	}
}

/** The legacy RepeatDonorBadge tooltip plural:
 * "{count} donation | {count} donations". */
export function donationCountLabel(count: number): string {
	return count === 1 ? '1 donation' : `${count} donations`;
}

/** The badge only shows for repeat donors, the legacy `count > 1`. */
export function isRepeatDonor(count: number | undefined): count is number {
	return count !== undefined && count > 1;
}
