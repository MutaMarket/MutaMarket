// The module browser: stats, cards, view switching, filter navigation.
import { expect, test } from '@playwright/test';

test('the browser shows the filter band and module cards', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'Modules for Sale' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Only contracts' })).toBeVisible();

	// The all-modules page always has cards, independent of whether the
	// live market has been swept yet.
	await page.goto('/all-modules');
	// Scoped to main: the nav's Appraise link also starts with /modules/.
	const cards = page.locator('main a[href^="/modules/"]');
	await expect(cards.first()).toBeVisible();
});

test('filter navigation updates the URL and keeps the browser mounted', async ({ page }) => {
	await page.goto('/');
	// Retry the click: it can land before hydration and get lost.
	await expect(async () => {
		await page.getByRole('button', { name: 'Only contracts' }).click();
		await expect(page).toHaveURL(/contracts-only/, { timeout: 1000 });
	}).toPass();
	await expect(page.getByRole('heading', { name: 'Modules for Sale' })).toBeVisible();
});

test('a card click opens the module show page', async ({ page }) => {
	await page.goto('/all-modules');
	const link = page.locator('main a[href^="/modules/"]').first();
	const href = await link.getAttribute('href');
	await link.click();
	await expect(page).toHaveURL(new RegExp(`${href}$`));
	// The show page hero and tab strip are up.
	await expect(page.getByText('Created by').first()).toBeVisible();
	await expect(page.getByRole('tab', { name: 'Source types' })).toBeVisible();
});

test('the list and table views mirror the legacy displays', async ({ page }) => {
	// A category page: the list gets sortable attribute columns.
	await page.goto('/all-modules/type/abyssal-stasis-webifier');
	// The view buttons need hydration, which lags networkidle under
	// parallel load on the dev server — click until the switch takes.
	await expect(async () => {
		await page.getByLabel('List view').first().click();
		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
	}).toPass();

	// The table view: real table rows with the Options dropdown.
	await expect(async () => {
		await page.getByLabel('Table view').first().click();
		await expect(page.locator('table')).toBeVisible({ timeout: 1000 });
	}).toPass();
	await expect(page.getByRole('button', { name: 'Options' }).first()).toBeVisible();

	// Without a category the table has no columns to offer. The view
	// choice persists through a background PUT, so retry the navigation
	// until its cookie has landed.
	await expect(async () => {
		await page.goto('/all-modules');
		await expect(page.getByText('Please select a category')).toBeVisible({ timeout: 1500 });
	}).toPass();

	// The list still works without columns: rows flow their own attributes.
	await expect(async () => {
		await page.getByLabel('List view').first().click();
		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
	}).toPass();

	// Back to the grid for the other tests (the choice persists by cookie).
	await expect(async () => {
		await page.getByLabel('Grid view').first().click();
		await expect(page.locator('.grid-cols-subgrid')).toHaveCount(0, { timeout: 1000 });
	}).toPass();
});

test('the appraise page validates and rejects a bad link', async ({ page }) => {
	await page.goto('/modules/add');
	await expect(page.getByRole('heading', { name: 'Paste an item link' })).toBeVisible();
	const appraise = page.getByRole('button', { name: 'Appraise' });
	await expect(appraise).toBeDisabled();

	// A syntactically valid link to a nonexistent item fails with the
	// legacy notification text.
	await page.waitForLoadState('networkidle');
	await page.getByPlaceholder(/showinfo/).fill('<url=showinfo:47740//1>Bogus</url>');
	await expect(appraise).toBeEnabled();
	await appraise.click();
	// The failure path calls real ESI from the dev stack; allow retries.
	await expect(page.getByText('We were unable to add the module')).toBeVisible({ timeout: 20000 });
});

test('collections can be created through the dialog and deleted', async ({ page, baseURL }) => {
	// A session for a character-owning user (create binds the active
	// character).
	const { execSync } = await import('node:child_process');
	const { randomBytes } = await import('node:crypto');
	const psql = (sql: string) =>
		execSync(
			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
			{ encoding: 'utf8' }
		).trim();
	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	psql(`delete from collections where name = 'E2E Prized Rolls'`);
	await page.context().addCookies([
		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
	]);

	await page.goto('/collections');
	await page.waitForLoadState('networkidle');
	await page.getByRole('button', { name: 'Create Collection' }).click();
	await page.getByLabel('Name').fill('E2E Prized Rolls');
	await page.getByRole('button', { name: 'Create Collection' }).last().click();
	await expect(page).toHaveURL(/\/collections\/e2e-prized-rolls-/);

	// Back on the index it sits in the personal section with the delete
	// action; deleting removes it.
	await page.goto('/collections');
	await page.waitForLoadState('networkidle');
	const card = page.locator('div').filter({ hasText: /^E2E Prized Rolls/ }).last();
	await page.getByTitle('Delete collection').first().click();
	await page.getByRole('button', { name: 'Delete', exact: true }).click();
	await expect(page.getByText('E2E Prized Rolls')).toHaveCount(0);
	void card;
});

test('the sell page shows the published set and the select dialog', async ({ page, baseURL }) => {
	const { execSync } = await import('node:child_process');
	const { randomBytes } = await import('node:crypto');
	const psql = (sql: string) =>
		execSync(
			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
			{ encoding: 'utf8' }
		).trim();
	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	await page.context().addCookies([
		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
	]);

	await page.goto('/sell/modules');
	await expect(page.getByRole('heading', { name: 'Sell Modules' })).toBeVisible();
	// Retry the click: it can land before hydration and get lost.
	await expect(async () => {
		await page.getByRole('button', { name: 'Select modules' }).click();
		await expect(page.getByText(/make whole containers public/)).toBeVisible({ timeout: 1000 });
	}).toPass();
});

test('guests are sent to login from the sell page', async ({ page }) => {
	await page.goto('/sell/modules');
	await expect(page).toHaveURL(/\/login/);
});

test('the offers index renders for a signed-in user', async ({ page, baseURL }) => {
	const { execSync } = await import('node:child_process');
	const { randomBytes } = await import('node:crypto');
	const psql = (sql: string) =>
		execSync(
			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
			{ encoding: 'utf8' }
		).trim();
	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	await page.context().addCookies([
		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
	]);

	await page.goto('/offers');
	await expect(page.getByRole('heading', { name: 'Offers' })).toBeVisible();
	await expect(page.getByText(/No offers yet|Threads/).first()).toBeVisible();
});

test('guests are sent to login from the offers page', async ({ page }) => {
	await page.goto('/offers');
	await expect(page).toHaveURL(/\/login/);
});
