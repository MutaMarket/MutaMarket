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
		format
	}: {
		title: string;
		series: VitalSeries[];
		points: VitalPoint[];
		format: (value: number) => string;
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
	<div class="mb-2 flex items-baseline justify-between gap-3">
		<h3 class="hud-label">{title}</h3>
		{#if series.length > 1}
			<div class="flex gap-3">
				{#each series as s (s.key)}
					<span class="flex items-center gap-1.5 text-xs text-muted-foreground">
						<span class="h-2 w-2 rounded-[2px]" style="background: {s.color}"></span>
						{s.label}
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
