// The workbench state, the legacy useWorkbench composable + shared
// Inertia `workbench` prop: the signed-in user's scratch set of modules
// with the drawer's open flag, kept fresh over the JSON API.
import { writable, get } from 'svelte/store';
import { notifySuccess } from './toast';
import type { ModuleDetail } from './types';

export interface WorkbenchEntry {
	/** The workbench_modules row id (remove handle). */
	id: number;
	module: ModuleDetail;
}

export const workbenchEntries = writable<WorkbenchEntry[]>([]);
export const workbenchOpen = writable(false);

export function isBenched(moduleId: number): boolean {
	return get(workbenchEntries).some((entry) => entry.module.id === moduleId);
}

export async function refreshWorkbench() {
	const response = await fetch('/api/workbench');
	if (response.ok) {
		workbenchEntries.set(await response.json());
	}
}

export async function addToWorkbench(moduleId: number) {
	const wasEmpty = get(workbenchEntries).length === 0;
	await fetch('/workbench-modules', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ module_id: moduleId }),
		redirect: 'manual'
	});
	notifySuccess('Module added!', 'You have successfully added the module to your workbench.');
	await refreshWorkbench();
	// The legacy behavior: the first module opens the drawer.
	if (wasEmpty) {
		workbenchOpen.set(true);
	}
}

export async function removeFromWorkbench(entryId: number) {
	await fetch(`/workbench-modules/${entryId}`, { method: 'DELETE', redirect: 'manual' });
	notifySuccess('Module removed!', 'You have successfully removed the module from your workbench.');
	await refreshWorkbench();
}

export async function clearWorkbench() {
	await fetch('/workbench-modules/all', { method: 'DELETE', redirect: 'manual' });
	notifySuccess('Modules removed!', 'You have successfully removed all modules from your workbench.');
	await refreshWorkbench();
}

/** The shareable invitation link of the current set. */
export function workbenchShareLink(entries: WorkbenchEntry[]): string {
	return `${location.origin}/workbench/${entries.map((entry) => entry.module.id).join('/')}`;
}
