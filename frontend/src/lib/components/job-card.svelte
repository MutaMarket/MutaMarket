<script lang="ts">
	// One bento card of the admin dashboard: a scheduler job with the
	// visuals that fit it — live progress meter while a fan-out run
	// reports "N/M", the headline metric of the last run, and either
	// multi-line per-run series (config.series over the runs' recorded
	// metrics) or work-per-run spark columns. Text is kept minimal: the
	// description, items label and summary live in tooltips, the times
	// ride behind icons.
	import { Clock, Moon, Repeat, ScrollText, Timer } from '@lucide/svelte';
	import { LineChart } from 'layerchart';
	import { Button } from '$lib/components/ui/button';
	import * as Chart from '$lib/components/ui/chart';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { humanizeInterval, parseDbTimestamp, relativeTime } from '$lib/duration';
	import { progressFraction, type JobCardConfig } from '$lib/job-cards';
	import type { SchedulerJob, SchedulerRun } from '$lib/admin-types';

	let {
		job,
		config,
		now,
		onRunNow,
		onSetPaused
	}: {
		job: SchedulerJob;
		config: JobCardConfig;
		now: number;
		onRunNow: (job: string) => void;
		onSetPaused: (job: string, paused: boolean) => void;
	} = $props();

	/** Work-per-run line: the app accent. */
	const SPARK_COLOR = '#a3e635';

	let showHistory = $state(false);

	const finishedRuns = $derived(job.last_runs.filter((run) => run.finished_at !== null));
	const last = $derived(finishedRuns[0] ?? null);
	// Chronological (oldest left) for the charts.
	const sparkRuns = $derived([...finishedRuns].reverse());
	const sparkMax = $derived(Math.max(...sparkRuns.map((run) => run.items ?? 0), 1));

	const fraction = $derived(job.running ? progressFraction(job.progress) : null);

	const lamp = $derived.by(() => {
		if (job.running) return { class: 'bg-positive animate-pulse', title: 'running' };
		if (job.paused) return { class: 'bg-[#fab219]', title: 'paused' };
		if (last?.outcome === 'error') return { class: 'bg-negative', title: 'last run failed' };
		return { class: 'bg-muted-foreground/40', title: 'idle' };
	});

	function runTitle(run: SchedulerRun): string {
		const when = relativeTime(parseDbTimestamp(run.started_at) - now);
		const items =
			run.items === null ? '' : ` · ${run.items.toLocaleString('en-US')} ${config.itemsLabel}`;
		return `${when}${items}${run.duration_seconds !== null ? ` · ${run.duration_seconds}s` : ''}`;
	}

	// The multi-line chart: runs with recorded metrics, drawn on one
	// shared scale (the series share a unit by contract) through the
	// shadcn chart stack.
	const lineRuns = $derived(
		config.series ? sparkRuns.filter((run) => run.metrics !== null) : []
	);
	const lineRows = $derived(
		lineRuns.map((run) => ({
			at: parseDbTimestamp(run.started_at),
			...Object.fromEntries(
				(config.series ?? []).map((s) => [s.key, run.metrics?.[s.key] ?? 0])
			)
		}))
	);
	const lineConfig = $derived(
		Object.fromEntries(
			(config.series ?? []).map((s) => [s.key, { label: s.label, color: s.color }])
		) satisfies Chart.ChartConfig
	);
	const lineSeries = $derived(
		(config.series ?? []).map((s) => ({ key: s.key, label: s.label, color: s.color }))
	);
</script>

