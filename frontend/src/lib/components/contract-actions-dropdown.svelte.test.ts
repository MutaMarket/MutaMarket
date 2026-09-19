import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

vi.mock('$app/state', () => ({
  page: { data: { nav: null }, url: new URL('https://mutamarket.com/') },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

const ContractActionsDropdown = (await import('./contract-actions-dropdown.svelte')).default;

import { t } from '$lib/i18n.svelte';
import type { HistoricContract } from '$lib/types';

function contract(status: string): HistoricContract {
  return {
    id: 987_654,
    status,
    type: 'item_exchange',
    price: 150_000_000,
    asking_for_items: false,
    plex_count: 0,
    non_abyssal_modules_count: 0,
    abyssal_modules_count: 1,
    issuer: null,
    date_issued: '2026-08-01 10:00:00+00',
    date_expired: '2026-08-15 10:00:00+00',
  } as HistoricContract;
}

async function openMenu(status: string) {
  const screen = await render(ContractActionsDropdown, { contract: contract(status) });
  await screen.getByRole('button', { name: t('contracts.actionsDropdown.openMenu') }).click();
  await expect.element(screen.getByRole('menu')).toBeVisible();
  return [...screen.baseElement.querySelectorAll('[role="menuitem"]')].map((item) =>
    item.textContent?.trim(),
  );
}

describe('the contract actions dropdown', () => {
  it('keeps both copies on a failed contract, minus the in-game window', async () => {
    // Expired, cancelled and deleted contracts all fold into `failed`.
    // The link pastes into chat and looks the contract up there; ESI
    // cannot open a window for one that is gone. Signed out, the
    // moderation entries stay away.
    expect(await openMenu('failed')).toEqual([
      t('contracts.actionsDropdown.copyId'),
      t('contracts.actionsDropdown.copyContractLink'),
    ]);
  });

  it('adds the in-game window for an outstanding contract', async () => {
    expect(await openMenu('outstanding')).toEqual([
      t('contracts.actionsDropdown.copyId'),
      t('contracts.actionsDropdown.copyContractLink'),
      t('contracts.actionsDropdown.openInGame'),
    ]);
  });

  it('copies the in-game link of a failed contract', async () => {
    const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    const screen = await render(ContractActionsDropdown, { contract: contract('failed') });

    await screen.getByRole('button', { name: t('contracts.actionsDropdown.openMenu') }).click();
    await screen
      .getByRole('menuitem', { name: t('contracts.actionsDropdown.copyContractLink') })
      .click();

    expect(writeText).toHaveBeenCalledWith('<url=contract:30000142//987654>Contract 987654</url>');
    writeText.mockRestore();
  });
});
