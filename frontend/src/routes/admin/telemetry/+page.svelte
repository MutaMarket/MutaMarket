<script lang="ts">
	// Outgoing ESI telemetry: the last hour as per-minute columns, split
	// by endpoint for volume and by error class for failures. The window
	// is anchored on the minute, not on the poll, so the columns only
	// shift once a minute.
	import TelemetryChart from '$lib/components/telemetry-chart.svelte';
	import { apply, live, subscribe } from '$lib/admin-live.svelte';
	import {
		CHART_WINDOW_MINUTES,
		ERROR_SERIES,
		assignSlots,
		chartMinutes,
		endpointTotals,
		hourTotals,
		requestSeries
	} from '$lib/admin-telemetry';
	import { compact } from '$lib/admin-vitals';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	$effect(() => {
		apply(data.live);
	});
	$effect(() => subscribe(['telemetry']));

	const telemetry = $derived(
		live.telemetry ?? data.live.telemetry ?? { window_minutes: CHART_WINDOW_MINUTES, buckets: [] }
	);
	const totals = $derived(endpointTotals(telemetry.buckets));

	// Sticky slots: color follows the endpoint, so a poll that reshuffles
	// volumes never repaints an existing series.
	let slots = $state<string[]>([]);
	$effect(() => {
		const next = assignSlots(slots, totals);
		if (next.join('\n') !== slots.join('\n')) {
			slots = next;
		}
	});

	const series = $derived(requestSeries(slots, totals.size > slots.length));
	/** The window anchor: invalidates once per minute, not per poll. */
	const minuteNow = $derived(Math.floor(live.now / 60) * 60);
	const minutes = $derived(chartMinutes(telemetry, slots, minuteNow));
	const hour = $derived(hourTotals(telemetry.buckets, totals));
</script>

<svelte:head><title>Telemetry - Admin - MutaMarket</title></svelte:head>

<div class="grid gap-3 xl:grid-cols-2">
	<TelemetryChart
		title="Requests / minute"
		headline={compact(hour.requests)}
		headlineClass="text-primary"
		sub={`last hour · avg ${hour.averageMs} ms${hour.busiest ? ` · busiest ${hour.busiest[0]}` : ''}`}
		{series}
		minutes={minutes.requests}
		emptyText="No ESI requests in the last hour."
	/>
	<TelemetryChart
		title="Errors / minute"
		headline={compact(hour.errors)}
		headlineClass={hour.errors > 0 ? 'text-negative' : 'text-foreground'}
		sub="last hour"
		series={ERROR_SERIES}
		minutes={minutes.errors}
		emptyText="No failed requests in the last hour."
	/>
</div>

<!-- The endpoint roll-up under the charts: what the hour's traffic
     actually went to, beyond the four that hold a color slot. -->
<section class="mt-8">
	<h2 class="hud-label mb-3">Endpoints // Requests this hour</h2>
	<div class="hud-frame divide-y divide-border">
		{#each [...totals.entries()].sort((a, b) => b[1] - a[1]) as [endpoint, count] (endpoint)}
			<div class="flex items-center gap-3 px-4 py-2">
				<span
					class="size-2 shrink-0 rounded-full"
					style="background: {series.find((s) => s.key === endpoint)?.color ?? '#898781'}"
				></span>
				<span class="min-w-0 truncate font-mono text-xs">{endpoint}</span>
				<span class="ml-auto text-sm tabular-nums">{count.toLocaleString('en-US')}</span>
			</div>
		{:else}
			<p class="px-4 py-3 text-sm text-muted-foreground">
				No ESI requests in the last hour.
			</p>
		{/each}
	</div>
</section>
