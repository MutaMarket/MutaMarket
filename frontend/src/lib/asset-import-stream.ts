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
 * Opens the session's event socket and calls `onData` for every pushed
 * event with the given name. The socket is session-authenticated and
 * only ever carries the user's own channel, so no channel filter is
 * needed. Returns the cleanup for `$effect`.
 */
export function subscribeUserEvent<T>(name: string, onData: (data: T) => void): () => void {
	const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
	const socket = new WebSocket(`${scheme}://${location.host}/ws`);

	socket.onmessage = (event) => {
		try {
			const envelope = JSON.parse(event.data as string) as {
				channel: string;
				event: string;
				data: T;
			};
			if (envelope.event === name) {
				onData(envelope.data);
			}
		} catch {
			// Not an envelope; ignore.
		}
	};

	return () => socket.close();
}

/**
 * The AssetImportUpdated subscription used by the personal and sell
 * pages. The userId parameter is kept for call-site clarity; the socket
 * itself is already scoped to the session's user.
 */
export function subscribeAssetImport(
	_userId: number,
	onUpdate: (view: AssetImportView | null) => void
): () => void {
	return subscribeUserEvent('AssetImportUpdated', onUpdate);
}
