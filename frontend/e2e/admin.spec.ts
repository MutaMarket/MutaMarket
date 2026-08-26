// The admin operations console: guests bounce to login, a seeded admin
// session sees the job cards.
import { expect, test } from '@playwright/test';
import { adminSessionToken } from './helpers';

test('guests are sent to login', async ({ page }) => {
	await page.goto('/admin/scheduler');
	await expect(page).toHaveURL(/\/login/);
});

test('an admin session sees the scheduler dashboard', async ({ page, baseURL }) => {
	const token = adminSessionToken();
	await page.context().addCookies([
		{
			name: 'mm_session',
			value: token,
			url: baseURL ?? 'http://localhost:5100'
		}
	]);
	await page.goto('/admin/scheduler');
	await expect(page.getByText('Region contracts')).toBeVisible();
	await expect(page.getByText('Training modules')).toBeVisible();
});
