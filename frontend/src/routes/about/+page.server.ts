import { redirect } from '@sveltejs/kit';

// The legacy /about shortcut.
export function load(): never {
	redirect(301, '/documentation/about');
}
