# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: browse.spec.ts >> collections can be created through the dialog and deleted
- Location: e2e/browse.spec.ts:92:1

# Error details

```
Error: locator.fill: Error: strict mode violation: getByLabel('Name') resolved to 2 elements:
    1) <button type="button" aria-label="Rename bookmark" class="relative z-10 cursor-pointer text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground">…</button> aka getByRole('button', { name: 'Rename bookmark' })
    2) <input type="text" data-slot="input" id="collection-name" class="h-7 rounded-md border border-input bg-input/20 px-2 py-0.5 text-sm transition-colors file:h-6 file:text-xs/relaxed file:font-medium focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30 aria-invalid:border-destructive aria-invalid:ring-2 aria-invalid:ring-destructive/20 md:text-xs/relaxed dark:bg-input/30 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 w-full min-w-0 outline-none file:in…/> aka getByRole('textbox', { name: 'Name' })

Call log:
  - waiting for getByLabel('Name')

```

# Page snapshot

```yaml
- generic:
  - generic:
    - banner:
      - generic:
        - generic:
          - generic:
            - link:
              - /url: /
            - navigation:
              - link "Buy":
                - /url: /
              - link "Appraise":
                - /url: /modules/add
              - link "Sell":
                - /url: /sell/modules
              - link "Characters":
                - /url: /characters
              - link "Collections":
                - /url: /collections
              - link "Offers":
                - /url: /offers
              - link "My modules":
                - /url: /personal/modules
              - button "More"
            - button "Grim Dredtog"
    - main:
      - generic:
        - generic:
          - generic:
            - generic:
              - heading "Collections" [level=1]
              - paragraph: Curated module showcases by the community
          - generic:
            - generic:
              - generic:
                - textbox "Search collections"
              - button "Create Collection"
        - generic:
          - heading "Your Collections" [level=2]
          - paragraph: You have not created any collections yet.
        - generic:
          - heading "Public Collections" [level=2]
          - generic:
            - generic:
              - generic:
                - generic:
                  - link "Liquidation (all)":
                    - /url: /collections/liquidation-all-MVJ1g8rrpADU2RCk
                  - paragraph:
                    - generic: by Aerodinamica Attiva
                - generic: Premium
              - paragraph: SELLING ORDERS OF 10B+ ONLY
              - generic:
                - generic: "+42"
                - generic: 577 modules
            - generic:
              - generic:
                - generic:
                  - link "All for sale":
                    - /url: /collections/all-for-sale-OwEj2oRLK8ec1KNH
                  - paragraph:
                    - generic: by Vernkon Seig Aldard
                - generic: Premium
              - generic:
                - generic: "+46"
                - generic: 5,571 modules
            - generic:
              - generic:
                - generic:
                  - link "Capital abyssal mods":
                    - /url: /collections/capital-abyssal-mods-lcOVO08mK1sTDsF3
                  - paragraph:
                    - generic: by Ty Bacard
                - generic: Premium
              - generic: 42 modules
            - generic:
              - generic:
                - generic:
                  - link "wts":
                    - /url: /collections/wts-2RmeipRoR6crVGPr
                  - paragraph:
                    - generic: by Henry Mancini
                - generic: Premium
              - paragraph: Delivery to JITA in 1 day
              - generic:
                - generic: "+3"
                - generic: 15 modules
            - generic:
              - generic:
                - generic:
                  - link "For sale":
                    - /url: /collections/for-sale-hmxePV7trb2rLk7t
                  - paragraph:
                    - generic: by Reslo Kusoni
                - generic: Premium
              - paragraph: In jita or fountain. free shipping.
              - generic:
                - generic: "+1"
                - generic: 21 modules
            - generic:
              - generic:
                - generic:
                  - link "New Collection":
                    - /url: /collections/new-collection-yJpMSYN3tTo9rboG
                  - paragraph:
                    - generic: by Astrocytoma
                - generic: Premium
              - paragraph: New Collection
              - generic: 3 modules
            - generic:
              - generic:
                - generic:
                  - link "offer":
                    - /url: /collections/offer-YeD0vday8tQMYlyY
                  - paragraph:
                    - generic: by Reslo Kusoni
                - generic: Premium
              - paragraph: "send a mail in game to: Reslo | Reslo kusoni or msg me on discord: \"Reslo.\""
              - generic: 1 modules
            - generic:
              - generic:
                - generic:
                  - link "Fit Puzzle Reward":
                    - /url: /collections/fit-puzzle-reward-gI0GMm9EXhbRQBrL
                  - paragraph:
                    - generic: by Astrocytoma
                - generic: Premium
              - generic: 5 modules
            - generic:
              - generic:
                - generic:
                  - link "For sale":
                    - /url: /collections/for-sale-C6wzayTMVgPCyD0C
                  - paragraph:
                    - generic: by Meridthal2
                - generic: Premium
              - generic:
                - generic: "+41"
                - generic: 1,081 modules
            - generic:
              - generic:
                - generic:
                  - link "WTS":
                    - /url: /collections/wts-4Ixcf3QGKAg4Fv7j
                  - paragraph:
                    - generic: by Rangeen
              - paragraph: WTS all of them
              - generic: 1 modules
            - generic:
              - generic:
                - generic:
                  - link "Jita Collection":
                    - /url: /collections/jita-collection-ZoPiMWdGTt05NCUy
                  - paragraph:
                    - generic: by Solara Merlin
              - paragraph: Jita
              - generic:
                - generic: "+5"
                - generic: 82 modules
            - generic:
              - generic:
                - generic:
                  - link "For Sale":
                    - /url: /collections/for-sale-hqiqd3tnwiNjIHIR
                  - paragraph:
                    - generic: by indCraig
              - generic:
                - generic: "+2"
                - generic: 61 modules
      - generic:
        - generic:
          - generic:
            - generic:
              - generic: Bookmarks
              - generic: "1"
            - button "Add current page"
          - generic:
            - generic:
              - link "Abyssal Entropic Radiation Sink":
                - /url: /modules/type/49734
              - button "Rename bookmark"
              - button "Delete bookmark"
        - generic:
          - link "EVE Store":
            - /url: https://store.eveonline.com
            - img "EVE Store"
          - generic: Advertisement
        - generic:
          - generic: Premium
          - generic:
            - paragraph: Unlock historic sales, similar modules, priority ordering, and more.
            - generic:
              - generic:
                - generic: Monthly
                - generic: 100 million ISK
              - generic:
                - generic: Yearly
                - generic: 1 billion ISK
              - paragraph: Save 2 months with yearly
          - generic:
            - button "Send ISK to MutaMate":
              - generic: Send ISK to
              - code: MutaMate
        - link "Buy me some Quafe Help me stay awake and code more":
          - /url: https://ko-fi.com/nicolaskion
          - generic:
            - generic: Buy me some Quafe
            - generic: Help me stay awake and code more
        - generic:
          - generic: Partner
          - link "WormholeSystems Wormhole mapping & intel":
            - /url: https://wormhole.systems
            - generic:
              - generic: WormholeSystems
              - generic: Wormhole mapping & intel
    - contentinfo:
      - paragraph: MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online.
    - region "Notifications alt+T"
    - button "Workbench 3":
      - generic: Workbench
      - generic: "3"
  - dialog [ref=e2]:
    - heading "Create a new collection" [level=2] [ref=e3]
    - generic [ref=e4]: Create a new collection to organize your assets. You can add assets to this collection later.
    - generic [ref=e5]:
      - generic [ref=e6]:
        - generic [ref=e7]: Name
        - textbox "Name" [active] [ref=e8]
      - generic [ref=e9]:
        - generic [ref=e10]: Description
        - textbox "Description" [ref=e11]
      - generic [ref=e12]:
        - generic [ref=e13]: Visibility
        - radiogroup [ref=e14]:
          - generic [ref=e15]:
            - radio "private" [checked] [ref=e16] [cursor=pointer]
            - text: private
          - generic [ref=e19]:
            - radio "unlisted" [ref=e20] [cursor=pointer]
            - text: unlisted
          - generic [ref=e22]:
            - radio "public" [ref=e23] [cursor=pointer]
            - text: public
        - paragraph [ref=e25]: Private collections are only visible to you.
      - generic [ref=e26]:
        - button "Cancel" [ref=e27] [cursor=pointer]
        - button "Create Collection" [disabled]
    - button "Close" [ref=e28] [cursor=pointer]
```

