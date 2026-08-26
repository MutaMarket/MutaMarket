<script lang="ts">
	// A stacked-column minute chart for the admin telemetry, on the
	// shadcn chart stack (LayerChart under Chart.Container/Tooltip): one
	// column per minute over the window, stacked by series, hover
	// tooltip, legend. The props API is unchanged from the hand-rolled
	// predecessor.
	import { scaleBand } from 'd3-scale';
	import { BarChart } from 'layerchart';
	import * as Chart from '$lib/components/ui/chart';

	export interface ChartSeries {
		key: string;
		label: string;
		color: string;
	}

	export interface ChartMinute {
		/** Unix seconds of the minute's start. */
		minuteStart: number;
		/** Value per series key. */
		values: Record<string, number>;
		/** Extra tooltip line (e.g. average latency), if any. */
		detail?: string;
	}

	let {
		title,
		series,
		minutes,
		emptyText
	}: {
		title: string;
		series: ChartSeries[];
		minutes: ChartMinute[];
		emptyText: string;
	} = $props();

	const hasData = $derived(
		minutes.some((minute) => series.some((s) => (minute.values[s.key] ?? 0) > 0))
	);

	// Endpoint keys carry slashes, which cannot become CSS variable
	// names, so colors are passed to the series directly and the config
	// only supplies tooltip labels.
	const chartConfig = $derived(
		Object.fromEntries(
			series.map((s) => [s.key, { label: s.label, color: s.color }])
		) satisfies Chart.ChartConfig
	);

	const rows = $derived(
		minutes.map((minute) => ({
			minuteStart: minute.minuteStart,
			detail: minute.detail,
			...Object.fromEntries(series.map((s) => [s.key, minute.values[s.key] ?? 0]))
		}))
	);

	const chartSeries = $derived(
		series.map((s, index) => ({
			key: s.key,
			label: s.label,
			color: s.color,
			// Only the top stack segment wears the rounded data-end.
			props: { rounded: index === series.length - 1 ? ('top' as const) : ('none' as const) }
		}))
	);

	function timeLabel(minuteStart: number): string {
		const date = new Date(minuteStart * 1000);
		const hh = String(date.getUTCHours()).padStart(2, '0');
		const mm = String(date.getUTCMinutes()).padStart(2, '0');
		return `${hh}:${mm}`;
	}

	/** Ticks every 15 minutes, aligned to the wall clock. */
	const timeTicks = $derived(
		minutes.map((minute) => minute.minuteStart).filter((start) => start % 900 === 0)
	);
</script>

<div class="hud-panel p-4">
	<div class="mb-2 flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
		<h2 class="hud-label whitespace-nowrap">{title}</h2>
		{#if series.length > 1}
			<div class="flex flex-wrap gap-3">
				{#each series as s (s.key)}
					<span class="flex items-center gap-1.5 text-xs text-muted-foreground">
						<span class="h-2 w-2 rounded-[2px]" style="background: {s.color}"></span>
						{s.label}
					</span>
				{/each}
			</div>
		{/if}
	</div>

	{#if !hasData}
		<div class="grid h-[180px] place-items-center text-xs text-muted-foreground">
			{emptyText}
		</div>
	{:else}
		<Chart.Container config={chartConfig} class="h-[180px] w-full">
			<BarChart
				data={rows}
				x="minuteStart"
				xScale={scaleBand().padding(0.25)}
				series={chartSeries}
				seriesLayout="stack"
				axis={true}
				props={{
					bars: { stroke: 'none' },
					highlight: {
						area: { fill: 'color-mix(in oklab, var(--color-foreground) 6%, transparent)' }
					},
					xAxis: { format: timeLabel, ticks: timeTicks },
					yAxis: { format: (value: number) => value.toLocaleString('en-US') }
				}}
			>
				{#snippet tooltip()}
					<Chart.Tooltip labelFormatter={(value: number) => `${timeLabel(value)} EVE`} />
				{/snippet}
			</BarChart>
		</Chart.Container>
	{/if}
</div>
