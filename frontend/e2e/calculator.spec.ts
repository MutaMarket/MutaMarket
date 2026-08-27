// The mutation calculator: empty state, combination table, search and
// column sorting.
import { expect, test } from '@playwright/test';

test('the calculator asks for a category, then lists combinations', async ({ page }) => {
	await page.goto('/calculator');
	await expect(page.getByRole('heading', { name: 'Mutation Calculator' })).toBeVisible();
	await expect(page.getByText('Please select a category')).toBeVisible();

	await page.goto('/calculator/type/abyssal-stasis-webifier');
	await expect(page.locator('table').getByText('Stasis Webifier I', { exact: true }).first()).toBeVisible();
	// No bounds: every combination rolls into range.
	await expect(page.getByText('1 in 1').first()).toBeVisible();
	await expect(page.getByText('daily average price in Jita')).toBeVisible();
});

test('search narrows and headers sort the combination table', async ({ page }) => {
	await page.goto('/calculator/type/abyssal-stasis-webifier');
	const rows = page.locator('table tbody tr');
	await expect(rows.first()).toBeVisible();
	const before = await rows.count();

	const search = page.getByPlaceholder('Search for a module or combination');
	// Retry the fill: it can land before hydration and get lost.
	await expect(async () => {
		await search.fill('khanid');
		await expect(rows.first()).toContainText(/khanid/i, { timeout: 1000 });
	}).toPass();
	expect(await rows.count()).toBeLessThan(before);

	await search.fill('');
	// Sorting by type name puts a leading-alphabet module first.
	await page.getByRole('button', { name: 'Type', exact: true }).click();
	const first = await rows.first().textContent();
	await page.getByRole('button', { name: 'Type', exact: true }).click();
	const flipped = await rows.first().textContent();
	expect(first).not.toEqual(flipped);
});
