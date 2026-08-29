<script lang="ts">
	// The shared-workbench invitation, the legacy
	// ShowWorkbenchInvitationPage: the shared modules as cards, with the
	// accept action importing them into the visitor's own workbench.
	import { FlaskConical } from '@lucide/svelte';
	import { page } from '$app/state';
	import ModuleCard from '$lib/components/module-card.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { defaultDisplaySettings } from '$lib/display';
	import { notifySuccess } from '$lib/toast';
	import { refreshWorkbench, workbenchOpen } from '$lib/workbench';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const signedIn = $derived(Boolean(page.data.nav?.user));
	let accepting = $state(false);

	async function accept() {
		accepting = true;
		try {
			await fetch(`/workbench/${data.shared}`, { method: 'POST', redirect: 'manual' });
			notifySuccess(
				'Modules added to workbench',
				`You have added ${data.modules.length} modules to your workbench.`
			);
			await refreshWorkbench();
			workbenchOpen.set(true);
		} finally {
			accepting = false;
		}
	}
</script>

<svelte:head><title>Shared Workbench - MutaMarket</title></svelte:head>

<PageHeader
	title="Shared Workbench"
	subtitle="Someone shared {data.modules.length} module{data.modules.length === 1 ? '' : 's'} with you"
>
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<FlaskConical class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
	{#snippet actions()}
		{#if signedIn}
			<Button class="h-8" disabled={accepting || data.modules.length === 0} onclick={accept}>
				Add all to my workbench
			</Button>
		{:else}
			<Button class="h-8" href="/login">Log in to add them</Button>
		{/if}
	{/snippet}
</PageHeader>

{#if data.modules.length === 0}
	<div class="hud-frame p-6">
		<span class="block text-lg font-medium">Nothing here</span>
		<p class="text-muted-foreground">This workbench link contains no known modules.</p>
	</div>
{:else}
	<div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
		{#each data.modules as module (module.id)}
			<ModuleCard {module} settings={defaultDisplaySettings()} />
		{/each}
	</div>
{/if}