# Test source

```ts
  15  | });
  16  | 
  17  | test('filter navigation updates the URL and keeps the browser mounted', async ({ page }) => {
  18  | 	await page.goto('/');
  19  | 	// Retry the click: it can land before hydration and get lost.
  20  | 	await expect(async () => {
  21  | 		await page.getByRole('button', { name: 'Only contracts' }).click();
  22  | 		await expect(page).toHaveURL(/contracts-only/, { timeout: 1000 });
  23  | 	}).toPass();
  24  | 	await expect(page.getByRole('heading', { name: 'Modules for Sale' })).toBeVisible();
  25  | });
  26  | 
  27  | test('a card click opens the module show page', async ({ page }) => {
  28  | 	await page.goto('/all-modules');
  29  | 	const link = page.locator('main a[href^="/modules/"]').first();
  30  | 	const href = await link.getAttribute('href');
  31  | 	await link.click();
  32  | 	await expect(page).toHaveURL(new RegExp(`${href}$`));
  33  | 	// The show page hero and tab strip are up.
  34  | 	await expect(page.getByText('Created by').first()).toBeVisible();
  35  | 	await expect(page.getByRole('tab', { name: 'Source types' })).toBeVisible();
  36  | });
  37  | 
  38  | test('the list and table views mirror the legacy displays', async ({ page }) => {
  39  | 	// A category page: the list gets sortable attribute columns.
  40  | 	await page.goto('/all-modules/type/abyssal-stasis-webifier');
  41  | 	// The view buttons need hydration, which lags networkidle under
  42  | 	// parallel load on the dev server — click until the switch takes.
  43  | 	await expect(async () => {
  44  | 		await page.getByLabel('List view').first().click();
  45  | 		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
  46  | 	}).toPass();
  47  | 
  48  | 	// The table view: real table rows with the Options dropdown.
  49  | 	await expect(async () => {
  50  | 		await page.getByLabel('Table view').first().click();
  51  | 		await expect(page.locator('table')).toBeVisible({ timeout: 1000 });
  52  | 	}).toPass();
  53  | 	await expect(page.getByRole('button', { name: 'Options' }).first()).toBeVisible();
  54  | 
  55  | 	// Without a category the table has no columns to offer. The view
  56  | 	// choice persists through a background PUT, so retry the navigation
  57  | 	// until its cookie has landed.
  58  | 	await expect(async () => {
  59  | 		await page.goto('/all-modules');
  60  | 		await expect(page.getByText('Please select a category')).toBeVisible({ timeout: 1500 });
  61  | 	}).toPass();
  62  | 
  63  | 	// The list still works without columns: rows flow their own attributes.
  64  | 	await expect(async () => {
  65  | 		await page.getByLabel('List view').first().click();
  66  | 		await expect(page.locator('.grid-cols-subgrid').first()).toBeVisible({ timeout: 1000 });
  67  | 	}).toPass();
  68  | 
  69  | 	// Back to the grid for the other tests (the choice persists by cookie).
  70  | 	await expect(async () => {
  71  | 		await page.getByLabel('Grid view').first().click();
  72  | 		await expect(page.locator('.grid-cols-subgrid')).toHaveCount(0, { timeout: 1000 });
  73  | 	}).toPass();
  74  | });
  75  | 
  76  | test('the appraise page validates and rejects a bad link', async ({ page }) => {
  77  | 	await page.goto('/modules/add');
  78  | 	await expect(page.getByRole('heading', { name: 'Paste an item link' })).toBeVisible();
  79  | 	const appraise = page.getByRole('button', { name: 'Appraise' });
  80  | 	await expect(appraise).toBeDisabled();
  81  | 
  82  | 	// A syntactically valid link to a nonexistent item fails with the
  83  | 	// legacy notification text.
  84  | 	await page.waitForLoadState('networkidle');
  85  | 	await page.getByPlaceholder(/showinfo/).fill('<url=showinfo:47740//1>Bogus</url>');
  86  | 	await expect(appraise).toBeEnabled();
  87  | 	await appraise.click();
  88  | 	// The failure path calls real ESI from the dev stack; allow retries.
  89  | 	await expect(page.getByText('We were unable to add the module')).toBeVisible({ timeout: 20000 });
  90  | });
  91  | 
  92  | test('collections can be created through the dialog and deleted', async ({ page, baseURL }) => {
  93  | 	// A session for a character-owning user (create binds the active
  94  | 	// character).
  95  | 	const { execSync } = await import('node:child_process');
  96  | 	const { randomBytes } = await import('node:crypto');
  97  | 	const psql = (sql: string) =>
  98  | 		execSync(
  99  | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  100 | 			{ encoding: 'utf8' }
  101 | 		).trim();
  102 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  103 | 	const token = randomBytes(24).toString('hex');
  104 | 	psql(
  105 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  106 | 	);
  107 | 	psql(`delete from collections where name = 'E2E Prized Rolls'`);
  108 | 	await page.context().addCookies([
  109 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  110 | 	]);
  111 | 
  112 | 	await page.goto('/collections');
  113 | 	await page.waitForLoadState('networkidle');
  114 | 	await page.getByRole('button', { name: 'Create Collection' }).click();
> 115 | 	await page.getByLabel('Name').fill('E2E Prized Rolls');
      |                                ^ Error: locator.fill: Error: strict mode violation: getByLabel('Name') resolved to 2 elements:
  116 | 	await page.getByRole('button', { name: 'Create Collection' }).last().click();
  117 | 	await expect(page).toHaveURL(/\/collections\/e2e-prized-rolls-/);
  118 | 
  119 | 	// Back on the index it sits in the personal section with the delete
  120 | 	// action; deleting removes it.
  121 | 	await page.goto('/collections');
  122 | 	await page.waitForLoadState('networkidle');
  123 | 	const card = page.locator('div').filter({ hasText: /^E2E Prized Rolls/ }).last();
  124 | 	await page.getByTitle('Delete collection').first().click();
  125 | 	await page.getByRole('button', { name: 'Delete', exact: true }).click();
  126 | 	await expect(page.getByText('E2E Prized Rolls')).toHaveCount(0);
  127 | 	void card;
  128 | });
  129 | 
  130 | test('the sell page shows the published set and the select dialog', async ({ page, baseURL }) => {
  131 | 	const { execSync } = await import('node:child_process');
  132 | 	const { randomBytes } = await import('node:crypto');
  133 | 	const psql = (sql: string) =>
  134 | 		execSync(
  135 | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  136 | 			{ encoding: 'utf8' }
  137 | 		).trim();
  138 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  139 | 	const token = randomBytes(24).toString('hex');
  140 | 	psql(
  141 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  142 | 	);
  143 | 	await page.context().addCookies([
  144 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  145 | 	]);
  146 | 
  147 | 	await page.goto('/sell/modules');
  148 | 	await expect(page.getByRole('heading', { name: 'Sell Modules' })).toBeVisible();
  149 | 	// Retry the click: it can land before hydration and get lost.
  150 | 	await expect(async () => {
  151 | 		await page.getByRole('button', { name: 'Select modules' }).click();
  152 | 		await expect(page.getByText(/make whole containers public/)).toBeVisible({ timeout: 1000 });
  153 | 	}).toPass();
  154 | });
  155 | 
  156 | test('guests are sent to login from the sell page', async ({ page }) => {
  157 | 	await page.goto('/sell/modules');
  158 | 	await expect(page).toHaveURL(/\/login/);
  159 | });
  160 | 
  161 | test('the offers index renders for a signed-in user', async ({ page, baseURL }) => {
  162 | 	const { execSync } = await import('node:child_process');
  163 | 	const { randomBytes } = await import('node:crypto');
  164 | 	const psql = (sql: string) =>
  165 | 		execSync(
  166 | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  167 | 			{ encoding: 'utf8' }
  168 | 		).trim();
  169 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  170 | 	const token = randomBytes(24).toString('hex');
  171 | 	psql(
  172 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  173 | 	);
  174 | 	await page.context().addCookies([
  175 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  176 | 	]);
  177 | 
  178 | 	await page.goto('/offers');
  179 | 	await expect(page.getByRole('heading', { name: 'Offers' })).toBeVisible();
  180 | 	await expect(page.getByText(/No offers yet|Threads/).first()).toBeVisible();
  181 | });
  182 | 
  183 | test('guests are sent to login from the offers page', async ({ page }) => {
  184 | 	await page.goto('/offers');
  185 | 	await expect(page).toHaveURL(/\/login/);
  186 | });
  187 | 
  188 | test('the workbench drawer opens with benched modules', async ({ page, baseURL }) => {
  189 | 	const { execSync } = await import('node:child_process');
  190 | 	const { randomBytes } = await import('node:crypto');
  191 | 	const psql = (sql: string) =>
  192 | 		execSync(
  193 | 			`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(sql.replace(/\s+/g, ' ').trim())}`,
  194 | 			{ encoding: 'utf8' }
  195 | 		).trim();
  196 | 	const userId = psql('select user_id from characters where user_id is not null order by id limit 1');
  197 | 	const moduleId = psql('select id from modules order by id desc limit 1');
  198 | 	psql(
  199 | 		`insert into workbench_modules (user_id, module_id) values (${userId}, ${moduleId}) on conflict do nothing`
  200 | 	);
  201 | 	const token = randomBytes(24).toString('hex');
  202 | 	psql(
  203 | 		`insert into sessions (token, user_id, expires_at) values ('${token}', ${userId}, now() + interval '1 hour')`
  204 | 	);
  205 | 	await page.context().addCookies([
  206 | 		{ name: 'mm_session', value: token, url: baseURL ?? 'http://localhost:5100' }
  207 | 	]);
  208 | 
  209 | 	await page.goto('/');
  210 | 	// The collapsed pill appears once the workbench loads; opening it
  211 | 	// shows the drawer with its views.
  212 | 	await expect(async () => {
  213 | 		await page.getByRole('button', { name: /Workbench/ }).click();
  214 | 		await expect(page.getByRole('button', { name: 'Compare' })).toBeVisible({ timeout: 1000 });
  215 | 	}).toPass();
```