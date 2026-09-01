<script lang="ts">
	// The page header strip (no legacy counterpart, replacing the bare
	// h1 titles): identity on the left — an icon or portrait, the title,
	// a muted subtitle — and a right-aligned readout of the resource's
	// key numbers, hairline-separated, with the headline stat carrying an
	// accent color. Deliberately unboxed: the filter band below is
	// already a heavy panel.
	import type { Snippet } from 'svelte';

	export interface HeaderStat {
		label: string;
		value: string;
		accent?: 'primary' | 'gold' | 'diamond';
	}

	let {
		title,
		subtitle = null,
		stats = [],
		icon,
		actions,
	}: {
		title: string;
		subtitle?: string | null;
		stats?: HeaderStat[];
		icon?: Snippet;
		/** Page-level actions rendered right of the stats. */
		actions?: Snippet;
	} = $props();

	const accentClass = (accent: HeaderStat['accent']) => {
		switch (accent) {
			case 'primary':
				return 'text-primary';
			case 'gold':
				return 'text-gold';
			case 'diamond':
				return 'text-diamond';
			default:
				return 'text-foreground';
		}
	};
</script>

<header class="mb-4 flex flex-wrap items-center justify-between gap-x-8 gap-y-3">
	<div class="flex min-w-0 items-center gap-3">
		{#if icon}
			{@render icon()}
		{/if}
		<div class="min-w-0">
			<h1 class="truncate text-xl leading-tight font-semibold tracking-tight">{title}</h1>
			{#if subtitle}
				<p class="truncate text-sm text-muted-foreground">{subtitle}</p>
			{/if}
		</div>
	</div>
	<div class="flex items-center gap-6">
		{#if stats.length > 0}
			<dl class="flex items-center">
				{#each stats as stat, index (stat.label)}
					<div
						class="flex flex-col items-end gap-1 {index > 0
							? 'ml-6 border-l border-border pl-6'
							: ''}"
					>
						<dt class="hud-label whitespace-nowrap">{stat.label}</dt>
						<dd class="text-lg leading-none font-semibold tabular-nums {accentClass(stat.accent)}">
							{stat.value}
						</dd>
					</div>
				{/each}
			</dl>
		{/if}
		{#if actions}
			{@render actions()}
		{/if}
	</div>
</header>
