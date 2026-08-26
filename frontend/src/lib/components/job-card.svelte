<script lang="ts">
	// One bento card of the operations console: a scheduler job with the
	// state and visuals that fit it — live progress meter while a fan-out
	// run reports "N/M", the headline metric of the last run, work-per-run
	// spark columns over the recorded history, and the outcome strip.
	import { Clock, Moon, Repeat, Timer } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
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

	/** Work-per-run column color (dataviz sequential hue, dark step). */
	const SPARK_COLOR = '#3987e5';

	let showHistory = $state(false);

	const finishedRuns = $derived(job.last_runs.filter((run) => run.finished_at !== null));
	const last = $derived(finishedRuns[0] ?? null);
	// Chronological for the spark columns (oldest left).
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
		const items = run.items === null ? '' : ` · ${run.items.toLocaleString('en-US')} ${config.itemsLabel}`;
		return `${when}${items}${run.duration_seconds !== null ? ` · ${run.duration_seconds}s` : ''}`;
	}
</script>

<article
	class="hud-panel flex flex-col gap-3 p-4 {config.size === 'wide' ? 'sm:col-span-2' : ''}"
>
	<header class="flex items-start gap-2.5">
		<span class="mt-1.5 size-2 shrink-0 rounded-full {lamp.class}" title={lamp.title}></span>
		<div class="min-w-0">
			<h3 class="text-sm font-semibold text-foreground">{config.title}</h3>
			<p class="truncate text-xs text-muted-foreground">{config.description}</p>
		</div>
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
				<div class="mb-1 h-1 overflow-hidden rounded-full bg-[#184f95]/40">
					<div
						class="h-full rounded-full bg-[#3987e5] transition-[width] duration-1000"
						style="width: {fraction * 100}%"
					></div>
				</div>
			{/if}
			<p class="animate-pulse text-xs text-positive">{job.progress ?? 'running…'}</p>
		</div>
	{:else if last}
		<div class="flex items-end justify-between gap-3">
			<div>
				<div class="text-2xl font-semibold text-foreground">
					{(last.items ?? 0).toLocaleString('en-US')}
				</div>
				<div class="flex items-center gap-1 text-xs text-muted-foreground">
					{config.itemsLabel}
					<Clock class="ml-1 size-3" stroke-width={1.5} />
					{relativeTime(parseDbTimestamp(last.finished_at ?? last.started_at) - now)}
				</div>
			</div>
			{#if sparkRuns.length > 1}
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
		{:else if config.size === 'wide' && last.summary}
			<p class="truncate text-xs text-muted-foreground" title={last.summary}>{last.summary}</p>
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
				class="ml-auto underline hover:text-foreground"
				onclick={() => (showHistory = !showHistory)}
			>
				{showHistory ? 'hide' : 'history'}
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
