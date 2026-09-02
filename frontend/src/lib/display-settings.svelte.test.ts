import { describe, expect, it, vi } from 'vitest';

vi.mock('$app/state', () => ({
  page: {
    data: {
      displaySettings: {
        display: 'list',
        attribute_bar_mode: 'type',
        show_attribute_scores: false,
      },
    },
  },
}));

const { useDisplaySettings } = await import('./display-settings.svelte');

describe('useDisplaySettings', () => {
  it('seeds from the layout data once and shares the object across pages', () => {
    const first = useDisplaySettings();
    expect(first.display).toBe('list');
    expect(first.attribute_bar_mode).toBe('type');

    // A toggle on one page is what the next page starts from, even
    // though the layout data still says list.
    first.display = 'table';
    const next = useDisplaySettings();
    expect(next).toBe(first);
    expect(next.display).toBe('table');
  });
});
