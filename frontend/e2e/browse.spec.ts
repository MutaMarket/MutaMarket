// The module browser: stats, cards, view switching, filter navigation.
import { expect, test } from '@playwright/test';

test('the browser shows stats and module cards', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'Abyssal Modules' })).toBeVisible();
	// The stats sit behind the flip toggle on the market page.
	await page.getByRole('button', { name: 'Toggle stats' }).click();
	await expect(page.getByText('Total modules')).toBeVisible();

	// The all-modules page always has cards, independent of whether the
	// live market has been swept yet.
	await page.goto('/all-modules');
	const cards = page.locator('a[href^="/modules/"]');
	await expect(cards.first()).toBeVisible();
});

test('filter navigation updates the URL and keeps the browser mounted', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('switch', { name: 'Only contracts' }).click();
	await expect(page).toHaveURL(/contracts-only/);
	await expect(page.getByRole('heading', { name: 'Abyssal Modules' })).toBeVisible();
});

test('a card click opens the module show page', async ({ page }) => {
	await page.goto('/all-modules');
	const link = page.locator('a[href^="/modules/"]').first();
	const href = await link.getAttribute('href');
	await link.click();
	await expect(page).toHaveURL(new RegExp(`${href}$`));
	// The show page hero and tab strip are up.
	await expect(page.getByText('Created by').first()).toBeVisible();
	await expect(page.getByRole('tab', { name: 'Source types' })).toBeVisible();
});
