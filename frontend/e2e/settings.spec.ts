// The account settings page: guest redirect and the four tabs.
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
		 values (encode(sha256('${token}'::bytea), 'hex'), ${userId}, now() + interval '1 hour')`,
  );
  await page
    .context()
    .addCookies([{ name: 'mm_session', value: token, domain: 'localhost', path: '/' }]);

  await page.goto('/settings');
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  // The account tab opens by default: characters, access and the theme.
  await expect(page.getByRole('heading', { name: 'Characters and access' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Theme color' })).toBeVisible();

  // Retry the tab clicks: they can land before hydration and get lost.
  const openTab = async (name: string) => {
    await expect(async () => {
      await page.getByRole('tab', { name }).click();
      await expect(page).toHaveURL(new RegExp(`#${name.toLowerCase()}$`), { timeout: 1000 });
    }).toPass();
  };

  await openTab('Notifications');
  await expect(page.getByRole('heading', { name: 'EVE Mail & Raffles' })).toBeVisible();
  await expect(page.getByText('Change Character')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Blocked users' })).toBeVisible();

  await openTab('Connections');
  for (const brand of ['Discord', 'Twitch', 'Patreon']) {
    await expect(page.getByRole('heading', { name: brand })).toBeVisible();
  }

  await openTab('Prizes');
  await expect(page.getByRole('heading', { name: 'Raffle Wins' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open EVE Online Code Activation' })).toBeVisible();

  // The character menu's access anchor lands on the account tab.
  await page.goto('/settings#access');
  await expect(page.getByRole('heading', { name: 'Characters and access' })).toBeVisible();
});
