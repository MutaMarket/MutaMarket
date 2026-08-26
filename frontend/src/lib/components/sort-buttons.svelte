<script lang="ts">
	// The vertical sort trio of a slider row, mirroring the legacy
	// SortByButtons.vue: chevron up, a tiny SORT label, chevron down.
	// The active direction pulses primary; clicking it again unsorts.
	import { ChevronUp } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import { buildQueryPath, type UiSearch } from '$lib/query';

	let {
		prefix,
		search,
		field
	}: {
		prefix: string;
		search: UiSearch;
		/** `price`, `value` or an attribute name. */
		field: string;
	} = $props();

	const active = $derived(
		search.sort !== null && search.sort[0].toLowerCase() === field.toLowerCase()
	);
	const activeAsc = $derived(active && search.sort?.[1] === false);
	const activeDesc = $derived(active && search.sort?.[1] === true);

	function navigate(descending: boolean, isActive: boolean) {
		const next: UiSearch = {
			...search,
			sort: isActive ? null : [field, descending]
		};
		goto(buildQueryPath(prefix, next), { keepFocus: true, noScroll: true });
	}
</script>

<div class="grid place-items-center gap-2">
	<Button
		data-active={activeAsc}
		variant="ghost"
		size="icon"
		class="data-[active=true]:animate-pulse data-[active=true]:text-primary"
		title="Sort ascending"
		onclick={() => navigate(false, activeAsc)}
	>
		<ChevronUp class="size-4" />
	</Button>
	<span class="text-2xs leading-none font-medium uppercase">Sort</span>
	<Button
		data-active={activeDesc}
		variant="ghost"
		size="icon"
		class="data-[active=true]:animate-pulse data-[active=true]:text-primary"
		title="Sort descending"
		onclick={() => navigate(true, activeDesc)}
	>
		<ChevronUp class="size-4 rotate-180" />
	</Button>
</div>
