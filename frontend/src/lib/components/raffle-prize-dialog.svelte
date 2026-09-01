<script lang="ts">
	// The legacy RafflePrize.vue: the site-wide dialog a drawn prize
	// opens, claiming it to the settings page or declining it back into
	// the pool.
	import { goto, invalidateAll } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import type { RafflePrize } from '$lib/types';

	let { prize }: { prize: RafflePrize } = $props();

	let submitting = $state(false);

	async function respond(method: 'PUT' | 'DELETE') {
		submitting = true;
		try {
			const response = await fetch(`/raffle/${prize.id}`, { method });
			if (!response.ok && !response.redirected) return;
			if (method === 'PUT') {
				await goto('/settings', { invalidateAll: true });
			} else {
				await invalidateAll();
			}
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root open={true}>
	<Dialog.Content class="max-w-md">
		<Dialog.Header>
			<Dialog.Title>Congratulations!</Dialog.Title>
			<Dialog.Description>
				You won {prize.name ?? 'a prize'} in the MutaMarket raffle.
			</Dialog.Description>
		</Dialog.Header>

		<div
			class="border-border bg-card grid grid-cols-[auto_1fr] items-center gap-4 rounded-lg border p-2"
		>
			{#if prize.type}
				<img
					alt={prize.type.name}
					class="size-10 rounded-lg"
					src="https://images.evetech.net/types/{prize.type.id}/icon?size=64"
				/>
			{:else if prize.icon_url}
				<img alt={prize.name ?? 'Prize'} class="size-10 rounded-lg" src={prize.icon_url} />
			{/if}
			<div>
				<span class="text-muted-foreground block leading-none">Your prize</span>
				<h2 class="text-lg">{prize.name}</h2>
				{#if prize.description}
					<p class="text-muted-foreground text-xs">{prize.description}</p>
				{/if}
			</div>
		</div>

		<Dialog.Footer>
			<Button disabled={submitting} variant="secondary" onclick={() => respond('DELETE')}>
				Decline
			</Button>
			<Button disabled={submitting} onclick={() => respond('PUT')}>Claim prize</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
