<script lang="ts">
	// One bento card of the admin dashboard: a scheduler job with the
	// visuals that fit it — live progress meter while a fan-out run
	// reports "N/M", the headline metric of the last run, and either
	// multi-line per-run series (config.series over the runs' recorded
	// metrics) or work-per-run spark columns. Text is kept minimal: the
	// description, items label and summary live in tooltips, the times
	// ride behind icons.
	import { Clock, Moon, Repeat, ScrollText, Timer } from '@lucide/svelte';
	import { defineChart, lineY } from '@tanstack/charts';
	import { scaleLinear } from '@tanstack/charts/scales/linear';
	import { Chart } from '@tanstack/charts/svelte';
	import { tooltip } from '@tanstack/charts/tooltip';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
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
	// shared scale (the series share a unit by contract). One mark per
	// sub-metric, each with its own paint, so nothing stacks.
	const lineRuns = $derived(config.series ? sparkRuns.filter((run) => run.metrics !== null) : []);
	const lineDefinition = $derived(
		defineChart({
			marks: (config.series ?? []).map((s) =>
				lineY(
					lineRuns.map((run) => ({
						at: parseDbTimestamp(run.started_at),
						label: s.label,
						value: run.metrics?.[s.key] ?? 0
					})),
					{ id: s.key, x: 'at', y: 'value', stroke: s.color, strokeWidth: 1.5 }
				)
			),
			scales: {
				x: { scale: scaleLinear, axis: false },
				y: { scale: scaleLinear, axis: false }
			},
			focus: 'group-x',
			tooltip: {
				use: tooltip,
				formatGroup: (focused) =>
					[
						relativeTime(Number(focused[0]?.xValue ?? 0) - now),
						...focused.map(
							(point) => `${point.datum.label}: ${point.datum.value.toLocaleString('en-US')}`
						)
					].join('\n')
			}
		})
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
				<!-- Per-run sub-metric lines on a shared scale. -->
				<div class="flex min-w-0 w-full max-w-72 grow flex-col items-end gap-1">
					<Chart definition={lineDefinition} ariaLabel="{config.title} per run" height={56} />
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
				onclick={() => (showHistory = true)}
			>
				<ScrollText class="size-3.5" stroke-width={1.5} />
			</button>
		{/if}
	</footer>

</article>

<Dialog.Root bind:open={showHistory}>
	<Dialog.Content class="sm:max-w-2xl">
		<Dialog.Header>
			<Dialog.Title>{config.title} // Run history</Dialog.Title>
			<Dialog.Description>
				{config.description} · the newest {job.last_runs.length} recorded runs.
			</Dialog.Description>
		</Dialog.Header>
		<div class="max-h-[60vh] overflow-y-auto">
			<table class="w-full text-sm">
				<thead class="sticky top-0 bg-card-1">
					<tr class="border-b border-border text-left">
						<th class="hud-label py-2 pr-3 font-normal">Started</th>
						<th class="hud-label py-2 pr-3 font-normal">Outcome</th>
						<th class="hud-label py-2 pr-3 text-right font-normal">
							{config.itemsLabel}
						</th>
						<th class="hud-label py-2 pr-3 text-right font-normal">Took</th>
						<th class="hud-label py-2 font-normal">Summary</th>
					</tr>
				</thead>
				<tbody>
					{#each job.last_runs as run (run.started_at)}
						<tr class="border-b border-border/60 align-top last:border-0">
							<td class="py-2 pr-3 whitespace-nowrap text-muted-foreground">
								{relativeTime(parseDbTimestamp(run.started_at) - now)}
							</td>
							<td
								class="py-2 pr-3 whitespace-nowrap {run.outcome === 'success'
									? 'text-positive'
									: run.outcome === null
										? 'text-muted-foreground'
										: 'text-negative'}"
							>
								{run.outcome ?? 'running'}
							</td>
							<td class="py-2 pr-3 text-right tabular-nums">
								{run.items === null ? '—' : run.items.toLocaleString('en-US')}
							</td>
							<td class="py-2 pr-3 text-right whitespace-nowrap tabular-nums text-muted-foreground">
								{run.duration_seconds === null ? '—' : `${run.duration_seconds}s`}
							</td>
							<td class="py-2 {run.error ? 'text-negative' : 'text-foreground'}">
								{run.summary ?? run.error ?? ''}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</Dialog.Content>
</Dialog.Root>
