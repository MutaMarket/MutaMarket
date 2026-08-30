<script lang="ts">
	// The console overview: who the background work acts through, the
	// container's vitals over the toggled window, what the ingestion has
	// landed, and a one-line-per-job roll-up that links into the jobs
	// board. The heavy per-job cards and the ESI charts live on their own
	// sections, so this page only ever mounts the five vital charts.
	import VitalChart from '$lib/components/vital-chart.svelte';
	import { apply, live, subscribe } from '$lib/admin-live.svelte';
	import {
		HISTORY_WINDOWS,
		LOAD_SERIES,
		NETWORK_SERIES,
		SIZE_SERIES,
		USED_SERIES,
		cpuPercent,
		cpuPoints,
		formatBytes,
		gaugePoints,
		networkRates,
		percentOf,
		percentPoints,
		ratePoints,
		type HistoryWindow
	} from '$lib/admin-vitals';
	import { JOB_CARDS } from '$lib/job-cards';
	import { parseDbTimestamp, relativeTime } from '$lib/duration';
	import type { MetricsHistory } from '$lib/admin-types';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	$effect(() => {
		apply(data.live);
	});
	$effect(() => subscribe(['system', 'database', 'jobs']));

	const system = $derived(live.system ?? data.live.system ?? null);
	const database = $derived(live.database ?? data.live.database ?? null);

	// The capacities the charts divide by, as their own derived numbers.
	// They never move, so a poll that reassigns `system` recomputes them
	// to the same value and stops there — the history-derived point
	// arrays below are not rebuilt. Reading system.cpu_cores inside those
	// deriveds instead is what used to redraw the CPU chart every five
	// seconds.
	const cores = $derived(system?.cpu_cores ?? null);
	/** The cgroup limit, else the machine's total memory. */
	const memoryCapacity = $derived(
		system === null ? null : (system.memory_limit_bytes ?? system.memory_total_bytes)
	);
	const diskCapacity = $derived(system?.disk_total_bytes ?? null);

	// --- Vitals history ----------------------------------------------------

	let historyWindow = $state<HistoryWindow>('24h');
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

	// Only `history` and the (stable) capacities feed these, so a poll
	// that leaves them alone never hands the charts new data.
	const cpu = $derived(cpuPoints(history, cores));
	const memory = $derived(percentPoints(history, 'memory_bytes', memoryCapacity));
	const disk = $derived(percentPoints(history, 'disk_used_bytes', diskCapacity));
	const network = $derived(
		ratePoints(history, { rx: 'network_rx_bytes', tx: 'network_tx_bytes' })
	);
	const databaseSize = $derived(gaugePoints(history, 'database_size_bytes'));

	const sample = $derived(live.currentSample);
	const load = $derived(sample === null ? null : cpuPercent(live.previousSample, sample));
	const rates = $derived(sample === null ? null : networkRates(live.previousSample, sample));
	const cpuUtilization = $derived(load === null ? null : load / (cores ?? 1));
	const memoryUsed = $derived(
		system === null ? null : (system.memory_current_bytes ?? system.memory_rss_bytes)
	);
	const memoryPercent = $derived(percentOf(memoryUsed, memoryCapacity));
	const diskPercent = $derived(percentOf(system?.disk_used_bytes ?? null, diskCapacity));

	const databaseTiles = $derived(
		database === null
			? []
			: ([
					['Modules', database.modules],
					['No estimate', database.modules_without_estimate],
					['Contracts', database.contracts],
					['Contract items', database.contract_items],
					['Characters', database.characters],
					['Users', database.users],
					['Assets', database.assets],
					['Public ownerships', database.public_ownerships],
					['Market days', database.market_history_days]
				] as const)
	);

	// --- Job roll-up -------------------------------------------------------

	/** Jobs needing attention first: failing, then paused, then the rest. */
	const jobSummary = $derived(
		live.jobs
			.map((job) => {
				const last = job.last_runs.find((run) => run.finished_at !== null) ?? null;
				const failed = last?.outcome === 'error';
				return {
					name: job.name,
					title: JOB_CARDS[job.name]?.title ?? job.name,
					running: job.running,
					paused: job.paused,
					failed,
					last,
					rank: failed ? 0 : job.paused ? 1 : 2
				};
			})
			.sort((a, b) => a.rank - b.rank || a.title.localeCompare(b.title))
	);
	const attention = $derived(jobSummary.filter((job) => job.rank < 2));
	const running = $derived(jobSummary.filter((job) => job.running).length);
