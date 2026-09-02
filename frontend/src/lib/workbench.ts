// The workbench state, the legacy useWorkbench composable + shared
// Inertia `workbench` prop: the signed-in user's scratch set of modules
// with the drawer's open flag. The server load carries the set on every
// page; mutations refresh it over the JSON API into the store, which
// then wins over the page data.
import { writable, get } from 'svelte/store';
import { page } from '$app/state';
import { t } from './i18n.svelte';
import { notifySuccess } from './toast';
import type { ModuleDetail } from './types';

export interface WorkbenchEntry {
  /** The workbench_modules row id (remove handle). */
  id: number;
  module: ModuleDetail;
}

/** null until the first mutation refreshed it; see `currentWorkbench`. */
export const workbenchEntries = writable<WorkbenchEntry[] | null>(null);
export const workbenchOpen = writable(false);

/** The set to render: the refreshed store, else the page's server-loaded
 * set, else empty (guests). */
export function currentWorkbench(
  refreshed: WorkbenchEntry[] | null,
  fromPage: WorkbenchEntry[] | null | undefined,
): WorkbenchEntry[] {
  return refreshed ?? fromPage ?? [];
}

export async function refreshWorkbench() {
  const response = await fetch('/api/workbench');
  if (response.ok) {
    workbenchEntries.set(await response.json());
  }
}

export async function addToWorkbench(moduleId: number) {
  const wasEmpty = currentWorkbench(get(workbenchEntries), page.data.workbench).length === 0;
  await fetch('/workbench-modules', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ module_id: moduleId }),
    redirect: 'manual',
  });
  notifySuccess(t('admin.workbench.moduleAddedTitle'), t('admin.workbench.moduleAddedBody'));
  await refreshWorkbench();
  // The legacy behavior: the first module opens the drawer.
  if (wasEmpty) {
    workbenchOpen.set(true);
  }
}

export async function removeFromWorkbench(entryId: number) {
  await fetch(`/workbench-modules/${entryId}`, { method: 'DELETE', redirect: 'manual' });
  notifySuccess(t('admin.workbench.moduleRemovedTitle'), t('admin.workbench.moduleRemovedBody'));
  await refreshWorkbench();
}

export async function clearWorkbench() {
  await fetch('/workbench-modules/all', { method: 'DELETE', redirect: 'manual' });
  notifySuccess(t('admin.workbench.modulesRemovedTitle'), t('admin.workbench.modulesRemovedBody'));
  await refreshWorkbench();
}

/** The shareable invitation link of the current set. */
export function workbenchShareLink(entries: WorkbenchEntry[]): string {
  return `${location.origin}/workbench/${entries.map((entry) => entry.module.id).join('/')}`;
}
