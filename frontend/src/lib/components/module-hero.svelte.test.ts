import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import ModuleHero from './module-hero.svelte';
import { MINIMUM_TRAINING_TRADES } from '$lib/estimator-score';
import type { EstimatorStatistic, ModuleDetail } from '$lib/types';

// Only the fields the hero reads; the component takes the full DTO but
// never touches the rest.
function module(overrides: Partial<ModuleDetail> = {}): ModuleDetail {
	return {
		id: 7,
		slug: 'a-module-7',
		type: { id: 47_800, name: 'Abyssal Heat Sink' },
		creator: null,
		contract: null,
		estimated_value: 142_000_000,
		estimated_value_updated_at: null,
		mutated_attributes: [],
		...overrides
	} as unknown as ModuleDetail;
}

function statistic(overrides: Partial<EstimatorStatistic> = {}): EstimatorStatistic {
	return {
		r2: null,
		mae: null,
		nmae: null,
		data_count: 0,
		data_statistics: null,
		last_trained_at: null,
		...overrides
	};
}

/** Rendered text with markup whitespace collapsed, so assertions can
 * read the way the sentence does. */
function text(container: HTMLElement): string {
	return (container.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** The meter's filled width, as the style attribute reports it. */
function meterWidth(container: HTMLElement): string | undefined {
	return container.querySelector<HTMLElement>('.bg-primary.rounded-full')?.style.width;
}

describe('module-hero untrained state', () => {
	it('counts the trades recorded against the threshold', () => {
		const { container } = render(ModuleHero, {
			module: module(),
			statistic: statistic({ data_count: 12 })
		});

		expect(text(container)).toContain(`12 / ${MINIMUM_TRAINING_TRADES}`);
		expect(text(container)).toContain('38 trades to go');
		expect(text(container)).toContain('Not enough data yet');
		expect(meterWidth(container)).toBe('24%');
	});

	it('renders an empty meter for a type with no trades at all', () => {
		const { container } = render(ModuleHero, { module: module(), statistic: null });

		expect(text(container)).toContain(`0 / ${MINIMUM_TRAINING_TRADES}`);
		expect(text(container)).toContain(`${MINIMUM_TRAINING_TRADES} trades to go`);
		expect(meterWidth(container)).toBe('0%');
	});

	it('clamps the meter and drops the countdown once the threshold is met', () => {
		// Above the threshold but still untrained: the job runs on its own
		// cadence, so this state is reachable and must not overflow.
		const { container } = render(ModuleHero, {
			module: module(),
			statistic: statistic({ data_count: MINIMUM_TRAINING_TRADES + 40 })
		});

		expect(meterWidth(container)).toBe('100%');
		expect(text(container)).toContain('queued for the next training run');
		expect(text(container)).not.toContain('to go');
	});

	it('offers the historic sales the disclaimer tells people to check', () => {
		const { container } = render(ModuleHero, { module: module(), statistic: null });

		expect(container.querySelector('a[href="/historic-sales/type/47800"]')).not.toBeNull();
	});

	it('uses the singular for the last missing trade', () => {
		const { container } = render(ModuleHero, {
			module: module(),
			statistic: statistic({ data_count: MINIMUM_TRAINING_TRADES - 1 })
		});

		expect(text(container)).toContain('1 trade to go');
	});
});

describe('module-hero trained state', () => {
	it('shows the readout instead of the shortfall', () => {
		const { container } = render(ModuleHero, {
			module: module(),
			statistic: statistic({ r2: 0.82, mae: 12_000_000, nmae: 9, data_count: 400 })
		});

		expect(text(container)).toContain('AI value prediction');
		expect(text(container)).toContain('±9%');
		expect(text(container)).not.toContain('Not enough data yet');
		expect(meterWidth(container)).toBeUndefined();
	});
});

describe('module-hero tooltips', () => {
	const trained = statistic({ r2: 0.82, mae: 12_000_000, nmae: 9, data_count: 400 });

	/** The tooltip currently open, if any. */
	function tooltip(): string {
		const content = document.querySelector('[data-slot="tooltip-content"]');
		return (content?.textContent ?? '').replace(/\s+/g, ' ').trim();
	}

	async function focusAndSettle(element: HTMLElement) {
		element.focus();
		await new Promise((resolve) => setTimeout(resolve, 400));
	}

	it('explains the prediction from a keyboard-reachable trigger', async () => {
		const { container } = render(ModuleHero, { module: module(), statistic: trained });

		// A hover-only trigger would leave this tooltip unreachable, so the
		// disclaimer sits on a button rather than the heading or a span.
		const info = container.querySelector<HTMLElement>('button.cursor-help');
		expect(info).not.toBeNull();

		await focusAndSettle(info!);
		expect(tooltip()).toContain('always do your own research');
	});

	it('announces that the estimate readout copies to the clipboard', async () => {
		const { container } = render(ModuleHero, { module: module(), statistic: trained });

		const copy = container.querySelector<HTMLElement>('button.cursor-pointer');
		expect(copy).not.toBeNull();

		await focusAndSettle(copy!);
		expect(tooltip()).toBe('Copy to clipboard');
	});
});
