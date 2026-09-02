import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The public-asset row reads the nav user to pick offer vs login.
vi.mock('$app/state', () => ({
  page: { data: { nav: null }, url: new URL('https://mutamarket.com/') },
}));
vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
  invalidate: vi.fn(),
  invalidateAll: vi.fn(),
  afterNavigate: vi.fn(),
  beforeNavigate: vi.fn(),
  onNavigate: vi.fn(),
}));

const ModuleCard = (await import('./module-card.svelte')).default;

import { defaultDisplaySettings } from '$lib/display';
import type { AssetLocationView, ModuleDetail } from '$lib/types';

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
  location_id: 60_003_760,
  location_type: 'station',
  location_flag: 'Hangar',
  location_index: 0,
  corporation_id: null,
};

const publicAsset = { owner: { id: 90, name: 'Seller' }, price: 0 };

describe('the location row', () => {
  it('offers on a public asset the viewer does not own', async () => {
    const screen = render(ModuleCard, {
      module: module({ public_asset: publicAsset }),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText('Make offer')).toBeInTheDocument();
  });

  it("shows where the viewer's own module sits instead of offering on it", async () => {
    const screen = render(ModuleCard, {
      module: module({ public_asset: publicAsset, asset: ownAsset }),
      settings: defaultDisplaySettings(),
    });
    await expect.element(screen.getByText('Jita IV - Moon 4')).toBeInTheDocument();
    expect(screen.baseElement.textContent).not.toContain('Make offer');
  });
});