</script>

<svelte:head><title>Admin - MutaMarket</title></svelte:head>

<!-- Service character: who the background features act through
     (structure resolution, donation processing when it lands). -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Service // Character</h2>
	<div class="hud-frame flex flex-wrap items-center gap-4 p-4">
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
			sub={cores !== null ? `of ${cores} cores` : undefined}
			series={LOAD_SERIES}
			points={cpu}
			yDomain={[0, 100]}
			format={(value) => `${value.toFixed(0)}%`}
		/>
		<!-- Utilization needs a capacity: the cgroup limit, else the
		     machine's total memory. Without either (non-Linux) the chart
		     falls back to plain bytes. -->
		{#if memoryCapacity !== null}
			<VitalChart
				title="Memory"
				headline={memoryPercent === null ? '—' : `${memoryPercent.toFixed(0)}%`}
				sub={`${formatBytes(memoryUsed)} of ${formatBytes(memoryCapacity)}`}
				series={USED_SERIES}
				points={memory}
				yDomain={[0, 100]}
				format={(value) => `${value.toFixed(0)}%`}
			/>
		{:else}
			<VitalChart
				title="Memory"
				headline={formatBytes(memoryUsed)}
				series={USED_SERIES}
				points={gaugePoints(history, 'memory_bytes')}
				format={(value) => formatBytes(Math.round(value))}
			/>
		{/if}
		<VitalChart
			title="Storage"
			headline={diskPercent === null ? '—' : `${diskPercent.toFixed(0)}%`}
			sub={system?.disk_used_bytes != null && diskCapacity !== null
				? `${formatBytes(diskCapacity - system.disk_used_bytes)} free of ${formatBytes(diskCapacity)}`
				: undefined}
			series={USED_SERIES}
			points={disk}
			yDomain={[0, 100]}
			format={(value) => `${value.toFixed(0)}%`}
		/>
		<VitalChart
			title="Network"
			headline={rates === null
				? '—'
				: `${formatBytes(Math.round(rates.rx))}/s · ${formatBytes(Math.round(rates.tx))}/s`}
			sub="in · out"
			series={NETWORK_SERIES}
			points={network}
			format={(value) => formatBytes(Math.round(value))}
		/>
		<VitalChart
			title="Database"
			headline={formatBytes(system?.database_size_bytes ?? null)}
			series={SIZE_SERIES}
			points={databaseSize}
			format={(value) => formatBytes(Math.round(value))}
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

<!-- Jobs roll-up: the state of the board without its charts. Anything
     failing or paused surfaces here; the rest is a count. -->
<section>
	<div class="mb-3 flex items-center gap-4">
		<h2 class="hud-label">Jobs // Scheduler</h2>
		<a class="text-xs text-muted-foreground hover:text-foreground" href="/admin/jobs">
			Open the board
		</a>
		<span class="ml-auto text-xs text-muted-foreground tabular-nums">
			{jobSummary.length} jobs · {running} running
		</span>
	</div>
	<div class="hud-frame divide-y divide-border">
		{#each attention as job (job.name)}
			<a
				href="/admin/jobs"
				class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 transition hover:bg-white/[0.03]"
			>
				<span
					class="size-2 shrink-0 rounded-full {job.failed
						? 'bg-negative'
						: 'bg-[#fab219]'}"
				></span>
				<span class="text-sm font-medium">{job.title}</span>
				<span class="text-xs text-muted-foreground">
					{job.failed ? 'last run failed' : 'paused'}
				</span>
				{#if job.failed && job.last?.error}
					<span class="min-w-0 truncate text-xs text-negative">{job.last.error}</span>
				{/if}
				{#if job.last}
					<span class="ml-auto text-xs text-muted-foreground">
						{relativeTime(
							parseDbTimestamp(job.last.finished_at ?? job.last.started_at) - live.now
						)}
					</span>
				{/if}
			</a>
		{:else}
			<p class="px-4 py-3 text-sm text-muted-foreground">
				Every job is scheduled and its last run succeeded.
			</p>
		{/each}
	</div>
</section>
