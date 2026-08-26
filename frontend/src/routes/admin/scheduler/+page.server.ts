// The dashboard grew beyond the scheduler and moved to /admin; keep the
// old bookmark working.
import { redirect } from '@sveltejs/kit';

export function load(): never {
	redirect(301, '/admin');
}
