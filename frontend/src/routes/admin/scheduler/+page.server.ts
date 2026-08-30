// The scheduler became the console's jobs board; keep the old bookmark
// working.
import { redirect } from '@sveltejs/kit';

export function load(): never {
	redirect(301, '/admin/jobs');
}
