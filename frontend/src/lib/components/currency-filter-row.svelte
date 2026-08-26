<script lang="ts">
	// The price / estimated-value filter row, mirroring the legacy
	// PriceFilter.vue and ValueFilter.vue: wallet title, compact ISK
	// inputs (m/b multiplier keys), a log-scale slider, sort trio
	// (specs/browser-filters.md §4).
	import { Wallet } from '@lucide/svelte';
	import RangeSlider, { type SliderMark } from './range-slider.svelte';
	import SortButtons from './sort-buttons.svelte';
	import { goto } from '$app/navigation';
	import { Input } from '$lib/components/ui/input';
	import { toVeryCompact } from '$lib/format-number';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import { clamp, currencyToNormalized, currencyToOriginal } from '$lib/slider-scale';

	let {
		prefix,
		search,
		kind
	}: {
		prefix: string;
		search: UiSearch;
		/** `price` (reversed slider, single bound = maximum) or `value`. */
		kind: 'price' | 'value';
	} = $props();

	/** The legacy hardcoded ISK slider range. */
	const LOWEST = 1_000_000;
	const HIGHEST = 100_000_000_000;
	const LABEL_STEP = 20;
	const SEARCH_DEBOUNCE_MS = 200;

	const reversed = $derived(kind === 'price');
	const bounds = $derived(kind === 'price' ? search.price : search.value);

	function normalized(amount: number): number {
		return clamp(currencyToNormalized(amount, LOWEST, HIGHEST), 0, 100);
	}

	function initialValues(): [number, number] {
		if (bounds === null) {
			return [0, 100];
		}
		const [lower, upper] = bounds;
		if (upper !== null) {
			const first = normalized(lower);
			const second = normalized(upper);
			return [Math.min(first, second), Math.max(first, second)];
		}
		// Lower-bound-only: the reversed price slider reads its single
		// bound from the right handle ("at most X"), the value slider
		// from the left ("at least X").
		return reversed ? [0, normalized(lower)] : [normalized(lower), 100];
	}

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let values: [number, number] = $state(initialValues());

	const marks: SliderMark[] = Array.from({ length: 100 / LABEL_STEP + 1 }, (_, index) => ({
		position: index * LABEL_STEP,
		kind: 'regular' as const,
		label: toVeryCompact(currencyToOriginal(index * LABEL_STEP, LOWEST, HIGHEST))
	}));

	function navigate([lower, upper]: [number, number]) {
		let next: [number, number | null] | null;
		if (lower === 0 && upper === 100) {
			next = null;
		} else if (reversed && lower === 0) {
			// Price with only the right handle moved: "at most X".
			next = [currencyToOriginal(upper, LOWEST, HIGHEST), null];
		} else if (!reversed && upper === 100) {
			// Value with only the left handle moved: "at least X".
			next = [currencyToOriginal(lower, LOWEST, HIGHEST), null];
		} else {
			next = [
				currencyToOriginal(lower, LOWEST, HIGHEST),
				currencyToOriginal(upper, LOWEST, HIGHEST)
			];
		}
		goto(buildQueryPath(prefix, { ...search, [kind]: next }), {
			keepFocus: true,
			noScroll: true
		});
	}

	let debounce: ReturnType<typeof setTimeout> | null = null;
	function searchSoon(next: [number, number]) {
		if (debounce !== null) {
			clearTimeout(debounce);
		}
		debounce = setTimeout(() => navigate(next), SEARCH_DEBOUNCE_MS);
	}

	let lowerInput = $state('');
	let upperInput = $state('');
	$effect(() => {
		lowerInput = String(Math.round(currencyToOriginal(values[0], LOWEST, HIGHEST)));
		upperInput = String(Math.round(currencyToOriginal(values[1], LOWEST, HIGHEST)));
	});

	function submitInputs() {
		const lower = Number(lowerInput);
		const upper = Number(upperInput);
		if (Number.isNaN(lower) || Number.isNaN(upper)) {
			return;
		}
		values = [normalized(lower), normalized(upper)];
		navigate(values);
	}

	/** Typing m or b multiplies the focused input (legacy shortcut). */
	function onInputKey(event: KeyboardEvent, bound: 0 | 1) {
		const multiplier = event.key === 'm' ? 1_000_000 : event.key === 'b' ? 1_000_000_000 : null;
		if (multiplier !== null) {
			event.preventDefault();
			if (bound === 0) {
				lowerInput = String(Number(lowerInput) * multiplier);
			} else {
				upperInput = String(Number(upperInput) * multiplier);
			}
			return;
		}
		if (event.key === 'Enter') {
			submitInputs();
		}
	}
</script>

<div class="flex gap-2 p-4">
	<div class="flex w-full flex-wrap items-start gap-2">
		<h2 class="flex items-center gap-2 text-sm font-medium">
			<Wallet class="size-4" />
			<span>{kind === 'price' ? 'Price' : 'Est. value'}</span>
		</h2>
		<div class="ml-auto grid w-full max-w-[300px] grid-cols-2">
			{#each [0, 1] as bound (bound)}
				<div class="relative">
					<Input
						class="h-8 w-full min-w-0 {bound === 0
							? 'rounded-r-none'
							: 'rounded-l-none border-l-0'} border border-border/50 bg-input text-right text-xs"
						type="number"
						aria-label="{kind} {bound === 0 ? 'lower' : 'upper'} bound"
						value={bound === 0 ? lowerInput : upperInput}
						oninput={(event) => {
							const text = (event.target as HTMLInputElement).value;
							if (bound === 0) lowerInput = text;
							else upperInput = text;
						}}
						onblur={submitInputs}
						onkeydown={(event) => onInputKey(event, bound as 0 | 1)}
					/>
					<span
						class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-[0.625rem] text-muted-foreground"
					>
						{toVeryCompact(Number(bound === 0 ? lowerInput : upperInput) || 0)}
					</span>
				</div>
			{/each}
		</div>
		<div class="z-10 w-full grow px-4">
			<RangeSlider bind:values {marks} {reversed} oninput={searchSoon}>
				{#snippet tooltip(position)}
					<div class="rounded-lg border border-primary bg-card p-2 text-sm shadow-lg">
						{toVeryCompact(currencyToOriginal(position, LOWEST, HIGHEST))}
					</div>
				{/snippet}
			</RangeSlider>
		</div>
	</div>
	<SortButtons {prefix} {search} field={kind} />
</div>
