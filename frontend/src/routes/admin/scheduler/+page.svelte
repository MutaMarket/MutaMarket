<script lang="ts">
	// The operations console: outgoing ESI telemetry (per-minute charts),
	// live database counts, and the background job board with run-now and
	// pause controls. Polls both admin endpoints so everything on the page
	// moves on its own. Styled as the app's HUD console (hud-panel frames,
	// mono hud-label group headings, EVE/UTC time).
	import { Button } from '$lib/components/ui/button';
	import JobCard from '$lib/components/job-card.svelte';
	import TelemetryChart, {
		type ChartMinute,
		type ChartSeries
	} from '$lib/components/telemetry-chart.svelte';
	import { JOB_CARDS, JOB_CARD_ORDER } from '$lib/job-cards';
	import type { PageProps } from './$types';
	import type { SchedulerStatus, TelemetrySnapshot } from './+page.server';

	let { data }: PageProps = $props();

	/** Live-status poll cadence. */
	const POLL_INTERVAL_MS = 5000;
	/** Minutes shown on the charts (the API keeps the same window). */
	const CHART_WINDOW_MINUTES = 60;

	// Endpoint series slots, validated for this surface (dataviz palette,
	// dark steps); the gray carries the folded "other" tail.
	const ENDPOINT_COLORS = ['#3987e5', '#d95926', '#199e70', '#c98500'];
	const OTHER_COLOR = '#898781';

	// Error classes wear the reserved status colors; this stack order
	// passes the adjacency gates on this surface.
	const ERROR_SERIES: ChartSeries[] = [
		{ key: 'client_errors', label: '4xx', color: '#ec835a' },
		{ key: 'server_errors', label: '5xx', color: '#d03b3b' },
		{ key: 'transport_errors', label: 'no response', color: '#fab219' }
	];

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let status = $state<SchedulerStatus>(data.status);
	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let telemetry = $state<TelemetrySnapshot>(data.telemetry);
	let now = $state(Math.floor(Date.now() / 1000));
	let notice = $state<string | null>(null);

	$effect(() => {
		const poll = setInterval(refresh, POLL_INTERVAL_MS);
		const tick = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
		return () => {
			clearInterval(poll);
			clearInterval(tick);
		};
	});

	async function refresh() {
		now = Math.floor(Date.now() / 1000);
		try {
			const [statusResponse, telemetryResponse] = await Promise.all([
				fetch('/api/admin/scheduler'),
				fetch('/api/admin/telemetry')
			]);
			if (statusResponse.ok) status = await statusResponse.json();
			if (telemetryResponse.ok) telemetry = await telemetryResponse.json();
		} catch {
			// Keep the last state while the API is unreachable.
		}
	}

	async function runNow(job: string) {
		notice = null;
		const response = await fetch(`/api/admin/scheduler/${job}/run`, { method: 'POST' });
		if (!response.ok) {
			const body: { message?: string } = await response.json().catch(() => ({}));
			notice = `${job}: ${body.message ?? 'Run failed to start.'}`;
		}
		await refresh();
	}

	async function setPaused(job: string, paused: boolean) {
		notice = null;
		const response = await fetch(`/api/admin/scheduler/${job}`, {
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ paused })
		});
		if (!response.ok) {
			const body: { message?: string } = await response.json().catch(() => ({}));
			notice = `${job}: ${body.message ?? 'Update failed.'}`;
		}
		await refresh();
	}

	// --- Telemetry shaping -------------------------------------------------

	// Sticky endpoint -> slot assignment: color follows the entity, so a
	// poll that reshuffles volumes never repaints existing series.
	let slotAssignment = $state<string[]>([]);

	const endpointTotals = $derived.by(() => {
		const totals = new Map<string, number>();
		for (const bucket of telemetry.buckets) {
			for (const [endpoint, counts] of Object.entries(bucket.endpoints)) {
				totals.set(endpoint, (totals.get(endpoint) ?? 0) + counts.requests);
			}
		}
		return totals;
	});

	$effect(() => {
		const present = [...endpointTotals.entries()].sort((a, b) => b[1] - a[1]);
		const kept = slotAssignment.filter((endpoint) => endpointTotals.has(endpoint));
		for (const [endpoint] of present) {
			if (kept.length >= ENDPOINT_COLORS.length) break;
			if (!kept.includes(endpoint)) kept.push(endpoint);
		}
		if (kept.join('\n') !== slotAssignment.join('\n')) {
			slotAssignment = kept;
		}
	});

	const hasOther = $derived(endpointTotals.size > slotAssignment.length);
	const requestSeries = $derived.by(() => {
		const series: ChartSeries[] = slotAssignment.map((endpoint, index) => ({
			key: endpoint,
			label: endpoint,
			color: ENDPOINT_COLORS[index]
		}));
		if (hasOther) {
			series.push({ key: '__other', label: 'other', color: OTHER_COLOR });
		}
		return series;
	});

	/** The fixed chart window: the last hour, gaps filled with zeros. */
	const chartMinutes = $derived.by(() => {
		const byMinute = new Map(telemetry.buckets.map((bucket) => [bucket.minute_start, bucket]));
		const currentMinute = Math.floor(now / 60) * 60;
		const minutes: { requests: ChartMinute; errors: ChartMinute }[] = [];

		for (let offset = CHART_WINDOW_MINUTES - 1; offset >= 0; offset -= 1) {
			const minuteStart = currentMinute - offset * 60;
			const bucket = byMinute.get(minuteStart);

			const requests: ChartMinute = { minuteStart, values: {} };
			const errors: ChartMinute = { minuteStart, values: {} };
			if (bucket) {
				let totalRequests = 0;
				let totalMs = 0;
				for (const [endpoint, counts] of Object.entries(bucket.endpoints)) {
					const key = slotAssignment.includes(endpoint) ? endpoint : '__other';
					requests.values[key] = (requests.values[key] ?? 0) + counts.requests;
					for (const errorClass of ERROR_SERIES) {
						errors.values[errorClass.key] =
							(errors.values[errorClass.key] ?? 0) +
							counts[errorClass.key as keyof typeof counts];
					}
					totalRequests += counts.requests;
					totalMs += counts.total_ms;
				}
				if (totalRequests > 0) {
					requests.detail = `avg ${Math.round(totalMs / totalRequests)} ms`;
				}
			}
			minutes.push({ requests, errors });
		}

		return minutes;
	});

	const hourTotals = $derived.by(() => {
		let requests = 0;
		let errors = 0;
		let totalMs = 0;
		for (const bucket of telemetry.buckets) {
			for (const counts of Object.values(bucket.endpoints)) {
				requests += counts.requests;
				errors += counts.client_errors + counts.server_errors + counts.transport_errors;
				totalMs += counts.total_ms;
			}
		}
		const busiest = [...endpointTotals.entries()].sort((a, b) => b[1] - a[1])[0] ?? null;
		return {
			requests,
			errors,
			averageMs: requests > 0 ? Math.round(totalMs / requests) : 0,
			busiest
		};
	});

	function compact(value: number): string {
		if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
		if (value >= 10_000) return `${(value / 1_000).toFixed(1)}K`;
		return value.toLocaleString('en-US');
	}

	const databaseTiles = $derived([
		['Modules', status.database.modules],
		['No estimate', status.database.modules_without_estimate],
		['Contracts', status.database.contracts],
		['Contract items', status.database.contract_items],
		['Characters', status.database.characters],
		['Users', status.database.users],
		['Assets', status.database.assets],
		['Public ownerships', status.database.public_ownerships],
		['Market days', status.database.market_history_days]
	] as const);
