import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const OverviewPage = (await import('./+page.svelte')).default;

/** The box on 2026-09-11: 4.6 GB of 7.5 GB in use, of which the API
 * container held 1.6 GB. The console used to report only the 1.6. */
const HOST_TOTAL_BYTES = 7_916_036 * 1024;
const HOST_USED_BYTES = (7_916_036 - 3_099_880) * 1024;
const API_BYTES = 1_703_505_920;

const system = {
  disk_used_bytes: 40 * 1024 ** 3,
  disk_total_bytes: 150 * 1024 ** 3,
  memory_total_bytes: HOST_TOTAL_BYTES,
  memory_rss_bytes: API_BYTES,
  memory_current_bytes: API_BYTES,
  memory_limit_bytes: null,
  host_memory_used_bytes: HOST_USED_BYTES,
  host_cpu_seconds: 1_000,
  cpu_seconds: 100,
  cpu_cores: 4,
  network_rx_bytes: 10,
  network_tx_bytes: 20,
  host_network_rx_bytes: 57_563_795_512,
  host_network_tx_bytes: 430_962_483_426,
  uptime_seconds: 4_000,
  database_size_bytes: 20 * 1024 ** 3,
};

describe('the console overview vitals', () => {
  it('reports the machine, with the api container beside it', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ window: '24h', step_seconds: 300, series: {} }), {
        status: 200,
      }),
    );
    const screen = await render(OverviewPage, {
      data: {
        live: { system, jobs: [] },
        service: { character: null, source: null },
      },
    } as never);

    // 4.6 of 7.5 GB is 61% of the box; the old readout showed the
    // container's 1.6 GB against the same capacity, so it said 20%.
    await expect.element(screen.getByText('61%')).toBeInTheDocument();
    await expect.element(screen.getByText('4.6 GB of 7.5 GB · api 1.6 GB')).toBeInTheDocument();
  });

  it('gives each network direction its own readout, scoped to the machine', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ window: '24h', step_seconds: 300, series: {} }), {
        status: 200,
      }),
    );
    const screen = await render(OverviewPage, {
      data: {
        live: { system, jobs: [] },
        service: { character: null, source: null },
      },
    } as never);

    await expect.element(screen.getByText('Network in')).toBeInTheDocument();
    await expect.element(screen.getByText('Network out')).toBeInTheDocument();
    // Both say whose traffic it is: the uplinks, not our veth.
    const scope = screen.getByText('server uplinks');
    await expect.element(scope.first()).toBeInTheDocument();
  });
});
