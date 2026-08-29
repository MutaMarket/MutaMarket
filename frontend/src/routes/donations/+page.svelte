<script lang="ts">
	// The donations page, the legacy Donations/DonationsPage.vue: the
	// support CTA with the copy button, the 14-day and all-time
	// leaderboards, and the recent activity list.
	import { Copy, Crown, Sparkles, Trophy } from '@lucide/svelte';
	import type { PageProps } from './$types';
	import DonationsList from '$lib/components/donations-list.svelte';
	import { Button } from '$lib/components/ui/button';
	import { notifySuccess } from '$lib/toast';

	let { data }: PageProps = $props();

	const character = $derived(data.premium.premium_character);

	// The legacy handleDonate with the premium.donations.copied* strings.
	function handleDonate() {
		void navigator.clipboard.writeText(character);
		notifySuccess(
			'Character name copied to clipboard',
			`You can now send ISK to "${character}"`
		);
	}
</script>

<svelte:head>
	<title>Donations - MutaMarket</title>
	<meta name="description" content="Support MutaMarket and help us keep the site running!" />
	<meta name="keywords" content="donations, support, isk" />
</svelte:head>

<div class="space-y-6">
	<!-- Support CTA -->
	<div class="rounded-lg border bg-gradient-to-r from-primary/10 via-primary/5 to-transparent p-4">
		<div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
			<div>
				<h2 class="font-semibold">Support MutaMarket</h2>
				<p class="text-sm text-muted-foreground">
					Send ISK to
					<strong class="text-foreground">{character}</strong>
					to help keep the servers running.
				</p>
			</div>
			<Button onclick={handleDonate} class="shrink-0 gap-2">
				<Copy class="size-4" />
				Copy character name
			</Button>
		</div>
	</div>

	<!-- Leaderboards -->
	<div class="grid gap-4 lg:grid-cols-2">
		<div class="hud-frame">
			<div class="flex items-center gap-2 border-b px-4 py-3">
				<Trophy class="size-4 text-yellow-500" />
				<h3 class="font-medium">Top 14 Days</h3>
			</div>
			<div class="p-3">
				<DonationsList
					donations={data.donations.recent}
					emptyMessage="No donations in the last 14 days"
					showRank={true}
				/>
			</div>
		</div>

		<div class="hud-frame">
			<div class="flex items-center gap-2 border-b px-4 py-3">
				<Crown class="size-4 text-amber-500" />
				<h3 class="font-medium">Hall of Fame</h3>
			</div>
			<div class="p-3">
				<DonationsList donations={data.donations.highest} showRank={true} />
			</div>
		</div>
	</div>

	<!-- Recent Activity -->
	<div class="hud-frame">
		<div class="flex items-center gap-2 border-b px-4 py-3">
			<Sparkles class="size-4 text-primary" />
			<h3 class="font-medium">Recent Donations</h3>
		</div>
		<div class="p-3">
			<DonationsList donations={data.donations.latest} />
		</div>
	</div>
</div>
