// The admin raffle page data (GET /api/admin/raffles) and the status
// labels the legacy RafflePage.vue renders.
import { STATUS_ACTIVE, STATUS_CLAIMED, STATUS_PAID_OUT, STATUS_PENDING } from '$lib/raffle-status';

export interface AdminRaffleItem {
	id: number;
	name: string | null;
	description: string | null;
	code: string;
	status: number;
	type: { id: number; name: string | null } | null;
	winner: { id: number; name: string | null; character_id: number | null } | null;
	expires_at: string | null;
	created_at: string | null;
}

export interface AdminRafflesData {
	raffle_items: AdminRaffleItem[];
	types: { id: number; name: string }[];
	type_search: string;
}

/** The legacy status labels of the admin list. */
export function statusLabel(status: number): string {
	switch (status) {
		case STATUS_PAID_OUT:
			return 'Paid out';
		case STATUS_PENDING:
			return 'Pending';
		case STATUS_ACTIVE:
			return 'Active';
		case STATUS_CLAIMED:
			return 'Claimed';
		default:
			return 'Unknown';
	}
}

/** Drawn prizes and claimed ones are the ones with a winner to show. */
export function hasWinner(item: AdminRaffleItem): boolean {
	return item.winner !== null && item.status !== STATUS_PENDING;
}

/** The pool counts the legacy page shows above the list. */
export function poolCounts(items: AdminRaffleItem[]): {
	pending: number;
	active: number;
	claimed: number;
} {
	return {
		pending: items.filter((item) => item.status === STATUS_PENDING).length,
		active: items.filter((item) => item.status === STATUS_ACTIVE).length,
		claimed: items.filter(
			(item) => item.status === STATUS_CLAIMED || item.status === STATUS_PAID_OUT,
		).length,
	};
}
