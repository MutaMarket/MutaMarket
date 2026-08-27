<script lang="ts">
	// One system-vital chart of the admin dashboard: a LayerChart area
	// line over the toggled window, accent-colored, with the shadcn
	// hover tooltip. Series arrive pre-shaped (gauge values or derived
	// rates) from the page.
	import { AreaChart } from 'layerchart';
	import * as Chart from '$lib/components/ui/chart';

	export interface VitalSeries {
		key: string;
		label: string;
		color: string;
	}

	export interface VitalPoint {
		at: number;
		values: Record<string, number>;
	}

	let {
		title,
		series,
		points,
		format,
		headline,
		sub
	}: {
		title: string;
		series: VitalSeries[];
		points: VitalPoint[];
		format: (value: number) => string;
		/** The live value shown as the card's stat. */
		headline?: string;
		sub?: string;
	} = $props();

	const chartConfig = $derived(
		Object.fromEntries(
			series.map((s) => [s.key, { label: s.label, color: s.color }])
		) satisfies Chart.ChartConfig
	);

	const rows = $derived(
		points.map((point) => ({
			at: point.at,
			...Object.fromEntries(series.map((s) => [s.key, point.values[s.key] ?? 0]))
		}))
	);

	const chartSeries = $derived(
		series.map((s) => ({
			key: s.key,
			label: s.label,
			color: s.color,
			props: { fillOpacity: 0.12, line: { strokeWidth: 1.5 } }
		}))
	);

	function timeLabel(at: number): string {
		const date = new Date(at * 1000);
		const hh = String(date.getUTCHours()).padStart(2, '0');
		const mm = String(date.getUTCMinutes()).padStart(2, '0');
		const day = `${String(date.getUTCDate()).padStart(2, '0')}.${String(date.getUTCMonth() + 1).padStart(2, '0')}`;
		return `${day} ${hh}:${mm}`;
	}
</script>

<div class="hud-panel p-4">
	<div class="mb-2 flex items-start justify-between gap-3">
		<div class="min-w-0">
			<h3 class="hud-label">{title}</h3>
			{#if headline}
				<div class="mt-1 truncate text-lg font-semibold text-foreground tabular-nums">
					{headline}
				</div>
			{/if}
			{#if sub}
				<div class="truncate text-xs text-muted-foreground">{sub}</div>
			{/if}
		</div>
		{#if series.length > 1}
			<div class="flex min-w-0 flex-wrap justify-end gap-x-3 gap-y-1">
				{#each series as s (s.key)}
					<span class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
						<span class="size-2 shrink-0 rounded-[2px]" style="background: {s.color}"></span>
						<span class="truncate">{s.label}</span>
					</span>
				{/each}
			</div>
		{/if}
	</div>
	{#if rows.length < 2}
		<div class="grid h-[120px] place-items-center text-xs text-muted-foreground">
			Not enough samples yet.
		</div>
	{:else}
		<Chart.Container config={chartConfig} class="h-[120px] w-full">
			<AreaChart
				data={rows}
				x="at"
				series={chartSeries}
				axis="y"
				points={false}
				props={{
					yAxis: { format }
				}}
			>
				{#snippet tooltip()}
					<Chart.Tooltip labelFormatter={(at: number) => `${timeLabel(at)} EVE`} />
				{/snippet}
			</AreaChart>
		</Chart.Container>
	{/if}
</div>
