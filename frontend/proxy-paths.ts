// The shared-origin routing contract, mirrored by deploy/Caddyfile in
// production: keep both in sync when a prefix changes.

// Paths Axum owns outright: every method proxies to the API.
export const axumPrefixes = [
	'/api',
	'/eve',
	'/twitch',
	'/discord',
	'/patreon',
	'/og',
	'/img',
	'/display',
	'/logout',
	'/auth',
	'/sitemap.xml',
];

// Axum owns the WebSocket endpoint; it must never reach the node server.
export const axumWebsocketPrefix = '/ws';

// Prefixes shared between SvelteKit pages (GET/HEAD) and Axum actions
// (every other method). No SvelteKit form actions for these paths: all
// mutations call Axum via fetch.
export const sharedPrefixes = [
	'/modules',
	'/collections',
	'/collection-modules',
	'/collection-locations',
	'/collection-notes',
	'/location-collections',
	'/characters',
	'/personal',
	'/estimate',
	'/public-assets',
	'/bookmarks',
	'/offers',
	'/messages',
	'/settings',
	'/workbench',
	'/workbench-modules',
	'/workbench-collections',
	'/notes',
	'/raffle',
	'/raffles',
	'/blocked-users',
	'/advertisements',
	'/gear-items',
	'/moderator',
	'/historic-contracts',
	'/module-pricing',
	'/ui',
];
