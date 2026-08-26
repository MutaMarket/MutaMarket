<script lang="ts">
	// The sell page's select-modules dialog, the legacy
	// PublicLocationSettings.vue: the active character's containers with
	// abyssal modules, each with a publish switch driving the ported
	// /public-assets endpoints.
	import { invalidateAll } from '$app/navigation';
	import GameImage from './game-image.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Switch } from '$lib/components/ui/switch';
	import { locationFlagLabel } from '$lib/location-flags';
	import type { SellLocation } from '$lib/types';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	let locations = $state<SellLocation[] | null>(null);
	let busy = $state<number | null>(null);

	$effect(() => {
		if (open && locations === null) {
			void refresh();
		}
	});

	async function refresh() {
		const response = await fetch('/api/sell/locations');
		if (response.ok) {
			locations = await response.json();
		}
	}

	async function toggle(location: SellLocation, publish: boolean) {
		busy = location.asset_id;
		try {
			if (publish) {
				await fetch('/public-assets', {
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ asset_id: location.asset_id }),
					redirect: 'manual'
				});
			} else if (location.public_asset_id !== null) {
				await fetch(`/public-assets/${location.public_asset_id}`, {
					method: 'DELETE',
					redirect: 'manual'
				});
			}
			await refresh();
			await invalidateAll();
		} finally {
			busy = null;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Title>Select modules</Dialog.Title>
		<Dialog.Description>
			Make whole containers public: every abyssal module inside a published container appears on
			your sell page and character profile.
		</Dialog.Description>
		{#if locations === null}
			<p class="py-4 text-sm text-muted-foreground">Loading your locations…</p>
		{:else if locations.length === 0}
			<p class="py-4 text-sm text-muted-foreground">
				No containers with abyssal modules found. Run an asset import first.
			</p>
		{:else}
			<ul class="flex max-h-80 flex-col gap-1 overflow-y-auto">
				{#each locations as location (location.asset_id)}
					<li class="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-card-2">
						<GameImage
							src="https://images.evetech.net/types/{location.type_id}/icon?size=64"
							alt=""
							class="size-8 rounded"
						/>
						<div class="min-w-0 grow">
							<span class="block truncate text-sm">{location.name || 'Unnamed container'}</span>
							<span class="text-xs text-muted-foreground">
								{locationFlagLabel(location.location_flag)} ·
								{location.abyssal_count.toLocaleString('en-US')} modules
							</span>
						</div>
						<Switch
							checked={location.public_asset_id !== null}
							disabled={busy === location.asset_id}
							onCheckedChange={(on) => toggle(location, on)}
						/>
					</li>
				{/each}
			</ul>
		{/if}
		<Dialog.Footer>
			<Button variant="secondary" onclick={() => (open = false)}>Done</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
