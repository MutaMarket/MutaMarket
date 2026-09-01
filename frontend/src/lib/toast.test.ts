import { describe, expect, it, vi } from 'vitest';

vi.mock('svelte-sonner', () => ({
	toast: { success: vi.fn(), error: vi.fn() },
}));

import { toast } from 'svelte-sonner';
import { notifyError, notifySuccess } from './toast';

describe('toast helpers', () => {
	it('maps title and body onto a sonner success toast', () => {
		notifySuccess('Copied to clipboard', 'Your estimated value has been copied.');

		expect(toast.success).toHaveBeenCalledExactlyOnceWith('Copied to clipboard', {
			description: 'Your estimated value has been copied.',
		});
	});

	it('maps title and body onto a sonner error toast', () => {
		notifyError('Update failed', 'The contract could not be updated.');

		expect(toast.error).toHaveBeenCalledExactlyOnceWith('Update failed', {
			description: 'The contract could not be updated.',
		});
	});
});
