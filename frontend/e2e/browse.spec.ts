// The module browser: stats, cards, view switching, filter navigation.
import { expect, test } from '@playwright/test';

test('the browser shows the filter band and module cards', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'Abyssal Modules' })).toBeVisible();
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
	await page.getByRole('button', { name: 'Only contracts' }).click();
	await expect(page).toHaveURL(/contracts-only/);
	await expect(page.getByRole('heading', { name: 'Abyssal Modules' })).toBeVisible();
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
