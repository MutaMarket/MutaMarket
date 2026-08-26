// The make-offer flow, the legacy useMakeOffer composable: one dialog
// mounted in the layout, opened from any card's public-asset row, plus
// the signed-in user's sent-offer set for the Go to offer swap.
import { writable } from 'svelte/store';
import type { ModuleDetail } from './types';
import type { SentOffer } from './types-offers';

/** The module the dialog is open for; null keeps it closed. */
export const offerModule = writable<ModuleDetail | null>(null);

/** module id → offer id of the user's active sent offers. */
export const sentOffers = writable<Map<number, number>>(new Map());

export function openMakeOffer(module: ModuleDetail) {
	offerModule.set(module);
}

export function closeMakeOffer() {
	offerModule.set(null);
}

export async function refreshSentOffers() {
	const response = await fetch('/api/offers/sent');
	if (response.ok) {
		const entries = (await response.json()) as SentOffer[];
		sentOffers.set(new Map(entries.map((entry) => [entry.module_id, entry.id])));
	}
}

/** The legacy offers.create.defaultMessage, with the price filled in
 * (mirror of the backend's fallback for empty messages). */
export function defaultOfferMessage(price: number | null): string {
	const isk = price !== null && price > 0 ? Math.round(price).toLocaleString('en-US') : '…';
	return `Hey, I can offer you ${isk} ISK for it. Let me know if you're interested!`;
}
