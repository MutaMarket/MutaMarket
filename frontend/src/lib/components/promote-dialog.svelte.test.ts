import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The dialog reads the shared sidebar config for the rate and the
// character the ISK goes to.
vi.mock('$app/state', () => ({
  page: {
    data: { sidebar: { premium_character: 'MutaMate', promotion_daily_cost: 25_000_000 } },
    url: new URL('https://mutamarket.com/'),
  },
}));

// The sidebar module the dialog shares its config with reads these.
vi.mock('$app/env/public', () => ({
  PUBLIC_PATREON_LINK: '',
  PUBLIC_KOFI_LINK: '',
  PUBLIC_MARKEEDRAGON_URL: '',
  PUBLIC_MARKEEDRAGON_CODE: '',
  PUBLIC_ADSENSE_CLIENT_ID: '',
}));

const PromoteDialog = (await import('./promote-dialog.svelte')).default;

import { t } from '#lib/i18n.svelte.ts';
import { closePromote, openPromote } from '#lib/promotion.ts';
import type { ModuleDetail } from '#lib/types.ts';

function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id: 1_047_593_018_621,
    slug: 'abyssal-heat-sink-1047593018621',
    type: { id: 47_800, name: 'Abyssal Heat Sink' },
    source_type: null,
    contract: null,
    public_asset: null,
    ...overrides,
  } as unknown as ModuleDetail;
}

describe('the promote dialog', () => {
  it('stays closed until a module is picked', async () => {
    closePromote();
    const screen = await render(PromoteDialog);
    expect(screen.baseElement.textContent).not.toContain(t('modules.promote.title'));
  });

  it('shows the amount, the character and the reason code', async () => {
    const screen = await render(PromoteDialog);
    openPromote(module());

    await expect.element(screen.getByText(t('modules.promote.title'))).toBeInTheDocument();
    // Three days by default.
    await expect.element(screen.getByText(/ISK\s75,000,000$/)).toBeInTheDocument();
    await expect.element(screen.getByText('MutaMate', { exact: true })).toBeInTheDocument();
    await expect
      .element(screen.getByText('promote 1047593018621', { exact: true }))
      .toBeInTheDocument();
    await expect.element(screen.getByText(t('modules.promote.notRunning'))).toBeInTheDocument();

    await screen.getByLabelText(t('modules.promote.daysLabel')).fill('10');
    await expect.element(screen.getByText(/ISK\s250,000,000$/)).toBeInTheDocument();
    closePromote();
  });

  it('says when a running promotion ends', async () => {
    const screen = await render(PromoteDialog);
    openPromote(module({ promoted_until: '2099-01-02T03:04:05Z' }));
    await expect
      .element(screen.getByText(t('modules.promote.runningUntil', { date: '2099-01-02' })))
      .toBeInTheDocument();
    closePromote();
  });
});
