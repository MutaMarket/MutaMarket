import { redirect } from '@sveltejs/kit';

// The legacy /help shortcut.
export function load(): never {
  redirect(301, '/documentation');
}