<article class="hud-frame flex flex-col gap-3 p-4 {config.size === 'wide' ? 'sm:col-span-2' : ''}">
	<header class="flex items-center gap-2.5">
		<span class="size-2 shrink-0 rounded-full {lamp.class}" title={lamp.title}></span>
		<h3 class="min-w-0 truncate text-sm font-semibold text-foreground" title={config.description}>
			{config.title}
		</h3>
		<span class="ml-auto flex shrink-0 items-center gap-1">
			<Button
				variant="outline"
				size="sm"
				class="h-6 px-2 text-xs"
				disabled={job.running}
				onclick={() => onRunNow(job.name)}
			>
				Run
			</Button>
			<Button
				variant="outline"
				size="sm"
				class="h-6 px-2 text-xs"
				onclick={() => onSetPaused(job.name, !job.paused)}
			>
				{job.paused ? 'Resume' : 'Pause'}
			</Button>
		</span>
	</header>

	{#if job.running}
		<!-- Live run: the meter when progress reports N/M, else the line. -->
		<div>
			{#if fraction !== null}
				<div class="mb-1 h-1 overflow-hidden rounded-full bg-primary/20">
					<div
						class="h-full rounded-full bg-primary transition-[width] duration-1000"
						style="width: {fraction * 100}%"
					></div>
				</div>
			{/if}
			<p class="animate-pulse text-xs text-positive">{job.progress ?? 'running…'}</p>
		</div>
	{:else if last}
		<div class="flex items-end justify-between gap-3">
			<div>
				<div
					class="text-2xl font-semibold text-foreground"
					title="{config.itemsLabel}{last.summary ? ` — ${last.summary}` : ''}"
				>
					{(last.items ?? 0).toLocaleString('en-US')}
				</div>
				<div class="flex items-center gap-1 text-xs text-muted-foreground">
					<Clock class="size-3" stroke-width={1.5} />
					{relativeTime(parseDbTimestamp(last.finished_at ?? last.started_at) - now)}
				</div>
			</div>
			{#if config.series && lineRuns.length > 1}
				<!-- Per-run sub-metric lines on a shared scale, with the
				     shadcn hover tooltip. -->
				<div class="flex min-w-0 grow flex-col items-end gap-1">
					<Chart.Container config={lineConfig} class="h-14 w-full max-w-72">
						<LineChart
							data={lineRows}
							x="at"
							series={lineSeries}
							axis={false}
							points={false}
							props={{ spline: { strokeWidth: 1.5 } }}
						>
							{#snippet tooltip()}
								<Chart.Tooltip
									labelFormatter={(at: number) => relativeTime(at - now)}
								/>
							{/snippet}
						</LineChart>
					</Chart.Container>

				</div>
			{:else if sparkRuns.length > 1}
				<!-- Work per run, oldest to newest; hover carries the numbers. -->
				<div class="flex h-10 items-end gap-[2px]" aria-hidden="true">
					{#each sparkRuns as run (run.started_at)}
						<div
							class="w-[7px] rounded-t-[2px]"
							style="height: {Math.max(((run.items ?? 0) / sparkMax) * 100, 5)}%;
								background: {run.outcome === 'error' ? 'var(--color-negative)' : SPARK_COLOR};
								opacity: {run === sparkRuns[sparkRuns.length - 1] ? 1 : 0.55}"
							title={runTitle(run)}
						></div>
					{/each}
				</div>
			{/if}
		</div>
		{#if last.outcome === 'error'}
			<p class="text-xs text-negative">{last.error}</p>
		{/if}
	{:else}
		<p class="text-xs text-muted-foreground">No recorded runs yet.</p>
	{/if}

	<footer class="mt-auto flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
		<span class="flex items-center gap-1 font-mono" title="Cadence">
			<Repeat class="size-3" stroke-width={1.5} />
			{humanizeInterval(job.interval_seconds)}
		</span>
		{#if job.paused}
			<span class="text-[#fab219]">paused</span>
		{:else if !job.running && job.next_run_at !== null}
			<span class="flex items-center gap-1" title="Next scheduled run">
				<Timer class="size-3" stroke-width={1.5} />
				{relativeTime(job.next_run_at - now)}
			</span>
		{/if}
		{#if job.downtime_guarded}
			<Tooltip.Provider delayDuration={300}>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<span {...props} class="text-muted-foreground/60">
								<Moon class="size-3" stroke-width={1.5} />
							</span>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>Skips EVE's daily downtime window</Tooltip.Content>
				</Tooltip.Root>
			</Tooltip.Provider>
		{/if}
		{#if finishedRuns.length > 0}
			<button
				class="ml-auto hover:text-foreground"
				title="Run history"
				onclick={() => (showHistory = !showHistory)}
			>
				<ScrollText class="size-3.5" stroke-width={1.5} />
			</button>
		{/if}
	</footer>

	{#if showHistory}
		<ul class="flex flex-col gap-1 border-t border-border pt-2 text-xs">
			{#each job.last_runs as run (run.started_at)}
				<li class="flex flex-wrap gap-2">
					<span class="text-muted-foreground">
						{relativeTime(parseDbTimestamp(run.started_at) - now)}
					</span>
					<span
						class={run.outcome === 'success'
							? 'text-positive'
							: run.outcome === null
								? 'text-muted-foreground'
								: 'text-negative'}
					>
						{run.outcome ?? 'running'}
					</span>
					<span class="text-foreground">{run.summary ?? run.error ?? ''}</span>
				</li>
			{/each}
		</ul>
	{/if}
</article>
