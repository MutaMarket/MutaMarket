import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The search forms navigate on submit.
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const ModuleToolbar = (await import('./module-toolbar.svelte')).default;

import { get } from 'svelte/store';

import { t } from '#lib/i18n.svelte.ts';
import { closePromote, promotionModule } from '#lib/promotion.ts';
import type { ModuleDetail } from '#lib/types.ts';

function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id: 7,
    slug: 'a-module-7',
    type: { id: 47_800, name: 'Abyssal Heat Sink' },
    creator: null,
    contract: null,
    estimated_value: null,
    estimated_value_updated_at: null,
    mutated_attributes: [],
    ...overrides,
  } as unknown as ModuleDetail;
}

describe('module-toolbar', () => {
  it('names every icon-only control for screen readers', async () => {
    // The icons carry tooltips, which only describe a control once it
    // is hovered; the name has to be on the control itself.
    const { container } = await render(ModuleToolbar, { module: module() });
    const names = [...container.querySelectorAll<HTMLElement>('button, a')].map((control) =>
      control.textContent?.trim(),
    );

    expect(names).toEqual([
      t('modules.toolbar.searchType'),
      t('modules.menu.searchSimilar'),
      t('modules.menu.searchCheapest'),
      t('modules.menu.searchHistoric'),
      'Pyfa',
      t('modules.toolbar.itemLink'),
      t('modules.toolbar.itemId'),
      t('modules.toolbar.contractLink'),
      t('modules.toolbar.openContract'),
      t('modules.menu.shareModule'),
      t('modules.menu.promote'),
      t('modules.toolbar.more'),
    ]);
  });

  it('offers promotion only for a module that is for sale', async () => {
    const promote = (container: HTMLElement) =>
      [...container.querySelectorAll<HTMLButtonElement>('button')].find(
        (control) => control.textContent?.trim() === t('modules.menu.promote'),
      );

    const unlisted = await render(ModuleToolbar, { module: module({ public_asset: null }) });
    expect(promote(unlisted.container)?.disabled).toBe(true);

    const listed = await render(ModuleToolbar, {
      module: module({ contract: { id: 1 } as ModuleDetail['contract'] }),
    });
    const button = promote(listed.container);
    expect(button?.disabled).toBe(false);
    button?.click();
    expect(get(promotionModule)?.id).toBe(7);
    closePromote();
  });
});
