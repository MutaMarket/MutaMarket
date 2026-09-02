// The props every page carries, the legacy HandleInertiaRequests share
// list: the login state, the sidebar payload, and for a signed-in user
// the workbench and the active sent offers. Loaded on the server so the
// first paint already has the sidebar and the drawer instead of an
// empty frame that fills in after mount.
import type { SidebarPayload } from '$lib/sidebar';
import type { NavState } from '$lib/types';
import type { SentOffer } from '$lib/types-offers';
import type { WorkbenchEntry } from '$lib/workbench';

export interface SharedProps {
  /** null for guests. */
  nav: NavState | null;
  sidebar: SidebarPayload | null;
  /** null for guests. */
  workbench: WorkbenchEntry[] | null;
  /** null for guests. */
  sentOffers: SentOffer[] | null;
}

export async function loadSharedProps(fetch: typeof globalThis.fetch): Promise<SharedProps> {
  // An unreachable API renders as guest with an empty sidebar rather
  // than failing every page.
  const optional = <T>(path: string): Promise<T | null> =>
    fetch(path)
      .then((response) => (response.ok ? (response.json() as Promise<T>) : null))
      .catch(() => null);

  const sidebar = optional<SidebarPayload>('/api/sidebar');
  const nav = await optional<NavState>('/api/nav-state');
  const signedIn = nav?.user != null;
  const [sidebarPayload, workbench, sentOffers] = await Promise.all([
    sidebar,
    signedIn ? optional<WorkbenchEntry[]>('/api/workbench') : null,
    signedIn ? optional<SentOffer[]>('/api/offers/sent') : null,
  ]);

  return { nav, sidebar: sidebarPayload, workbench, sentOffers };
}
