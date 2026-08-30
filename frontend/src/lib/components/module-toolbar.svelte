<script lang="ts">
	// The show-page toolbar, mirroring Show/ModuleToolbar.vue: ghost icon
	// buttons with tooltips, grouped by hairline dividers; the three
	// search buttons open the variance forms.
	import {
		ChevronDown,
		Ellipsis,
		ExternalLink,
		FileCodeCorner,
		FilePenLine,
		GitCompareArrows,
		RotateCcwClock,
		Link as LinkIcon,
		Search,
		Share2,
		TrendingDown,
		type Icon as IconType
	} from '@lucide/svelte';
	import SearchMenuForm from './search-menu-form.svelte';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { openContractInGame } from '$lib/open-contract';
	import {
		copyContractLink,
		copyImageLink,
		copyItemLink,
		copyPyfa,
		shareModule
	} from '$lib/export';
	import {
		cheapestSearchPath,
		historicSearchPath,
		similarSearchPath
	} from '$lib/module-finder';
	import type { AbyssalTypeStatistic, ModuleDetail } from '$lib/types';

	let {
		module,
		typeStatistics = []
	}: {
		module: ModuleDetail;
		typeStatistics?: AbyssalTypeStatistic[];
	} = $props();

	const searchAttributes = $derived(
		module.mutated_attributes.map((attribute) => ({
			id: attribute.id,
			display_name: attribute.display_name
		}))
	);

	type SearchKind = 'similar' | 'cheapest' | 'historic';
	let searchVariance: Record<SearchKind, number> = $state({
		similar: 1,
		cheapest: 1,
		historic: 1
	});
	let searchEnabled: Record<SearchKind, number[]> = $state({
		similar: [],
		cheapest: [],
		historic: []
	});

	function submitSearch(kind: SearchKind) {
		const enabled = searchEnabled[kind];
		const variance = searchVariance[kind];
		const path =
			kind === 'similar'
				? similarSearchPath(module, typeStatistics, enabled, variance)
				: kind === 'cheapest'
					? cheapestSearchPath(module, typeStatistics, enabled, variance)
					: historicSearchPath(module, typeStatistics, enabled, variance);
		goto(path);
	}

	const openContractIngame = () => openContractInGame(module.contract?.id);

	interface ToolbarAction {
		icon: typeof IconType;
		label: string;
		chevron?: boolean;
		disabled?: boolean;
		disabledReason?: string;
		onclick?: () => void;
	}

	const noContract = $derived(module.contract === null);

	// The three variance-search dropdowns rendered between the type
	// search and the export group.
	const searchMenus: { kind: SearchKind; icon: typeof IconType; label: string }[] = [
		{ kind: 'similar', icon: GitCompareArrows, label: 'Search similar' },
		{ kind: 'cheapest', icon: TrendingDown, label: 'Search cheapest' },
		{ kind: 'historic', icon: RotateCcwClock, label: 'Search historic' }
	];

	const groups: ToolbarAction[][] = $derived([
		[
			{ icon: Search, label: 'Search this type', onclick: () => goto(`/modules/type/${module.type.id}`) }
		],
		[
			{ icon: FileCodeCorner, label: 'Pyfa', onclick: () => copyPyfa(module) },
			{ icon: LinkIcon, label: 'Copy item link', onclick: () => copyItemLink(module) },
			{
				icon: FilePenLine,
				label: 'Copy contract link',
				disabled: noContract,
				disabledReason: 'No active contract',
				onclick: () => copyContractLink(module)
			},
			{
				icon: ExternalLink,
				label: 'Open contract in game',
				disabled: noContract,
				disabledReason: 'No active contract',
				onclick: openContractIngame
			}
		],
		[{ icon: Share2, label: 'Share module', onclick: () => shareModule(module) }]
	]);
</script>

<Tooltip.Provider delayDuration={300}>
	<div class="flex flex-wrap items-center gap-1 border-t border-border px-3 py-2">
		{#each groups as group, groupIndex (groupIndex)}
			{#if groupIndex > 0}
				<div class="mx-1 h-5 w-px bg-border"></div>
			{/if}
			{#each group as action (action.label)}
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<span {...props} class="inline-flex">
								<Button
									variant="ghost"
									class="gap-1 px-2"
									disabled={action.disabled}
									onclick={action.onclick}
								>
									<action.icon class="size-4" />
									{#if action.chevron}
										<ChevronDown class="size-3 opacity-60" />
									{/if}
								</Button>
							</span>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>
						{action.disabled && action.disabledReason ? action.disabledReason : action.label}
					</Tooltip.Content>
				</Tooltip.Root>
			{/each}
			{#if groupIndex === 0}
				{#each searchMenus as menu (menu.kind)}
					<DropdownMenu.Root>
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props: tooltipProps })}
									<span {...tooltipProps} class="inline-flex">
										<DropdownMenu.Trigger>
											{#snippet child({ props })}
												<Button {...props} variant="ghost" class="gap-1 px-2">
													<menu.icon class="size-4" />
													<ChevronDown class="size-3 opacity-60" />
												</Button>
											{/snippet}
										</DropdownMenu.Trigger>
									</span>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content>{menu.label}</Tooltip.Content>
						</Tooltip.Root>
						<DropdownMenu.Content align="start">
							<SearchMenuForm
								attributes={searchAttributes}
								bind:enabledIds={searchEnabled[menu.kind]}
								bind:variance={searchVariance[menu.kind]}
							>
								{#snippet footer()}
									<Button
										disabled={searchEnabled[menu.kind].length === 0}
										onclick={() => submitSearch(menu.kind)}
									>
										{menu.kind === 'similar' ? 'Search modules for sale' : 'Search'}
									</Button>
								{/snippet}
							</SearchMenuForm>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				{/each}
			{/if}
		{/each}

		<DropdownMenu.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props: tooltipProps })}
						<span {...tooltipProps} class="inline-flex">
							<DropdownMenu.Trigger>
								{#snippet child({ props })}
									<Button {...props} variant="ghost" class="gap-1 px-2">
										<Ellipsis class="size-4" />
									</Button>
								{/snippet}
							</DropdownMenu.Trigger>
						</span>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>More actions</Tooltip.Content>
			</Tooltip.Root>
			<DropdownMenu.Content align="end">
				<DropdownMenu.Item onclick={() => copyImageLink(module)}>
					Copy image link
				</DropdownMenu.Item>
				<DropdownMenu.Item>
					{#snippet child({ props })}
						<a {...props} href="/og/module/{module.id}" download="{module.slug}.png">
							Download image
						</a>
					{/snippet}
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>
</Tooltip.Provider>
