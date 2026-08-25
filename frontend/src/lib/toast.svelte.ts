// A minimal notification store, standing in for the legacy
// NotificationStore: success/error toasts that dismiss themselves.

export interface Toast {
	id: number;
	kind: 'success' | 'error';
	title: string;
	body: string;
}

/** How long a toast stays on screen. */
const TOAST_LIFETIME_MS = 4000;

let nextId = 1;
const store = $state<{ toasts: Toast[] }>({ toasts: [] });

export function toasts(): Toast[] {
	return store.toasts;
}

function push(kind: Toast['kind'], title: string, body: string) {
	const id = nextId;
	nextId += 1;
	store.toasts.push({ id, kind, title, body });
	setTimeout(() => dismiss(id), TOAST_LIFETIME_MS);
}

export function dismiss(id: number) {
	store.toasts = store.toasts.filter((toast) => toast.id !== id);
}

export function notifySuccess(title: string, body: string) {
	push('success', title, body);
}

export function notifyError(title: string, body: string) {
	push('error', title, body);
}
