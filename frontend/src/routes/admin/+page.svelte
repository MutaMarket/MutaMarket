<script lang="ts">
	// The operations console: outgoing ESI telemetry (per-minute charts),
	// live database counts, and the background job board with run-now and
	// pause controls. Polls both admin endpoints so everything on the page
	// moves on its own. Styled as the app's HUD console (hud-panel frames,
	// mono hud-label group headings, EVE/UTC time).
	import JobCard from '$lib/components/job-card.svelte';
	import VitalChart, {
		type VitalPoint,
		type VitalSeries
	} from '$lib/components/vital-chart.svelte';
	import TelemetryChart, {
		type ChartMinute,
		type ChartSeries
	} from '$lib/components/telemetry-chart.svelte';
	import { JOB_CARDS, JOB_CARD_ORDER } from '$lib/job-cards';
	import type { PageProps } from './$types';
	import type {
		MetricsHistory,
		SchedulerStatus,
		SystemStats,
		TelemetrySnapshot
	} from '$lib/admin-types';

	let { data }: PageProps = $props();

	/** Live-status poll cadence. */
	const POLL_INTERVAL_MS = 5000;
	/** Minutes shown on the charts (the API keeps the same window). */
	const CHART_WINDOW_MINUTES = 60;

	// Endpoint series slots: the accent leads, then distinct partner
	// hues; the gray carries the folded "other" tail.
	const ENDPOINT_COLORS = ['#a3e635', '#22d3ee', '#a78bfa', '#f59e0b'];
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
	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let system = $state<SystemStats>(data.system);
	// The previous system sample, for cpu/network rates between polls.
	let previousSystem = $state<{ at: number; stats: SystemStats } | null>(null);
	let systemAt = $state(Date.now() / 1000);
	let now = $state(Math.floor(Date.now() / 1000));
	let notice = $state<string | null>(null);

	// One cadence for everything: the poll also advances `now`. A
	// separate one-second ticker used to invalidate every chart and job
	// card each second, which made the page crawl.
	$effect(() => {
		const poll = setInterval(refresh, POLL_INTERVAL_MS);
		return () => clearInterval(poll);
	});

	// Poll payloads land as fresh objects every five seconds even when
	// nothing changed, and every assignment re-renders a dozen charts.
	// Comparing the serialized payload makes unchanged polls free.
	let lastStatusText = '';
	let lastTelemetryText = '';
	let lastSystemText = '';

	async function refresh() {
		now = Math.floor(Date.now() / 1000);
		try {
			const [statusResponse, telemetryResponse, systemResponse] = await Promise.all([
				fetch('/api/admin/scheduler'),
				fetch('/api/admin/telemetry'),
				fetch('/api/admin/system')
			]);
			if (statusResponse.ok) {
				const next = await statusResponse.json();
				const text = JSON.stringify(next);
				if (text !== lastStatusText) {
					lastStatusText = text;
					status = next;
				}
			}
			if (telemetryResponse.ok) {
				const next = await telemetryResponse.json();
				const text = JSON.stringify(next);
				if (text !== lastTelemetryText) {
					lastTelemetryText = text;
					telemetry = next;
				}
			}
			if (systemResponse.ok) {
				const next = await systemResponse.json();
				const text = JSON.stringify(next);
				if (text !== lastSystemText) {
					lastSystemText = text;
					previousSystem = { at: systemAt, stats: system };
					system = next;
					systemAt = Date.now() / 1000;
				}
			}
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

	/** The chart window anchor: invalidates once per minute, not per poll. */
	const minuteNow = $derived(Math.floor(now / 60) * 60);

	/** The fixed chart window: the last hour, gaps filled with zeros. */
	const chartMinutes = $derived.by(() => {
		const byMinute = new Map(telemetry.buckets.map((bucket) => [bucket.minute_start, bucket]));
		const currentMinute = minuteNow;
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

	function formatBytes(value: number | null): string {
		if (value === null) return '—';
		if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(1)} GB`;
		if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
		if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
		return `${value} B`;
	}

	function formatUptime(seconds: number | null): string {
		if (seconds === null) return '—';
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
		if (seconds < 86_400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
		return `${Math.floor(seconds / 86_400)}d ${Math.floor((seconds % 86_400) / 3600)}h`;
	}

	/** CPU load between the last two samples, in percent of one core. */
	const cpuPercent = $derived.by(() => {
		if (previousSystem === null) return null;
		const { at, stats } = previousSystem;
		if (system.cpu_seconds === null || stats.cpu_seconds === null) return null;
		const wall = systemAt - at;
		if (wall <= 0) return null;
		return Math.max(((system.cpu_seconds - stats.cpu_seconds) / wall) * 100, 0);
	});

	/** Bytes per second between the last two samples. */
	const networkRates = $derived.by(() => {
		if (previousSystem === null) return null;
		const { at, stats } = previousSystem;
		if (
			system.network_rx_bytes === null ||
			stats.network_rx_bytes === null ||
			system.network_tx_bytes === null ||
			stats.network_tx_bytes === null
		) {
			return null;
		}
		const wall = systemAt - at;
		if (wall <= 0) return null;
		return {
			rx: Math.max((system.network_rx_bytes - stats.network_rx_bytes) / wall, 0),
			tx: Math.max((system.network_tx_bytes - stats.network_tx_bytes) / wall, 0)
		};
	});

	// --- Vitals history ----------------------------------------------------

	/** The timeframe toggle of the vitals charts. */
	const HISTORY_WINDOWS = ['24h', '3d', '7d'] as const;
	let historyWindow = $state<(typeof HISTORY_WINDOWS)[number]>('24h');
	let history = $state<MetricsHistory | null>(null);

	$effect(() => {
		const window = historyWindow;
		void (async () => {
			const response = await fetch(`/api/admin/metrics?window=${window}`);
			if (response.ok) {
				history = await response.json();
			}
		})();
	});

	const ACCENT = '#a3e635';
	const PARTNER = '#22d3ee';

	/** A gauge series as chart points. */
	function gaugePoints(metric: string): VitalPoint[] {
		return (history?.series[metric] ?? []).map((sample) => ({
			at: sample.taken_at,
			values: { value: sample.value }
		}));
	}

	/** Counter series as per-bucket rates (clamped at zero, which also
	 * absorbs restarts resetting the totals). */
	function ratePoints(metrics: Record<string, string>): VitalPoint[] {
		if (history === null) return [];
		const step = history.step_seconds;
		const perKey = Object.entries(metrics).map(([key, metric]) => {
			const series = history?.series[metric] ?? [];
			return series.slice(1).map((sample, index) => ({
				at: sample.taken_at,
				key,
				value: Math.max((sample.value - series[index].value) / step, 0)
			}));
		});
		const byAt = new Map<number, VitalPoint>();
		for (const series of perKey) {
			for (const point of series) {
				const existing = byAt.get(point.at) ?? { at: point.at, values: {} };
				existing.values[point.key] = point.value;
				byAt.set(point.at, existing);
			}
		}
		return [...byAt.values()].sort((a, b) => a.at - b.at);
	}

	const LOAD_SERIES: VitalSeries[] = [{ key: 'value', label: 'load', color: ACCENT }];
	const USED_SERIES: VitalSeries[] = [{ key: 'value', label: 'used', color: ACCENT }];
	const SIZE_SERIES: VitalSeries[] = [{ key: 'value', label: 'size', color: ACCENT }];
	const NETWORK_SERIES: VitalSeries[] = [
		{ key: 'rx', label: 'in', color: ACCENT },
		{ key: 'tx', label: 'out', color: PARTNER }
	];

	/** cpu_seconds deltas as percent of the machine (all cores). */
	const cpuPoints = $derived(
		ratePoints({ value: 'cpu_seconds' }).map((point) => ({
			at: point.at,
			values: { value: ((point.values.value ?? 0) * 100) / (system.cpu_cores ?? 1) }
		}))
	);
	const networkPoints = $derived(ratePoints({ rx: 'network_rx_bytes', tx: 'network_tx_bytes' }));

	/** A gauge series as utilization percent of a fixed capacity. */
	function percentPoints(metric: string, capacity: number | null): VitalPoint[] {
		if (capacity === null || capacity <= 0) return [];
		return gaugePoints(metric).map((point) => ({
			at: point.at,
			values: { value: ((point.values.value ?? 0) * 100) / capacity }
		}));
	}
	/** Utilization capacity: the cgroup limit, else the machine total. */
	const memoryCapacity = $derived(system.memory_limit_bytes ?? system.memory_total_bytes);
	const memoryPoints = $derived(percentPoints('memory_bytes', memoryCapacity));
	const diskPoints = $derived(percentPoints('disk_used_bytes', system.disk_total_bytes));

	const memoryPercent = $derived.by(() => {
		const current = system.memory_current_bytes ?? system.memory_rss_bytes;
		if (current === null || memoryCapacity === null) return null;
		return (current * 100) / memoryCapacity;
	});
	const diskPercent = $derived.by(() => {
		if (system.disk_used_bytes === null || system.disk_total_bytes === null) return null;
		return (system.disk_used_bytes * 100) / system.disk_total_bytes;
	});
	const cpuUtilization = $derived(
		cpuPercent === null ? null : cpuPercent / (system.cpu_cores ?? 1)
	);

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

<svelte:head><title>Admin - MutaMarket</title></svelte:head>

<!-- Header rail: what the machinery is, and whether it is allowed to run. -->
<div class="mb-6 flex flex-wrap items-center gap-3">
	<div>
		<span class="hud-label">Admin // Operations</span>
		<h1 class="mt-1 text-2xl font-bold">Dashboard</h1>
	</div>
	<span class="ml-auto flex flex-wrap items-center gap-2">
		<a
			class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground hover:text-foreground"
			href="/admin/advertisements"
		>
			advertisements
		</a>
		<span
			class="rounded-full border border-border px-2.5 py-0.5 text-xs {status.enabled
				? 'text-positive'
				: 'text-muted-foreground'}"
		>
			{status.enabled ? 'loops running' : 'loops disabled'}
		</span>
		<span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground">
			up {formatUptime(system.uptime_seconds)}
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

<!-- Service character: who the background features act through
     (structure resolution, donation processing when it lands). -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Service // Character</h2>
	<div class="hud-panel flex flex-wrap items-center gap-4 p-4">
		{#if data.service.character}
			<img
				src="https://images.evetech.net/characters/{data.service.character.id}/portrait?size=64"
				alt=""
				class="size-12 rounded-lg"
			/>
			<div>
				<div class="font-medium">
					{data.service.character.name ?? `Character ${data.service.character.id}`}
				</div>
				<div class="text-xs text-muted-foreground">
					{data.service.source === 'env'
						? 'from EVE_STRUCTURES_CHARACTER_ID (authorize to manage here)'
						: `${data.service.character.scopes.length} scopes authorized`}
					· resolves structures, will process donations
				</div>
			</div>
		{:else}
			<div class="text-sm text-muted-foreground">
				No service character yet. Background features that need ESI auth (structure
				resolution, donation processing) stay idle until one is authorized.
			</div>
		{/if}
		<a
			href="/eve/admin"
			rel="external"
			class="ml-auto rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground transition hover:brightness-110"
		>
			{data.service.character ? 'Re-authorize' : 'Authorize service character'}
		</a>
	</div>
</section>

<!-- System: live vitals with their recorded history, one card each. -->
<section class="mb-8">
	<div class="mb-3 flex items-center gap-4">
		<h2 class="hud-label">System // Container</h2>
		<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
			{#each HISTORY_WINDOWS as window (window)}
				<button
					type="button"
					class="flex h-6 items-center rounded-[5px] px-2.5 text-xs transition-colors {historyWindow ===
					window
						? 'bg-primary text-primary-foreground'
						: 'text-muted-foreground hover:text-foreground'}"
					onclick={() => (historyWindow = window)}
				>
					{window}
				</button>
			{/each}
		</div>
	</div>
	<div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-5">
		<VitalChart
			title="CPU"
			headline={cpuUtilization === null ? '—' : `${cpuUtilization.toFixed(0)}%`}
			sub={system.cpu_cores !== null ? `of ${system.cpu_cores} cores` : undefined}
			series={LOAD_SERIES}
			points={cpuPoints}
			yDomain={[0, 100]}
			format={(value) => `${value.toFixed(0)}%`}
		/>
		<!-- Capacity: the cgroup limit, else the machine's total memory;
		     without either (non-Linux) the chart falls back to bytes. -->
		{#if memoryCapacity !== null}
			<VitalChart
				title="Memory"
				headline={memoryPercent === null ? '—' : `${memoryPercent.toFixed(0)}%`}
				sub={`${formatBytes(system.memory_current_bytes ?? system.memory_rss_bytes)} of ${formatBytes(memoryCapacity)}`}
				series={USED_SERIES}
				points={memoryPoints}
				yDomain={[0, 100]}
				format={(value) => `${value.toFixed(0)}%`}
			/>
		{:else}
			<VitalChart
				title="Memory"
				headline={formatBytes(system.memory_current_bytes ?? system.memory_rss_bytes)}
				series={USED_SERIES}
				points={gaugePoints('memory_bytes')}
				format={(value) => formatBytes(Math.round(value))}
			/>
		{/if}
		<VitalChart
			title="Storage"
			headline={diskPercent === null ? '—' : `${diskPercent.toFixed(0)}%`}
			sub={system.disk_used_bytes !== null && system.disk_total_bytes !== null
				? `${formatBytes(system.disk_total_bytes - system.disk_used_bytes)} free of ${formatBytes(system.disk_total_bytes)}`
				: undefined}
			series={USED_SERIES}
			points={diskPoints}
			yDomain={[0, 100]}
			format={(value) => `${value.toFixed(0)}%`}
		/>
		<VitalChart
			title="Network"
			headline={networkRates === null
				? '—'
				: `${formatBytes(Math.round(networkRates.rx))}/s · ${formatBytes(Math.round(networkRates.tx))}/s`}
			sub="in · out"
			series={NETWORK_SERIES}
			points={networkPoints}
			format={(value) => formatBytes(Math.round(value))}
		/>
		<VitalChart
			title="Database"
			headline={formatBytes(system.database_size_bytes)}
			series={SIZE_SERIES}
			points={gaugePoints('database_size_bytes')}
			format={(value) => formatBytes(Math.round(value))}
		/>
	</div>
</section>

<!-- Telemetry: the outgoing ESI stream, last hour. -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Telemetry // Outgoing ESI</h2>
	<div class="grid gap-3 xl:grid-cols-2">
		<TelemetryChart
			title="Requests / minute"
			headline={compact(hourTotals.requests)}
			headlineClass="text-primary"
			sub={`last hour · avg ${hourTotals.averageMs} ms${hourTotals.busiest ? ` · busiest ${hourTotals.busiest[0]}` : ''}`}
			series={requestSeries}
			minutes={chartMinutes.map((minute) => minute.requests)}
			emptyText="No ESI requests in the last hour."
		/>
		<TelemetryChart
			title="Errors / minute"
			headline={compact(hourTotals.errors)}
			headlineClass={hourTotals.errors > 0 ? 'text-negative' : 'text-foreground'}
			sub="last hour"
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
				<div class="truncate text-xs text-muted-foreground">{label}</div>
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
