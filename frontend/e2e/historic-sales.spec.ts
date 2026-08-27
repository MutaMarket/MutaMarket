// The premium historic-sales browser: the premium gate redirects and
// the sold-price cards.
import { expect, test } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { psql } from './helpers';

function sessionFor(userId: string): string {
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`
	);
	return token;
}

test('guests are sent to the login page', async ({ page }) => {
	await page.goto('/historic-sales');
	await expect(page).toHaveURL(/\/login$/);
});

test('non-premium users are sent to the premium page', async ({ page }) => {
	const userId = psql(
		`select u.id from users u
		 where not u.is_admin
		   and not exists (select 1 from characters c
		                   where c.user_id = u.id and c.premium_paid_until > now())
		 limit 1`
	);
	test.skip(userId === '', 'no non-premium user in the database');
	await page.context().addCookies([
		{ name: 'mm_session', value: sessionFor(userId), domain: 'localhost', path: '/' }
	]);
	await page.goto('/historic-sales');
	await expect(page).toHaveURL(/\/premium$/);
});

test('premium users browse the recorded sales', async ({ page }) => {
	// Grant an hour of premium to a user with characters (dev data).
	const characterId = psql(
		`select c.id from characters c join users u on u.id = c.user_id
		 order by c.id limit 1`
	);
	test.skip(characterId === '', 'no account-linked character in the database');
	psql(
		`update characters set premium_paid_until = now() + interval '1 hour'
		 where id = ${characterId}`
	);
	const userId = psql(`select user_id from characters where id = ${characterId}`);
	await page.context().addCookies([
		{ name: 'mm_session', value: sessionFor(userId), domain: 'localhost', path: '/' }
	]);

	await page.goto('/historic-sales');
	await expect(page.getByRole('heading', { name: 'Historic Sales' })).toBeVisible();
	// Cards carry the sold strip when training data exists.
	const hasTraining = psql('select 1 from training_modules limit 1');
	if (hasTraining !== '') {
		await expect(page.locator('main a[href^="/modules/"]').first()).toBeVisible();
		// The training strip's '<estimate> | <time> ago' readout.
		await expect(page.getByText(/\| .* ago/).first()).toBeVisible();
	}
});
