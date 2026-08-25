<script lang="ts">
	// The category (type) picker, a one-to-one mirror of the legacy
	// TypeDialog.vue + TypeCategory.vue: a trigger button showing the
	// selected type and a full-screen dialog with the hardcoded catalog of
	// abyssal categories. Selecting a type navigates with only the legacy
	// getTypeLink subset of the current search carried over; clicking the
	// active type deselects it.
	import { CATALOG, type CatalogEntry } from '$lib/catalog';
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

<button
	class="flex h-10 w-full cursor-pointer items-center justify-between gap-2 rounded-md border border-border bg-card-2 px-3 py-2 text-start text-sm transition hover:brightness-125"
	onclick={() => (open = true)}
>
	{#if currentTypeId !== null}
		<img
			alt=""
			class="size-6 rounded-sm"
			src="https://images.evetech.net/types/{currentTypeId}/icon?size=64"
		/>
	{/if}
	<span class="truncate">{label}</span>
	<span aria-hidden="true" class="ml-auto shrink-0 opacity-50">{'▾'}</span>
</button>

{#if open}
	<div
		class="fixed inset-0 z-50 overflow-y-auto bg-black/80 p-4"
		role="presentation"
		onclick={() => (open = false)}
		onkeydown={(event) => {
			if (event.key === 'Escape') open = false;
		}}
	>
		<div
			class="mx-auto max-w-[1440px] rounded-lg border border-border bg-card"
			role="dialog"
			aria-label="Module categories"
			tabindex="-1"
			onclick={(event) => event.stopPropagation()}
			onkeydown={(event) => event.stopPropagation()}
		>
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
		</div>
	</div>
{/if}
