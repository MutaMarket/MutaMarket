import { error, redirect } from '@sveltejs/kit';

/**
 * Loads a JSON payload from the Axum API for an SSR load function,
 * translating the API's contract into SvelteKit control flow: 401 sends
 * the guest to the login page, 4xx/503 statuses become error pages
 * carrying the API's message, anything else unexpected a plain 500.
 */
export async function apiGet<T>(fetch: typeof globalThis.fetch, path: string): Promise<T> {
	const response = await fetch(path);

	if (response.status === 401) {
		redirect(303, '/login');
	}
	if (!response.ok) {
		const body: { message?: string } = await response.json().catch(() => ({}));
		error(response.status, body.message ?? 'The server is unavailable right now.');
	}

	return response.json();
}
