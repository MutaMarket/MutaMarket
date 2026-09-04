import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

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

// Nothing here should reach the API.
vi.stubGlobal(
  'fetch',
  vi.fn(async () => new Response(null, { status: 503 })),
);

const WorkbenchDrawer = (await import('./workbench-drawer.svelte')).default;

import { workbenchEntries, workbenchOpen } from '$lib/workbench';
import type { AssetLocationView, ModuleDetail } from '$lib/types';

function module(id: number, overrides: Partial<ModuleDetail> = {}): ModuleDetail {
  return {
    id,
    slug: `abyssal-heat-sink-${id}`,
    type: { id: 47_800, name: 'Abyssal Heat Sink' },
    creator: null,
    mutated_attributes: [],
    source_type: null,
    mutaplasmid: null,
    contract: null,
    estimated_value: null,
    estimated_value_updated_at: null,
    public_asset: { owner: { id: 90, name: 'Seller' }, price: 0 },
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

afterEach(() => {
  workbenchEntries.set(null);
  workbenchOpen.set(false);
});

describe('the compare matrix quick actions', () => {
  it('never offers on a benched module the viewer owns', async () => {
    workbenchEntries.set([
      { id: 1, module: module(7) },
      { id: 2, module: module(8, { asset: ownAsset }) },
    ]);
    workbenchOpen.set(true);
    const screen = await render(WorkbenchDrawer);
    await screen.getByText('Compare').click();
    await expect.element(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.baseElement.querySelectorAll('svg.lucide-hand-coins')).toHaveLength(1);
  });
});
