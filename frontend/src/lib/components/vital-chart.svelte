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
		sub,
		yDomain
	}: {
		title: string;
		series: VitalSeries[];
		points: VitalPoint[];
		format: (value: number) => string;
		/** The live value shown as the card's stat. */
		headline?: string;
		sub?: string;
		/** Fixed y-range (e.g. [0, 100] for utilization charts). */
		yDomain?: [number, number];
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
	<h3 class="hud-label">{title}</h3>
	<div class="mt-2 flex items-center gap-3">
		<div class="w-24 shrink-0">
			<div class="truncate text-xl font-semibold text-foreground tabular-nums">
				{headline ?? '—'}
			</div>
			{#if sub}
				<div class="truncate text-xs text-muted-foreground" title={sub}>{sub}</div>
			{/if}
		</div>
		<!-- The Pulse-style plot: its own tinted container, no axes; the
		     hover tooltip carries the readout. -->
		<div class="min-w-0 grow rounded-md bg-card-2/60">
			{#if rows.length < 2}
				<div class="grid h-12 place-items-center text-xs text-muted-foreground">
					Not enough samples yet.
				</div>
			{:else}
				<Chart.Container config={chartConfig} class="h-12 w-full overflow-hidden">
					<AreaChart
						data={rows}
						x="at"
						series={chartSeries}
						axis={false}
						grid={false}
						points={false}
						yDomain={yDomain ?? null}
						padding={{ left: 4, right: 4, top: 6, bottom: 6 }}
					>
						{#snippet tooltip()}
							<Chart.Tooltip labelFormatter={(at: number) => `${timeLabel(at)} EVE`}>
								{#snippet formatter({ value, name, item })}
									<span
										class="size-2.5 shrink-0 rounded-[2px]"
										style="background: {item.color}"
									></span>
									<span class="flex flex-1 justify-between gap-3 leading-none">
										<span class="text-muted-foreground">{name}</span>
										<span class="font-mono font-medium tabular-nums">
											{format(Number(value))}
										</span>
									</span>
								{/snippet}
							</Chart.Tooltip>
						{/snippet}
					</AreaChart>
				</Chart.Container>
			{/if}
		</div>
	</div>
</div>
