import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const ActivityPage = (await import('./+page.svelte')).default;

const DAY = 86_400;

/** Two days of activity, ten days apart, so the chart's axis is wider
 * than the data and the gap-filling shows. */
function history() {
  const today = new Date(Math.floor(Date.now() / 1000 / DAY) * DAY * 1000)
    .toISOString()
    .slice(0, 10);
  return {
    window: '24h',
    step_seconds: 3600,
    traffic: [],
    routes: [],
    top_users: [],
    daily_users: [{ day: today, users: 3, requests: 40 }],
    months: [],
    totals: {
      requests: 40,
      signed_in_requests: 20,
      page_views: 10,
      active_users: 3,
      new_users: 1,
    },
  };
}

describe('the admin activity page', () => {
  it('labels the active-users chart with its own span, not the window', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(history()), { status: 200 }),
    );
    const screen = await render(ActivityPage, {
      data: { live: { activity: null }, history: history() },
    } as never);

    // The window toggle says 24h; the per-day chart still reads seven
    // days, and the count has to be interpolated into the sentence.
    await expect
      .element(screen.getByText('distinct signed-in users · last 7 days'))
      .toBeInTheDocument();
  });
});
