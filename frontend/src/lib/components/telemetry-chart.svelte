<script lang="ts">
	// A stacked-column minute chart for the admin telemetry: one column per
	// minute over the window, stacked by series, with per-column hover
	// tooltips and a legend. Marks follow the dataviz specs: thin columns,
	// 2px surface gaps between segments, rounded data-end on the top
	// segment, hairline gridlines, mono muted ticks.

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

	const WIDTH = 560;
	const HEIGHT = 180;
	const PLOT_LEFT = 34;
	const PLOT_BOTTOM = HEIGHT - 18;
	const PLOT_TOP = 8;
	/** Surface gap between stack segments and columns, per the mark spec. */
	const GAP = 2;

	let hovered = $state<number | null>(null);

	const totals = $derived(
		minutes.map((minute) => series.reduce((sum, s) => sum + (minute.values[s.key] ?? 0), 0))
	);
	const maxTotal = $derived(Math.max(...totals, 0));
	const hasData = $derived(maxTotal > 0);

	/** A clean axis ceiling: 1/2/5 x 10^n at or above the max. */
	const ceiling = $derived.by(() => {
		if (maxTotal <= 4) return 4;
		const magnitude = Math.pow(10, Math.floor(Math.log10(maxTotal)));
		for (const step of [1, 2, 4, 5, 10]) {
			if (step * magnitude >= maxTotal) return step * magnitude;
		}
		return 10 * magnitude;
	});

	const slot = $derived((WIDTH - PLOT_LEFT) / Math.max(minutes.length, 1));
	const columnWidth = $derived(Math.max(Math.min(slot - GAP, 24), 2));
	const plotHeight = $derived(PLOT_BOTTOM - PLOT_TOP);

	function x(index: number): number {
		return PLOT_LEFT + index * slot + (slot - columnWidth) / 2;
	}

	function segmentHeight(value: number): number {
		return (value / ceiling) * plotHeight;
	}

	/** Stack segments bottom-up with surface gaps, top segment rounded. */
	function segments(minute: ChartMinute) {
		const parts: { key: string; color: string; y: number; height: number }[] = [];
		let baseline = PLOT_BOTTOM;
		for (const s of series) {
			const value = minute.values[s.key] ?? 0;
			if (value <= 0) continue;
			const height = Math.max(segmentHeight(value) - (parts.length > 0 ? GAP : 0), 1);
			baseline -= parts.length > 0 ? GAP : 0;
			parts.push({ key: s.key, color: s.color, y: baseline - height, height });
			baseline -= height;
		}
		return parts;
	}

	const gridValues = $derived([0.5, 1].map((fraction) => ceiling * fraction));

	function timeLabel(minuteStart: number): string {
		const date = new Date(minuteStart * 1000);
		const hh = String(date.getUTCHours()).padStart(2, '0');
		const mm = String(date.getUTCMinutes()).padStart(2, '0');
		return `${hh}:${mm}`;
	}

	/** Ticks every 15 minutes, aligned to the wall clock. */
	const timeTicks = $derived(
		minutes
			.map((minute, index) => ({ minute, index }))
			.filter(({ minute }) => minute.minuteStart % 900 === 0)
	);

	const hoveredMinute = $derived(hovered === null ? null : minutes[hovered]);
	// Keep the tooltip inside the plot: flip sides past the middle.
	const tooltipLeft = $derived(
		hovered === null ? 0 : Math.min(Math.max((x(hovered) / WIDTH) * 100, 6), 66)
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
		<div class="relative">
			<svg
				viewBox="0 0 {WIDTH} {HEIGHT}"
				class="block w-full"
				role="img"
				aria-label={title}
				onpointerleave={() => (hovered = null)}
			>
				<!-- Hairline gridlines with clean tick values. -->
				{#each gridValues as value (value)}
					<line
						x1={PLOT_LEFT}
						x2={WIDTH}
						y1={PLOT_BOTTOM - segmentHeight(value)}
						y2={PLOT_BOTTOM - segmentHeight(value)}
						stroke="var(--color-border)"
						stroke-width="1"
					/>
					<text
						x={PLOT_LEFT - 5}
						y={PLOT_BOTTOM - segmentHeight(value) + 3}
						text-anchor="end"
						class="chart-tick"
					>
						{value.toLocaleString('en-US')}
					</text>
				{/each}
				<line
					x1={PLOT_LEFT}
					x2={WIDTH}
					y1={PLOT_BOTTOM}
					y2={PLOT_BOTTOM}
					stroke="var(--color-border)"
					stroke-width="1"
				/>

				{#each minutes as minute, index (minute.minuteStart)}
					{#if totals[index] > 0}
						{#each segments(minute) as segment, si (segment.key)}
							<rect
								x={x(index)}
								y={segment.y}
								width={columnWidth}
								height={segment.height}
								fill={segment.color}
								opacity={hovered === null || hovered === index ? 1 : 0.45}
								rx={si === segments(minute).length - 1
									? Math.min(2, columnWidth / 2)
									: 0}
							/>
						{/each}
					{/if}
					<!-- Full-height hit target: the reader aims at a minute. -->
					<rect
						x={PLOT_LEFT + index * slot}
						y={PLOT_TOP}
						width={slot}
						height={plotHeight}
						fill="transparent"
						role="presentation"
						onpointerenter={() => (hovered = index)}
					/>
				{/each}

				{#each timeTicks as tick (tick.minute.minuteStart)}
					<text x={x(tick.index) + columnWidth / 2} y={HEIGHT - 4} text-anchor="middle" class="chart-tick">
						{timeLabel(tick.minute.minuteStart)}
					</text>
				{/each}
			</svg>

			{#if hoveredMinute !== null && hovered !== null}
				<div
					class="pointer-events-none absolute top-1 rounded-md border border-border bg-popover px-2.5 py-1.5 text-xs shadow-md"
					style="left: {tooltipLeft}%"
				>
					<div class="hud-label mb-1">{timeLabel(hoveredMinute.minuteStart)} EVE</div>
					{#each series as s (s.key)}
						{#if (hoveredMinute.values[s.key] ?? 0) > 0}
							<div class="flex items-center gap-1.5">
								<span class="h-[2px] w-3" style="background: {s.color}"></span>
								<span class="font-semibold text-foreground tabular-nums">
									{(hoveredMinute.values[s.key] ?? 0).toLocaleString('en-US')}
								</span>
								<span class="text-muted-foreground">{s.label}</span>
							</div>
						{/if}
					{/each}
					{#if hoveredMinute.detail}
						<div class="mt-1 text-muted-foreground">{hoveredMinute.detail}</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.chart-tick {
		font-family: var(--font-mono);
		font-size: 9px;
		fill: var(--color-muted-foreground);
	}
</style>
