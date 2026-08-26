import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import type { ProxyOptions } from 'vite';
import { playwright } from '@vitest/browser-playwright';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';

import { axumPrefixes, axumWebsocketPrefix, sharedPrefixes } from './proxy-paths.ts';

// Dev-only shared origin (Caddy plays this role in production): backend
// paths go to Axum, page GETs stay in SvelteKit, cookies never cross an
// origin. Never point the browser at Axum's port directly. Match the
// API's BIND_ADDR with AXUM_URL when the default port is taken (the SSR
// fetch rewrite in hooks.server.ts reads the same variable).
const AXUM_DEV_URL = process.env.AXUM_URL ?? 'http://127.0.0.1:3000';

// Inside the frontend-dev container the source is a macOS bind mount and
// file events don't cross Docker's VM boundary, so hot reload silently
// dies. VITE_POLL_WATCH (set by docker-compose) switches the watcher to
// polling there; native dev keeps event-based watching.
const watch = process.env.VITE_POLL_WATCH ? { usePolling: true, interval: 300 } : undefined;

const proxy: Record<string, ProxyOptions> = {
	[axumWebsocketPrefix]: { target: AXUM_DEV_URL, ws: true }
};
for (const prefix of axumPrefixes) {
	proxy[prefix] = { target: AXUM_DEV_URL };
}
for (const prefix of sharedPrefixes) {
	proxy[prefix] = {
		target: AXUM_DEV_URL,
		bypass: (req) => (req.method === 'GET' || req.method === 'HEAD' ? req.url : null)
	};
}

export default defineConfig({
	server: { proxy, watch },
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) => filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter()
		})
	],
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'client',
					browser: {
						enabled: true,
						provider: playwright(),
						instances: [{ browser: 'chromium', headless: true }]
					},
					include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
					exclude: ['src/lib/server/**']
				}
			},

			{
				extends: './vite.config.ts',
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
