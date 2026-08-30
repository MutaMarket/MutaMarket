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
	import EsiFailureDialog from '$lib/components/esi-failure-dialog.svelte';
	import {
		callerLabel,
		failureAt,
		failureClass,
		failureLabel,
		filterFailures
	} from '$lib/admin-failures';
	import { relativeTime } from '$lib/duration';
	import type { EsiFailureSummary } from '$lib/admin-types';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	$effect(() => {
		apply(data.live);
	});
	$effect(() => subscribe(['telemetry', 'failures']));

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

	// --- Captured failures -------------------------------------------------

	const section = $derived(live.failures ?? data.live.failures ?? null);
	/** The minute a chart column was clicked, if any. */
	let minute = $state<number | null>(null);
	/** Failures for a minute the live set does not reach. */
	let fetched = $state<EsiFailureSummary[] | null>(null);
	let inspecting = $state<EsiFailureSummary | null>(null);

	const captured = $derived(section?.captured ?? []);
	const shown = $derived(
		minute === null ? captured : (fetched ?? filterFailures(captured, { minute }))
	);
	/** Errors the telemetry counted in that minute, which is the number
	 * the sampler is measured against. */
	const countedInMinute = $derived.by(() => {
		if (minute === null) return null;
		const bucket = telemetry.buckets.find((entry) => entry.minute_start === minute);
		if (!bucket) return 0;
		return Object.values(bucket.endpoints).reduce(
			(sum, counts) =>
				sum + counts.client_errors + counts.server_errors + counts.transport_errors,
			0
		);
	});

	const CLASS_COLOR: Record<string, string> = {
		client_errors: '#ec835a',
		server_errors: '#d03b3b',
		transport_errors: '#fab219'
	};

	function inspectMinute(at: number) {
		minute = at;
		fetched = null;
		void (async () => {
			const response = await fetch(`/api/admin/esi-failures?minute=${at}`);
			if (response.ok) {
				const body: { failures: EsiFailureSummary[] } = await response.json();
				fetched = body.failures;
			}
		})();
	}

	function clearMinute() {
		minute = null;
		fetched = null;
	}

	function minuteLabel(at: number): string {
		const date = new Date(at * 1000);
		return `${String(date.getUTCHours()).padStart(2, '0')}:${String(date.getUTCMinutes()).padStart(2, '0')}`;
	}
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
		onSelect={inspectMinute}
	/>
</div>

<!-- The captured failures behind those counts. The chart counts every
     failure; this keeps a sample of them, so the two numbers differ on
     purpose during a burst. -->
<section class="mt-8">
	<div class="mb-3 flex flex-wrap items-center gap-3">
		<h2 class="hud-label">Failures // Captured</h2>
		{#if minute !== null}
			<button
				class="flex items-center gap-2 rounded-full border border-border px-2.5 py-0.5 text-xs text-foreground hover:bg-white/[0.04]"
				onclick={clearMinute}
			>
				{minuteLabel(minute)} EVE
				{#if countedInMinute !== null}
					· {countedInMinute} error{countedInMinute === 1 ? '' : 's'} · {shown.length} captured
				{/if}
				<span class="text-muted-foreground">✕</span>
			</button>
		{:else}
			<span class="text-xs text-muted-foreground">
				Click a column above to narrow to one minute.
			</span>
		{/if}
		{#if section}
			<span class="ml-auto text-xs text-muted-foreground">
				newest {section.keep.toLocaleString('en-US')} · kept {section.retention_days} days
			</span>
		{/if}
	</div>

	<div class="hud-frame divide-y divide-border">
		{#each shown as failure (failure.id)}
			<button
				class="flex w-full flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 text-left transition hover:bg-white/[0.03]"
				onclick={() => (inspecting = failure)}
			>
				<span
					class="size-2 shrink-0 rounded-full"
					style="background: {CLASS_COLOR[failureClass(failure)]}"
				></span>
				<span class="shrink-0 font-mono text-xs tabular-nums">{failureLabel(failure)}</span>
				<span class="min-w-0 truncate text-sm">
					<span class="text-muted-foreground">{failure.method}</span>
					{failure.endpoint}
				</span>
				{#if failure.error_message}
					<span class="min-w-0 truncate text-xs text-muted-foreground">
						{failure.error_message}
					</span>
				{/if}
				<span class="ml-auto shrink-0 text-xs text-muted-foreground">
					{callerLabel(failure) ?? '—'} · {relativeTime(failureAt(failure) - live.now)}
				</span>
			</button>
		{:else}
			<p class="px-4 py-3 text-sm text-muted-foreground">
				{minute === null
					? 'No failed ESI requests captured.'
					: 'Nothing captured in that minute.'}
			</p>
		{/each}
	</div>
</section>

<EsiFailureDialog bind:failure={inspecting} now={live.now} />

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
