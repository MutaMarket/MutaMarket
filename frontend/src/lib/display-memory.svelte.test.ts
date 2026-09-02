import { describe, expect, it } from 'vitest';
import { currentDisplaySettings, defaultDisplaySettings, rememberDisplaySettings } from './display';

describe('display settings across client navigation', () => {
  it('starts the next page from the last saved settings, not the layout data', () => {
    const fromLayout = defaultDisplaySettings();
    expect(currentDisplaySettings(fromLayout)).toEqual(fromLayout);

    rememberDisplaySettings({ ...fromLayout, display: 'list', show_attribute_scores: true });
    const next = currentDisplaySettings(fromLayout);
    expect(next.display).toBe('list');
    expect(next.show_attribute_scores).toBe(true);

    // A copy each time: a page mutating its own state must not leak
    // into what the following page starts from.
    next.display = 'table';
    expect(currentDisplaySettings(fromLayout).display).toBe('list');
  });
});
