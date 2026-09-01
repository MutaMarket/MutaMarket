// The tabbed statistics section: the overview telemetry board, the
// creator leaderboard with paging and name search, the personal tab,
// and the legacy URL redirect.
import { expect, test } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { psql } from './helpers';

test('the overview board and the tab rail', async ({ page }) => {
	await page.goto('/statistics');
	await expect(page.getByRole('heading', { name: 'Statistics', exact: true })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Modules in database' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Roll bars' })).toBeVisible();
	await expect(page.getByText(/Telemetry as of/)).toBeVisible();

	// The rail navigates between the sub-pages.
	await page.getByRole('link', { name: 'Top Characters' }).click();
	await expect(page).toHaveURL(/\/statistics\/characters$/);
});

test('the leaderboard pages and searches through the URL', async ({ page }) => {
	await page.goto('/statistics/characters');
	const rows = page.locator('table tbody tr');
	await expect(rows.first()).toBeVisible();

	// Paging moves the legacy page segment into the URL.
	// Retry the click: it can land before hydration and get lost.
	await expect(async () => {
		await page.getByRole('button', { name: 'Next' }).click();
		await expect(page).toHaveURL(/statistics\/characters\/page\/2/, { timeout: 1000 });
	}).toPass();
	await expect(rows.first()).toBeVisible();

	// A hopeless name search lands on the empty state, via the URL.
	const search = page.getByPlaceholder('Search statistics...');
	await search.fill('zzz-no-such-creator');
	await expect(page).toHaveURL(/name=zzz-no-such-creator/);
	await expect(page.getByText('No creators match your search.')).toBeVisible();
});

test('picking a category writes the legacy name slug and fills the trigger', async ({ page }) => {
	await page.goto('/statistics/characters');
	await expect(async () => {
		await page.getByRole('button', { name: 'All' }).click({ timeout: 1500 });
		await page
			.locator('a', { hasText: 'Ballistic Control System' })
			.first()
			.click({ timeout: 1500 });
		await expect(page).toHaveURL(
			/\/statistics\/characters\/type\/abyssal-ballistic-control-system$/,
			{
				timeout: 2000,
			},
		);
	}).toPass();
	// The trigger reflects the selection from the URL alone.
	await expect(page.getByRole('button', { name: 'Ballistic Control System' })).toBeVisible();
});

test('the pre-tabs statistics URLs redirect to the characters tab', async ({ page }) => {
	await page.goto('/statistics/page/2');
	await expect(page).toHaveURL(/\/statistics\/characters\/page\/2$/);
});

test('the personal tab shows totals to a signed-in user', async ({ page }) => {
	// Guests get the invitation.
	await page.goto('/statistics/personal');
	await expect(page.getByText('Sign in to see your own creation statistics.')).toBeVisible();

	// A session for some user whose characters created modules.
	const userId = psql(
		`select u.id from users u
		 join characters c on c.user_id = u.id
		 join modules m on m.creator_id = c.id
		 group by u.id order by count(m.id) desc limit 1`,
	);
	test.skip(userId === '', 'no user with created modules in the database');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`,
	);
	await page
		.context()
		.addCookies([{ name: 'mm_session', value: token, domain: 'localhost', path: '/' }]);

	await page.goto('/statistics/personal');
	await expect(page.getByRole('heading', { name: 'Modules created' })).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Money spent' })).toBeVisible();
	await expect(page.getByPlaceholder('Search stats...')).toBeVisible();
});
