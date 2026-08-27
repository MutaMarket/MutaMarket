// The unified statistics page: overview tiles, the creator
// leaderboard with paging and name search, and the personal section.
import { expect, test } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { psql } from './helpers';

test('the statistics page shows the overview and the leaderboard', async ({ page }) => {
	await page.goto('/statistics');
	await expect(page.getByRole('heading', { name: 'Statistics', exact: true })).toBeVisible();
	await expect(page.getByText('Known creators')).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Top Characters' })).toBeVisible();

	const rows = page.locator('table tbody tr');
	await expect(rows.first()).toBeVisible();

	// Paging moves the legacy page segment into the URL.
	// Retry the click: it can land before hydration and get lost.
	await expect(async () => {
		await page.getByRole('button', { name: 'Next' }).click();
		await expect(page).toHaveURL(/statistics\/page\/2/, { timeout: 1000 });
	}).toPass();
	await expect(rows.first()).toBeVisible();

	// A hopeless name search lands on the empty state, via the URL.
	const search = page.getByPlaceholder('Search statistics...');
	await search.fill('zzz-no-such-creator');
	await expect(page).toHaveURL(/name=zzz-no-such-creator/);
	await expect(page.getByText('No creators match your search.')).toBeVisible();

	// Guests see the sign-in invitation instead of personal stats.
	await expect(page.getByText('Sign in to see your own creation statistics.')).toBeVisible();
});

test('a signed-in user sees their personal statistics', async ({ page }) => {
	// A session for some user whose characters created modules.
	const userId = psql(
		`select u.id from users u
		 join characters c on c.user_id = u.id
		 join modules m on m.creator_id = c.id
		 group by u.id order by count(m.id) desc limit 1`
	);
	test.skip(userId === '', 'no user with created modules in the database');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	await page.context().addCookies([
		{ name: 'mm_session', value: token, domain: 'localhost', path: '/' }
	]);

	await page.goto('/statistics');
	await expect(page.getByRole('heading', { name: 'Your Statistics' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Modules created' })).toBeVisible();
	await expect(page.getByText('Money spent')).toBeVisible();
	await expect(page.getByPlaceholder('Search stats...')).toBeVisible();
});
