import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The public-asset row reads the nav user to pick offer vs login.
vi.mock('$app/state', () => ({
  page: { data: { nav: null }, url: new URL('https://mutamarket.com/') },
}));
vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
  invalidate: vi.fn(),
  refreshAll: vi.fn(),
  afterNavigate: vi.fn(),
  beforeNavigate: vi.fn(),
  onNavigate: vi.fn(),
}));

const ModuleCard = (await import('./module-card.svelte')).default;

import { defaultDisplaySettings } from '#lib/display.ts';
import { t } from '#lib/i18n.svelte.ts';
import type { AssetLocationView, ModuleDetail } from '#lib/types.ts';

function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id: 7,
    slug: 'abyssal-heat-sink-7',
    type: { id: 47_800, name: 'Abyssal Heat Sink' },
    creator: null,
    mutated_attributes: [],
    source_type: null,
    mutaplasmid: null,
    contract: null,
    estimated_value: null,
    estimated_value_updated_at: null,
    public_asset: null,
    average_fraction: null,
    ...overrides,
  } as unknown as ModuleDetail;
}

const ownAsset: AssetLocationView = {
  parent_name: 'Jita IV - Moon 4',
  parent_type_id: null,
  parent_slug: 'jita-iv-moon-4-60003760',
  station: null,
  owner: {
    id: 42,
    slug: 'wolfgang-bunwoll-42',
    name: 'Wolfgang Bunwoll',
    description: null,
    has_premium: false,
    corporation_id: null,
  },
  location_id: 60_003_760,
  location_type: 'station',
  location_flag: 'Hangar',
  location_index: 0,
  corporation_id: null,
};

const publicAsset = { owner: { id: 90, name: 'Seller' }, price: 0 };

describe('the promoted label', () => {
  it('marks a card whose promotion put it first', async () => {
    const screen = await render(ModuleCard, {
      module: module({ promoted_until: '2099-01-01T00:00:00Z' }),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText(t('modules.card.promoted'))).toBeInTheDocument();
  });

  it('stays off every other card', async () => {
    const screen = await render(ModuleCard, {
      module: module(),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText('Abyssal Heat Sink')).toBeInTheDocument();
    expect(screen.baseElement.textContent).not.toContain(t('modules.card.promoted'));
  });
});

describe('the location row', () => {
  it('explains on hover how to find an owned module in game', async () => {
    const screen = await render(ModuleCard, {
      module: module({ asset: { ...ownAsset, location_index: 23 } }),
      settings: defaultDisplaySettings(),
    });
    // bits-ui labels the trigger a button even when it renders a link.
    await screen.getByRole('button', { name: /Jita IV - Moon 4/ }).hover();

    await expect.element(screen.getByText('How to Find It')).toBeInTheDocument();
    const card = screen.baseElement.textContent ?? '';
    expect(card).toContain('Wolfgang Bunwoll');
    // Open the container, paste the item ID into its search: no counting.
    const steps = [...screen.baseElement.querySelectorAll('[data-slot="hover-card-content"] li')];
    expect(steps.map((step) => step.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'Open Jita IV - Moon 4 in your inventory',
      "Paste the item ID 7 into the inventory's search field",
    ]);
    expect(card).toContain('only this module stays in the window');
    expect(card).not.toContain('Count from the top');
    expect(card).not.toContain('Resize your inventory');
  });

  it('copies the item ID from the directions', async () => {
    const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    const screen = await render(ModuleCard, {
      module: module({ id: 1_046_412_257_683, asset: ownAsset }),
      settings: defaultDisplaySettings(),
    });
    await screen.getByRole('button', { name: /Jita IV - Moon 4/ }).hover();
    await screen.getByRole('button', { name: '1046412257683' }).click();
    expect(writeText).toHaveBeenCalledWith('1046412257683');
  });

  it('offers on a public asset the viewer does not own', async () => {
    const screen = await render(ModuleCard, {
      module: module({ public_asset: publicAsset }),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText('Make offer')).toBeInTheDocument();
  });

  it("shows where the viewer's own module sits instead of offering on it", async () => {
    const screen = await render(ModuleCard, {
      module: module({ public_asset: publicAsset, asset: ownAsset }),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText('Jita IV - Moon 4')).toBeInTheDocument();
    expect(screen.baseElement.textContent).not.toContain('Make offer');
  });
});

describe('the module menu', () => {
  it('is a named button that carries the menu state itself', async () => {
    // The trigger used to be a span around the button, which left the
    // button nameless and put aria-expanded on an element without a
    // role.
    const screen = await render(ModuleCard, {
      module: module(),
      settings: defaultDisplaySettings(),
    });
    const trigger = screen.getByRole('button', { name: t('modules.menu.open') });
    await expect.element(trigger).toHaveAttribute('aria-haspopup', 'menu');
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.container.querySelectorAll('span[aria-expanded]')).toHaveLength(0);
  });
});