</script>

<svelte:head><title>Operations - MutaMarket</title></svelte:head>

<!-- Header rail: what the machinery is, and whether it is allowed to run. -->
<div class="mb-6 flex flex-wrap items-center gap-3">
	<div>
		<span class="hud-label">Operations // Background</span>
		<h1 class="mt-1 text-2xl font-bold">Console</h1>
	</div>
	<span class="ml-auto flex flex-wrap items-center gap-2">
		<span
			class="rounded-full border border-border px-2.5 py-0.5 text-xs {status.enabled
				? 'text-positive'
				: 'text-muted-foreground'}"
		>
			{status.enabled ? 'loops running' : 'loops disabled'}
		</span>
		{#if status.in_downtime}
			<span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-[#fab219]">
				EVE downtime
			</span>
		{/if}
	</span>
</div>

{#if notice}
	<p class="mb-4 text-sm text-negative">{notice}</p>
{/if}

<!-- Telemetry: the outgoing ESI stream, last hour. -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Telemetry // Outgoing ESI</h2>
	<div class="mb-3 grid grid-cols-2 gap-2 lg:grid-cols-4">
		<div class="hud-panel px-3 py-2.5">
			<div class="text-lg font-semibold text-foreground">{compact(hourTotals.requests)}</div>
			<div class="text-xs text-muted-foreground">Requests, last hour</div>
		</div>
		<div class="hud-panel px-3 py-2.5">
			<div class="text-lg font-semibold {hourTotals.errors > 0 ? 'text-negative' : 'text-foreground'}">
				{compact(hourTotals.errors)}
			</div>
			<div class="text-xs text-muted-foreground">Errors, last hour</div>
		</div>
		<div class="hud-panel px-3 py-2.5">
			<div class="text-lg font-semibold text-foreground">{hourTotals.averageMs} ms</div>
			<div class="text-xs text-muted-foreground">Average response</div>
		</div>
		<div class="hud-panel px-3 py-2.5">
			<div class="truncate font-mono text-sm font-semibold text-foreground">
				{hourTotals.busiest?.[0] ?? '—'}
			</div>
			<div class="text-xs text-muted-foreground">
				Busiest endpoint{hourTotals.busiest ? ` · ${compact(hourTotals.busiest[1])}` : ''}
			</div>
		</div>
	</div>
	<div class="grid gap-3 xl:grid-cols-2">
		<TelemetryChart
			title="Requests / minute"
			series={requestSeries}
			minutes={chartMinutes.map((minute) => minute.requests)}
			emptyText="No ESI requests in the last hour."
		/>
		<TelemetryChart
			title="Errors / minute"
			series={ERROR_SERIES}
			minutes={chartMinutes.map((minute) => minute.errors)}
			emptyText="No failed requests in the last hour."
		/>
	</div>
</section>

<!-- Database: what the background work is landing. -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Database // Ingested rows</h2>
	<div class="grid grid-cols-3 gap-2 sm:grid-cols-5 lg:grid-cols-9">
		{#each databaseTiles as [label, value] (label)}
			<div class="hud-panel px-3 py-2.5">
				<div class="text-sm font-semibold text-foreground tabular-nums">
					{value.toLocaleString('en-US')}
				</div>
				<div class="text-xs text-muted-foreground">{label}</div>
			</div>
		{/each}
	</div>
</section>

<!-- Jobs: one designed card per job, heavy movers first. -->
<section>
	<h2 class="hud-label mb-3">Jobs // Scheduler</h2>
	<div class="grid grid-flow-dense grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
		{#each JOB_CARD_ORDER as name (name)}
			{@const job = status.jobs.find((candidate) => candidate.name === name)}
			{#if job}
				<JobCard {job} config={JOB_CARDS[name]} {now} onRunNow={runNow} onSetPaused={setPaused} />
			{/if}
		{/each}
	</div>
</section>
