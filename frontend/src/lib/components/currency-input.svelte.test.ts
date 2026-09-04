import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import CurrencyInput from './currency-input.svelte';

describe('CurrencyInput', () => {
  it('floats the short compact form of what is typed', async () => {
    const screen = await render(CurrencyInput, {
      value: '1500000000',
      label: 'Asking price',
      onchange: () => {},
    });
    await expect.element(screen.getByText('1.5B')).toBeInTheDocument();
  });

  it('shows the empty text instead when the field is blank', async () => {
    const screen = await render(CurrencyInput, {
      value: '',
      label: 'Asking price',
      empty: 'no price',
      onchange: () => {},
    });
    await expect.element(screen.getByText('no price')).toBeInTheDocument();
  });

  it('carries the ISK tag by default and drops it on request', async () => {
    const withUnit = await render(CurrencyInput, {
      value: '10',
      label: 'price lower bound',
      onchange: () => {},
    });
    await expect.element(withUnit.getByText('ISK')).toBeInTheDocument();

    const withoutUnit = await render(CurrencyInput, {
      value: '10',
      label: 'Asking price',
      unit: false,
      onchange: () => {},
    });
    expect(withoutUnit.container.textContent).not.toContain('ISK');
  });

  it('multiplies by a million on m and a billion on b', async () => {
    const onchange = vi.fn();
    const screen = await render(CurrencyInput, { value: '2', label: 'Amount', onchange });
    const field = screen.getByLabelText('Amount');

    await field.click();
    await screen
      .getByLabelText('Amount')
      .element()
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'm', bubbles: true }));
    expect(onchange).toHaveBeenLastCalledWith('2000000');

    screen
      .getByLabelText('Amount')
      .element()
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'b', bubbles: true }));
    expect(onchange).toHaveBeenLastCalledWith('2000000000');
  });

  it('submits on Enter', async () => {
    const onenter = vi.fn();
    const screen = await render(CurrencyInput, {
      value: '5',
      label: 'Amount',
      onchange: () => {},
      onenter,
    });
    screen
      .getByLabelText('Amount')
      .element()
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    expect(onenter).toHaveBeenCalled();
  });
});

describe('the ceiling', () => {
  it('leaves a value at the ceiling alone', async () => {
    const screen = await render(CurrencyInput, {
      value: '1000000000000',
      label: 'Asking price',
      max: 1_000_000_000_000,
      onchange: () => {},
    });
    await expect.element(screen.getByText('1T')).toBeInTheDocument();
    expect(screen.container.querySelector('input')?.getAttribute('aria-invalid')).not.toBe('true');
  });

  it('flags a value above the ceiling', async () => {
    const screen = await render(CurrencyInput, {
      value: '2000000000000',
      label: 'Asking price',
      max: 1_000_000_000_000,
      onchange: () => {},
    });
    await expect.element(screen.getByText('max 1T')).toBeInTheDocument();
    expect(screen.container.querySelector('input')?.getAttribute('aria-invalid')).toBe('true');
  });
});
