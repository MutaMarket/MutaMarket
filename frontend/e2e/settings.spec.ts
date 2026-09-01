// The account settings page: guest redirect, the notification card,
// the connection cards and the empty prizes card.
import { expect, test } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { psql } from './helpers';

test('guests are sent to the login page', async ({ page }) => {
	await page.goto('/settings');
	await expect(page).toHaveURL(/\/login$/);
});

test('a signed-in user sees their settings cards', async ({ page }) => {
	const userId = psql(
		`select u.id from users u join characters c on c.user_id = u.id
		 group by u.id order by count(c.id) desc limit 1`,
	);
	test.skip(userId === '', 'no user with characters in the database');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`,
	);
	await page
		.context()
		.addCookies([{ name: 'mm_session', value: token, domain: 'localhost', path: '/' }]);

	await page.goto('/settings');
	await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Notifications' })).toBeVisible();
	await expect(page.getByText('Change character')).toBeVisible();
	for (const brand of ['Discord', 'Twitch', 'Patreon']) {
		await expect(page.getByRole('heading', { name: brand })).toBeVisible();
	}
	await expect(page.getByRole('heading', { name: 'Your Prizes' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Open code activation' })).toBeVisible();
});
