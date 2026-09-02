// Success/error notifications, standing in for the legacy
// NotificationStore — delegated to sonner (the shadcn-svelte Toaster
// mounted in the root layout) instead of a hand-rolled stack.

import { toast } from 'svelte-sonner';

export function notifySuccess(title: string, body: string) {
  toast.success(title, { description: body });
}

export function notifyError(title: string, body: string) {
  toast.error(title, { description: body });
}
