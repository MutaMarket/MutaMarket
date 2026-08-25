<script lang="ts">
	// The admin scheduler dashboard: every background job with its live
	// state, run history and controls (run now, pause/resume). Polls the
	// status endpoint so triggered runs and progress show up without a
	// reload. No legacy counterpart.
	import { Button } from '$lib/components/ui/button';
	import { humanizeInterval, parseDbTimestamp, relativeTime } from '$lib/duration';
	import type { PageProps } from './$types';
	import type { SchedulerStatus } from './+page.server';

	let { data }: PageProps = $props();

	/** Live-status poll cadence. */
	const POLL_INTERVAL_MS = 5000;

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let status = $state<SchedulerStatus>(data.status);
	let now = $state(Math.floor(Date.now() / 1000));
	let notice = $state<string | null>(null);
	let expanded = $state<Record<string, boolean>>({});

	$effect(() => {
		const poll = setInterval(async () => {
			now = Math.floor(Date.now() / 1000);
			try {
				const response = await fetch('/api/admin/scheduler');
				if (response.ok) {
					status = await response.json();
				}
			} catch {
				// Keep the last state while the API is unreachable.
			}
		}, POLL_INTERVAL_MS);
		const tick = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
		return () => {
			clearInterval(poll);
			clearInterval(tick);
		};
	});

	async function refresh() {
		const response = await fetch('/api/admin/scheduler');
		if (response.ok) {
			status = await response.json();
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

	function lastFinished(job: (typeof status.jobs)[number]) {
		return job.last_runs.find((run) => run.finished_at !== null) ?? null;
	}
</script>

<svelte:head><title>Scheduler - MutaMarket</title></svelte:head>

<div class="mb-4 flex flex-wrap items-center gap-3">
	<h1 class="text-xl font-semibold">Scheduler</h1>
	<span
		class="rounded-full border border-border px-2 py-0.5 text-xs {status.enabled
			? 'text-positive'
			: 'text-muted-foreground'}"
	>
		{status.enabled ? 'loops running' : 'loops disabled'}
	</span>
	{#if status.in_downtime}
		<span class="rounded-full border border-border px-2 py-0.5 text-xs text-yellow-500">
			EVE downtime: guarded jobs paused
		</span>
	{/if}
</div>

{#if notice}
	<p class="mb-3 text-sm text-negative">{notice}</p>
{/if}

<div class="flex flex-col gap-3">
	{#each status.jobs as job (job.name)}
		{@const last = lastFinished(job)}
		<div class="rounded-lg border border-border bg-card-1 p-3">
			<div class="flex flex-wrap items-center gap-3">
				<span class="font-mono text-sm text-foreground">{job.name}</span>
				<span class="text-xs text-muted-foreground">{humanizeInterval(job.interval_seconds)}</span>
				{#if job.downtime_guarded}
					<span class="text-xs text-muted-foreground" title="Skips EVE's daily downtime window">
						downtime-guarded
					</span>
				{/if}
				{#if job.paused}
					<span class="rounded-full border border-border px-2 py-0.5 text-xs text-yellow-500">
						paused
					</span>
				{/if}
				{#if job.running}
					<span class="animate-pulse text-xs text-positive">running…</span>
				{:else if job.next_run_at !== null && !job.paused}
					<span class="text-xs text-muted-foreground">
						next run {relativeTime(job.next_run_at - now)}
					</span>
				{/if}
				<span class="ml-auto flex items-center gap-1">
					<Button
						variant="outline"
						size="sm"
						class="h-7 px-2 text-xs"
						disabled={job.running}
						onclick={() => runNow(job.name)}
					>
						Run now
					</Button>
					<Button
						variant="outline"
						size="sm"
						class="h-7 px-2 text-xs"
						onclick={() => setPaused(job.name, !job.paused)}
					>
						{job.paused ? 'Resume' : 'Pause'}
					</Button>
				</span>
			</div>

			<div class="mt-2 text-xs">
				{#if last}
					<span class={last.outcome === 'success' ? 'text-positive' : 'text-negative'}>
						{last.outcome}
					</span>
					<span class="text-muted-foreground">
						{relativeTime(parseDbTimestamp(last.finished_at ?? last.started_at) - now)}
						— {last.summary ?? last.error ?? ''}
					</span>
				{:else}
					<span class="text-muted-foreground">no recorded runs yet</span>
				{/if}
				{#if job.last_runs.length > 1}
					<button
						class="ml-2 text-muted-foreground underline hover:text-foreground"
						onclick={() => (expanded[job.name] = !expanded[job.name])}
					>
						{expanded[job.name] ? 'hide history' : `history (${job.last_runs.length})`}
					</button>
				{/if}
			</div>

			{#if expanded[job.name]}
				<ul class="mt-2 flex flex-col gap-1 border-t border-border pt-2 text-xs">
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
		</div>
	{/each}
</div>
