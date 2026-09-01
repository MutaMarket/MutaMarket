<script lang="ts">
	// One system-vital chart of the admin console: an area sparkline over
	// the toggled window with a grouped hover tooltip. Series arrive
	// pre-shaped (gauge values or derived rates) from the page. Each
	// series is its own mark pair so the two network directions overlay
	// instead of stacking, which a shared color channel would do.
	import { areaY, defineChart, lineY } from '@tanstack/charts';
	import { scaleLinear } from '@tanstack/charts/scales/linear';
	import { Chart } from '@tanstack/charts/svelte';
	import { tooltip } from '@tanstack/charts/tooltip';

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
		yDomain,
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

	/** The sparkline plot height. */
	const HEIGHT = 48;

	function rowsFor(s: VitalSeries) {
		return points.map((point) => ({
			at: point.at,
			label: s.label,
			value: point.values[s.key] ?? 0,
		}));
	}

	function timeLabel(at: number): string {
		const date = new Date(at * 1000);
		const hh = String(date.getUTCHours()).padStart(2, '0');
		const mm = String(date.getUTCMinutes()).padStart(2, '0');
		const day = `${String(date.getUTCDate()).padStart(2, '0')}.${String(date.getUTCMonth() + 1).padStart(2, '0')}`;
		return `${day} ${hh}:${mm}`;
	}

	const definition = $derived(
		defineChart({
			marks: series.flatMap((s) => {
				const rows = rowsFor(s);
				return [
					areaY(rows, {
						id: `${s.key}-area`,
						x: 'at',
						y: 'value',
						fill: s.color,
						fillOpacity: 0.12,
					}),
					lineY(rows, {
						id: `${s.key}-line`,
						x: 'at',
						y: 'value',
						stroke: s.color,
						strokeWidth: 1.5,
					}),
				];
			}),
			scales: {
				// No axes: the card's headline carries the current value and
				// the tooltip carries the readout.
				x: { scale: scaleLinear, axis: false },
				y: {
					scale: scaleLinear,
					axis: false,
					...(yDomain ? { viewport: { domain: yDomain } } : {}),
				},
			},
			focus: 'group-x',
			tooltip: {
				use: tooltip,
				formatGroup(focused) {
					const heading = `${timeLabel(Number(focused[0]?.xValue ?? 0))} EVE`;
					return [
						heading,
						...focused.map((point) => `${point.datum.label}: ${format(point.datum.value)}`),
					].join('\n');
				},
			},
		}),
	);
</script>

<div class="hud-frame p-4">
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
		<!-- The Pulse-style plot: its own tinted container, no axes. -->
		<div class="min-w-0 grow rounded-md bg-card-2/60">
			{#if points.length < 2}
				<div class="grid h-12 place-items-center text-xs text-muted-foreground">
					Not enough samples yet.
				</div>
			{:else}
				<Chart {definition} ariaLabel={title} height={HEIGHT} />
			{/if}
		</div>
	</div>
</div>
