import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import RangeSlider from './range-slider.svelte';

describe('range-slider', () => {
  it('names each handle and announces its formatted value', async () => {
    const screen = await render(RangeSlider, {
      values: [10, 90],
      labels: ['Minimum Price', 'Maximum Price'],
      valueText: (position: number) => `${position} million ISK`,
    });

    const lower = screen.getByRole('slider', { name: 'Minimum Price' });
    await expect.element(lower).toHaveAttribute('aria-valuenow', '10');
    await expect.element(lower).toHaveAttribute('aria-valuetext', '10 million ISK');

    const upper = screen.getByRole('slider', { name: 'Maximum Price' });
    await expect.element(upper).toHaveAttribute('aria-valuenow', '90');
    await expect.element(upper).toHaveAttribute('aria-valuetext', '90 million ISK');
  });
});
