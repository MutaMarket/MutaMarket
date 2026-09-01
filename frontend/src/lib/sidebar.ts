// The sidebar state and actions: one /api/sidebar payload (the legacy
// shared Inertia props) plus the bookmark mutations, and the static
// partner links the legacy read from env-backed config.
import { writable } from 'svelte/store';
import { notifySuccess } from './toast';
import type { BookmarkEntry } from './bookmark-routes';
import type { DonationLists } from './donations';

export interface SidebarAdvertisement {
	id: number;
	name: string;
	description: string | null;
	image_url: string | null;
	link: string | null;
	size: string;
}

export interface SidebarGearItem {
	id: number;
	name: string;
	description: string | null;
	image_url: string | null;
	link: string;
}

export interface DiscordInvite {
	name: string;
	/** null when the invite env var is unset; the card hides. */
	url: string | null;
	image: string | null;
	/** null until the discord-member-counts job stored a count. */
	member_count: number | null;
}

export interface SidebarPayload {
	/** null for guests. */
	bookmarks: BookmarkEntry[] | null;
	advertisements: SidebarAdvertisement[];
	gear_items: SidebarGearItem[];
	/** The legacy shared `donations` prop. */
	donations: DonationLists;
	/** The legacy DiscordInvites shared prop. */
	discord_invites: DiscordInvite[];
	/** The legacy AppData shared props (see $lib/premium). */
	premium_character: string;
	premium_cost: number;
	premium_yearly_cost: number;
}

export const sidebarData = writable<SidebarPayload | null>(null);

export async function refreshSidebar() {
	const response = await fetch('/api/sidebar');
	if (response.ok) {
		sidebarData.set(await response.json());
	}
}

export async function createBookmark(query: string, name: string, typeId: number | null) {
	await fetch('/bookmarks', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ query, name, type_id: typeId }),
		redirect: 'manual',
	});
	notifySuccess('Bookmark created!', 'You successfully created a bookmark');
	await refreshSidebar();
}

export async function renameBookmark(id: number, name: string) {
	await fetch(`/bookmarks/${id}`, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ name }),
		redirect: 'manual',
	});
	notifySuccess('Bookmark updated!', 'You successfully updated a bookmark');
	await refreshSidebar();
}

export async function deleteBookmark(id: number) {
	await fetch(`/bookmarks/${id}`, { method: 'DELETE', redirect: 'manual' });
	notifySuccess('Bookmark deleted!', 'You successfully deleted your bookmark');
	await refreshSidebar();
}

/** Partner links, the legacy services config; empty entries hide. */
export const KOFI_LINK = import.meta.env.PUBLIC_KOFI_LINK ?? 'https://ko-fi.com/nicolaskion';
export const PATREON_LINK = import.meta.env.PUBLIC_PATREON_LINK ?? '';

/** The payload invites worth a card: unconfigured ones (null url,
 * unset backend env) hide, like the other partner links. */
export function visibleDiscordInvites(
	invites: DiscordInvite[],
): (DiscordInvite & { url: string })[] {
	return invites.filter((invite): invite is DiscordInvite & { url: string } => invite.url !== null);
}
