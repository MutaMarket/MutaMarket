<script lang="ts">
	// The donation row list, the legacy Sidebar/DonationsList.vue with
	// its inlined DonationRankBadge and RepeatDonorBadge children:
	// optional podium rank, character portrait and link, the repeat-donor
	// zap, and the amount in whole millions.
	import { Heart, Zap } from '@lucide/svelte';
	import GameImage from './game-image.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { donationCountLabel, isRepeatDonor, rankGradient } from '$lib/donations';
	import type { DonationEntry } from '$lib/donations';
	import { toMillions } from '$lib/format-number';

	let {
		donations,
		showRank = false,
		emptyMessage = 'No donations yet',
	}: {
		donations: DonationEntry[];
		showRank?: boolean;
		emptyMessage?: string;
	} = $props();
</script>

<Tooltip.Provider delayDuration={300}>
	<div class="space-y-0.5">
		{#each donations as donation, index (donation.id)}
			{@const gradient = rankGradient(index + 1)}
			<a
				href="/characters/{donation.character.slug}"
				class="group flex items-center gap-2 rounded-md px-1.5 py-1 transition-colors hover:bg-muted/50"
			>
				{#if showRank}
					{#if gradient !== null}
						<span
							class="w-4 shrink-0 bg-gradient-to-b {gradient} bg-clip-text text-center text-xs font-bold text-transparent"
						>
							{index + 1}
						</span>
					{:else}
						<span class="w-4 shrink-0 text-center text-xs text-muted-foreground/60">
							{index + 1}
						</span>
					{/if}
				{/if}
				<GameImage
					src="https://images.evetech.net/characters/{donation.character.id}/portrait?size=64"
					alt={donation.character.name}
					class="size-5 shrink-0 rounded"
				/>
				<span class="flex min-w-0 flex-1 items-center gap-1 text-xs font-medium">
					<span class="truncate">{donation.character.name}</span>
					{#if isRepeatDonor(donation.donation_count)}
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<span {...props}>
										<Zap class="size-3 shrink-0 text-yellow-500" />
									</span>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content>
								<p class="text-xs">{donationCountLabel(donation.donation_count)}</p>
							</Tooltip.Content>
						</Tooltip.Root>
					{/if}
				</span>
				<span
					class="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground tabular-nums"
				>
					{toMillions(donation.amount)}
				</span>
			</a>
		{:else}
			<div class="flex items-center justify-center gap-2 py-4 text-xs text-muted-foreground">
				<Heart class="size-3.5" />
				<span>{emptyMessage}</span>
			</div>
		{/each}
	</div>
</Tooltip.Provider>
