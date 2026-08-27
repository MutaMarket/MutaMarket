import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';

export interface AdminAdvertisement {
	id: number;
	name: string;
	description: string | null;
	image_url: string | null;
	link: string | null;
	size: string;
	active: boolean;
	priority: number;
	starts_at: string | null;
	expires_at: string | null;
	status: 'live' | 'scheduled' | 'expired' | 'inactive';
}

// Guests hit the API 401 -> login; non-admins the 403 error page.
export const load: PageServerLoad = async ({ fetch }) => {
	const advertisements = await apiGet<AdminAdvertisement[]>(fetch, '/api/admin/advertisements');
	return { advertisements };
};
