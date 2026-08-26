// The live asset-import feed shared by the personal and sell pages:
// one subscription to the user's channel (the legacy AssetImportUpdated
// event), plus the gate deciding when a running import should refresh
// the visible module list so found modules stream in.
import type { AssetImportView } from './types';

/** Minimum gap between mid-import list refreshes; progress events can
 * arrive far faster than a refetch is worth. */
export const IMPORT_REFRESH_MIN_MS = 3000;

/**
 * Classifies each import update: `'stream'` for a throttled mid-import
 * refresh, `'completed'` exactly when the import finishes (the caller
 * does its full reload there), `null` otherwise.
 */
export function importRefreshGate(
	now: () => number = Date.now
): (view: AssetImportView | null) => 'stream' | 'completed' | null {
	let lastRefresh = -Infinity;
	return (view) => {
		if (view === null || view.status === 'failed') return null;
		if (view.status === 'completed') return 'completed';
		if (now() - lastRefresh < IMPORT_REFRESH_MIN_MS) return null;
		lastRefresh = now();
		return 'stream';
	};
}

/**
 * Opens the user-channel websocket and calls `onUpdate` for every
 * AssetImportUpdated event. Returns the cleanup for `$effect`.
 */
export function subscribeAssetImport(
	userId: number,
	onUpdate: (view: AssetImportView | null) => void
): () => void {
	const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
	const socket = new WebSocket(`${scheme}://${location.host}/ws`);
	const channel = `Users.${userId}`;

	socket.onmessage = (event) => {
		try {
			const envelope = JSON.parse(event.data as string) as {
				channel: string;
				event: string;
				data: AssetImportView | null;
			};
			if (envelope.channel === channel && envelope.event === 'AssetImportUpdated') {
				onUpdate(envelope.data);
			}
		} catch {
			// Not an envelope; ignore.
		}
	};

	return () => socket.close();
}
