// The module show page: card, hero, all three tabs and a search-menu
// round trip into the browser.
import { expect, test, type Page } from '@playwright/test';

async function openAnyModule(page: Page): Promise<void> {
	// unlisted=true: the show page must work regardless of whether the
	// live market has been swept yet.
	const response = await page.request.get('/api/module-cards?unlisted=true');
	const cards = await response.json();
	expect(cards.length).toBeGreaterThan(0);
	await page.goto(`/modules/${cards[0].slug}`);
}

test('the show page renders card, hero and source types', async ({ page }) => {
	await openAnyModule(page);
	await expect(page.getByText('Created by').first()).toBeVisible();

	// The source-types tab is the default: a table with the meta-level
	// column and at least one comparison row.
	await expect(page.getByRole('tab', { name: 'Source types' })).toBeVisible();
	await expect(page.locator('table tbody tr').first()).toBeVisible();
});

test('the contract-history tab lists contracts or an empty state', async ({ page }) => {
	await openAnyModule(page);
	await page.getByRole('tab', { name: 'Contract history' }).click();
	await expect(page.getByRole('button', { name: 'Issuer' })).toBeVisible();
});

test('the similar-sold tab teases premium to guests', async ({ page }) => {
	await openAnyModule(page);
	await page.getByRole('tab', { name: 'Similar sold' }).click();
	await expect(page.getByText('Upgrade to Premium')).toBeVisible();
});

test('the search menu builds an attribute-bounds search', async ({ page }) => {
	await openAnyModule(page);
	await page.locator('button[title="Search similar"]').click();
	await page.getByText('Select all').click();
	await page.getByRole('button', { name: 'Search modules for sale' }).click();
	await expect(page).toHaveURL(/\/attributes\//);
	await expect(page.getByRole('heading', { name: 'Abyssal Modules' })).toBeVisible();
});
