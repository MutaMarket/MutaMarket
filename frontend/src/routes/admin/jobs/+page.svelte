<script lang="ts">
	// The jobs board: one card per registered scheduler job with its
	// run-now and pause controls. Designed cards lead in their bento
	// order, every other registered job follows with a default card, so
	// nothing in the scheduler is invisible here.
	import JobCard from '$lib/components/job-card.svelte';
	import { apply, live, refresh, subscribe } from '$lib/admin-live.svelte';
	import { jobBoardOrder, jobCard } from '$lib/job-cards';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	$effect(() => {
		apply(data.live);
	});
	$effect(() => subscribe(['jobs']));

	let notice = $state<string | null>(null);

	const jobs = $derived(live.jobs.length > 0 ? live.jobs : (data.live.jobs ?? []));
	const board = $derived.by(() => {
		const byName = new Map(jobs.map((job) => [job.name, job]));
		return jobBoardOrder([...byName.keys()]).map((name) => ({
			job: byName.get(name)!,
			config: jobCard(name)
		}));
	});

	async function runNow(job: string) {
		notice = null;
		const response = await fetch(`/api/admin/scheduler/${job}/run`, { method: 'POST' });
		if (!response.ok) {
			const body: { message?: string } = await response.json().catch(() => ({}));
			notice = `${job}: ${body.message ?? 'Run failed to start.'}`;
		}
		await refresh(true);
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
		await refresh(true);
	}
</script>

<svelte:head><title>Jobs - Admin - MutaMarket</title></svelte:head>

{#if notice}
	<p class="mb-4 text-sm text-negative">{notice}</p>
{/if}

<div class="grid grid-flow-dense grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
	{#each board as entry (entry.job.name)}
		<JobCard
			job={entry.job}
			config={entry.config}
			now={live.now}
			onRunNow={runNow}
			onSetPaused={setPaused}
		/>
	{/each}
</div>
