<script lang="ts">
	// The show-page toolbar, mirroring Show/ModuleToolbar.vue: ghost icon
	// buttons with tooltips, grouped by hairline dividers. The three
	// search menus are stubbed disabled until the search-menu port lands
	// (specs/module-show.md §5).
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
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import {
		copyContractLink,
		copyImageLink,
		copyItemLink,
		copyPyfa,
		shareModule
	} from '$lib/export';
	import type { ModuleDetail } from '$lib/types';

	let { module }: { module: ModuleDetail } = $props();

	async function openContractIngame() {
		await fetch('/ui/contract', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ contract_id: module.contract?.id }),
			redirect: 'manual'
		});
	}

	interface ToolbarAction {
		icon: typeof IconType;
		label: string;
		chevron?: boolean;
		disabled?: boolean;
		disabledReason?: string;
		onclick?: () => void;
	}

	const noContract = $derived(module.contract === null);

	const groups: ToolbarAction[][] = $derived([
		[
			{ icon: Search, label: 'Search this type', onclick: () => goto(`/modules/type/${module.type.id}`) },
			{
				icon: GitCompareArrows,
				label: 'Search similar',
				chevron: true,
				disabled: true,
				disabledReason: 'Search menus are coming soon'
			},
			{
				icon: TrendingDown,
				label: 'Search cheapest',
				chevron: true,
				disabled: true,
				disabledReason: 'Search menus are coming soon'
			},
			{
				icon: RotateCcwClock,
				label: 'Search historic',
				chevron: true,
				disabled: true,
				disabledReason: 'Search menus are coming soon'
			}
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
		{/each}

		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<span {...props} class="inline-flex">
						<Button variant="ghost" class="gap-1 px-2" title="More">
							<Ellipsis class="size-4" />
						</Button>
					</span>
				{/snippet}
			</DropdownMenu.Trigger>
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
