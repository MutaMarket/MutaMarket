// The asset-locations tree and the per-location browser.
import { expect, test } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { psql } from './helpers';

test('guests are sent to the login page', async ({ page }) => {
	await page.goto('/locations');
	await expect(page).toHaveURL(/\/login$/);
});

test('a signed-in user browses their location tree', async ({ page }) => {
	const userId = psql(
		`select u.id from users u
		 join characters c on c.user_id = u.id
		 join assets a on a.character_id = c.id
		 where a.is_abyssal group by u.id order by count(a.id) desc limit 1`
	);
	test.skip(userId === '', 'no user with abyssal assets in the database');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	await page.context().addCookies([
		{ name: 'mm_session', value: token, domain: 'localhost', path: '/' }
	]);

	await page.goto('/locations');
	await expect(page.getByRole('heading', { name: 'Your Locations' })).toBeVisible();
	await expect(page.getByPlaceholder('Search locations...')).toBeVisible();

	// Follow the first location into its module browser.
	const first = page.locator('a[href^="/locations/"]').first();
	if ((await first.count()) > 0) {
		await first.click();
		await expect(page).toHaveURL(/\/locations\/.+/);
		await expect(page.getByRole('button', { name: 'Create Collection' })).toBeVisible();
	}
});
