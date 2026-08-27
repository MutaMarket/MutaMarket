<script lang="ts">
	import { ChevronDown } from '@lucide/svelte';
	// The category (type) picker, a one-to-one mirror of the legacy
	// TypeDialog.vue + TypeCategory.vue: a trigger button showing the
	// selected type and a full-screen dialog with the hardcoded catalog of
	// abyssal categories. Selecting a type navigates with only the legacy
	// getTypeLink subset of the current search carried over; clicking the
	// active type deselects it. The dialog is the shadcn Dialog so it
	// portals to the body — rendered inline it would be capped by the
	// filter band's stacking context and end up under the table view's
	// sticky cells.
	import { CATALOG, iconForType, type CatalogEntry } from '$lib/catalog';
	import * as Dialog from '$lib/components/ui/dialog';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import { typeSwitchSearch } from '$lib/type-switch';

	let {
		prefix,
		search,
		currentTypeId = null,
		currentTypeName = null
	}: {
		prefix: string;
		search: UiSearch;
		currentTypeId?: number | null;
		currentTypeName?: string | null;
	} = $props();

	let open = $state(false);

	// The trigger label strips the mutation words, like the legacy dialog
	// (whose double space HTML rendering collapses).
	const label = $derived(
		currentTypeName?.replace('Abyssal', '').replace('Mutated', '').trim() ?? 'All'
	);

	function iconSrc(icon: string): string {
		return `/img/icons/${icon}.png`;
	}

	function typeHref(typeId: number): string {
		return buildQueryPath(prefix, typeSwitchSearch(search, currentTypeId, typeId));
	}

	function singleTypeId(entry: CatalogEntry): number {
		return Number(entry.icon);
	}
</script>

{#snippet typeLink(active: boolean)}
	<div
		class={active
			? 'grid size-5 place-items-center rounded border border-primary bg-primary text-foreground'
			: 'grid size-5 place-items-center rounded border border-border text-foreground *:opacity-0'}
	>
		<span aria-hidden="true">{'✓'}</span>
	</div>
{/snippet}

<Dialog.Root bind:open>
	<Dialog.Trigger>
		{#snippet child({ props })}
			<button
				{...props}
				class="flex h-10 w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-border bg-card-2 px-3 py-2 text-start text-sm transition hover:brightness-125 focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
			>
				{#if currentTypeId !== null}
					{@const stem = iconForType(currentTypeId)}
					<img
						alt=""
						class="size-6 rounded-sm"
						src={stem !== null
							? iconSrc(stem)
							: `https://images.evetech.net/types/${currentTypeId}/icon?size=64`}
					/>
				{/if}
				<span class="truncate">{label}</span>
				<ChevronDown aria-hidden="true" class="ml-auto size-3.5 shrink-0 text-muted-foreground" />
			</button>
		{/snippet}
	</Dialog.Trigger>
	<Dialog.Content
		showCloseButton={false}
		class="top-12 max-h-[calc(100vh-6rem)] w-full translate-y-0 gap-0 overflow-y-auto rounded-lg border border-border bg-card p-0 ring-0 sm:max-w-[1440px]"
	>
		<Dialog.Title class="sr-only">Module categories</Dialog.Title>
		<div class="grid gap-[1px] md:grid-cols-2 xl:grid-cols-3">
			{#each CATALOG as column, columnIndex (columnIndex)}
				<div>
					{#each column as section (section.title)}
						<div class="grid gap-1 p-4">
							<h3 class="mb-2 text-lg text-primary">{section.title}</h3>
							{#each section.entries as entry (entry.name)}
								{#if entry.variants.length === 0}
									<a
										class="flex items-center gap-2 p-1 text-muted-foreground transition-colors duration-150 hover:text-foreground"
										href={typeHref(singleTypeId(entry))}
										onclick={() => (open = false)}
									>
										{@render typeLink(currentTypeId === singleTypeId(entry))}
										<img alt={entry.name} class="h-6 w-6" src={iconSrc(entry.icon)} />
										<span class="text-sm">{entry.name}</span>
									</a>
								{:else}
									<div class="flex items-center gap-2">
										<img alt={entry.name} class="size-8 rounded-lg" src={iconSrc(entry.icon)} />
										<div>
											<h1 class="mb-1 text-sm font-medium">{entry.name}</h1>
											<div class="flex flex-wrap gap-2">
												{#each entry.variants as [variant, typeId] (typeId)}
													<a
														class="flex items-center gap-2 p-1 text-muted-foreground transition-colors duration-150 hover:text-foreground"
														href={typeHref(typeId)}
														onclick={() => (open = false)}
													>
														{@render typeLink(currentTypeId === typeId)}
														<small>{variant}</small>
													</a>
												{/each}
											</div>
										</div>
									</div>
								{/if}
							{/each}
						</div>
					{/each}
				</div>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
