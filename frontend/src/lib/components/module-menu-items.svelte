<script lang="ts">
	// The module menu body, mirroring Menus/ModuleDropdownMenu.vue and
	// ModuleContextMenu.vue (same menu, different primitives — `kind`
	// picks the set): three variance-search submenus, the collections
	// submenu with membership toggles, the workbench toggle, share, copy
	// and image groups, and the admin estimate action. Note and
	// asking-price entries arrive with their backend features.
	import {
		Copy,
		ExternalLink,
		FlaskConical,
		Image as ImageIcon,
		Layers,
		Plus,
		Search,
		Sparkles,
		SquareArrowOutUpRight
	} from '@lucide/svelte';
	import SearchMenuForm from './search-menu-form.svelte';
	import { goto, invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import * as ContextMenu from '$lib/components/ui/context-menu';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import {
		copyContractLink,
		copyImageLink,
		copyItemLink,
		copyPageLink,
		copyPyfa
	} from '$lib/export';
	import {
		cheapestSearchPath,
		historicSearchPath,
		similarSearchPath
	} from '$lib/module-finder';
	import { typeStatistics } from '$lib/abyssal-statistics';
	import { notifySuccess, notifyError } from '$lib/toast';
	import { addToWorkbench, removeFromWorkbench, workbenchEntries } from '$lib/workbench';
	import type { AbyssalTypeStatistic, ModuleDetail } from '$lib/types';

	let {
		module,
		statistics: providedStatistics = null,
		kind
	}: {
		module: ModuleDetail;
		statistics?: AbyssalTypeStatistic[] | null;
		kind: 'dropdown' | 'context';
	} = $props();

	// Cards outside the show page have no SSR-provided statistics; fall
	// back to the cached client fetch (the legacy client-bundled static).
	let fetchedStatistics: AbyssalTypeStatistic[] = $state([]);
	$effect(() => {
		if (providedStatistics === null) {
			typeStatistics(module.type.id).then((rows) => (fetchedStatistics = rows));
		}
	});
	const statistics = $derived(providedStatistics ?? fetchedStatistics);

	// svelte-ignore state_referenced_locally -- the primitive set never
	// changes after mount
	const Menu = kind === 'dropdown' ? DropdownMenu : ContextMenu;

	const isAdmin = $derived(Boolean(page.data.nav?.user?.is_admin));

	const searchAttributes = $derived(
		module.mutated_attributes.map((attribute) => ({
			id: attribute.id,
			display_name: attribute.display_name
		}))
	);

	// Per-submenu form state, like the legacy useSimilar/useCheapest.
	let similarVariance = $state(1);
	let similarEnabled: number[] = $state([]);
	let cheapestVariance = $state(1);
	let cheapestEnabled: number[] = $state([]);
	let historicVariance = $state(1);
	let historicEnabled: number[] = $state([]);

	const signedIn = $derived(Boolean(page.data.nav?.user));
	const benchedEntry = $derived(
		$workbenchEntries.find((entry) => entry.module.id === module.id) ?? null
	);

	// The user's collections with this module's membership, fetched when
	// the menu content mounts (menus render lazily on open).
	interface MenuCollection {
		id: number;
		name: string;
		slug: string;
		collection_module_id: number | null;
	}
	let collections = $state<MenuCollection[]>([]);
	async function refreshCollections() {
		const response = await fetch(`/api/collections/module/${module.id}`);
		if (response.ok) {
			collections = await response.json();
		}
	}
	$effect(() => {
		if (signedIn) {
			void refreshCollections();
		}
	});

	async function toggleCollection(collection: MenuCollection) {
		if (collection.collection_module_id !== null) {
			await fetch(`/collection-modules/${collection.collection_module_id}`, {
				method: 'DELETE',
				redirect: 'manual'
			});
			notifySuccess('Module removed', `Removed from ${collection.name}.`);
		} else {
			await fetch('/collection-modules', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ collection_id: collection.id, module_id: module.id }),
				redirect: 'manual'
			});
			notifySuccess('Module added', `Added to ${collection.name}.`);
		}
		await refreshCollections();
	}

	// The legacy createCollectionAndAddModule defaults.
	async function createCollectionWithModule() {
		await fetch('/collections/modules', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				name: 'New Collection',
				description: 'New collection',
				visibility: 'private',
				modules: [module.id]
			}),
			redirect: 'manual'
		});
		notifySuccess('Collection created', 'The module is in your new collection.');
		await refreshCollections();
	}

	async function estimateValue() {
		const response = await fetch(`/estimate/${module.id}`, {
			method: 'POST',
			redirect: 'manual'
		});
		if (response.type === 'opaqueredirect' || response.ok) {
			notifySuccess('Estimate updated', 'The estimate has been refreshed.');
			await invalidateAll();
		} else {
			notifyError('Estimate failed', 'The estimate could not be refreshed.');
		}
	}
</script>

<Menu.Sub>
	<Menu.SubTrigger>
		<Search class="size-4" />
		Search for similar
	</Menu.SubTrigger>
	<Menu.SubContent>
		<SearchMenuForm
			attributes={searchAttributes}
			bind:enabledIds={similarEnabled}
			bind:variance={similarVariance}
		>
			{#snippet footer()}
				<Button
					disabled={similarEnabled.length === 0}
					onclick={(event) => {
						event.stopPropagation();
						goto(similarSearchPath(module, statistics, similarEnabled, similarVariance));
					}}
				>
					Search modules for sale
				</Button>
			{/snippet}
		</SearchMenuForm>
	</Menu.SubContent>
</Menu.Sub>
<Menu.Sub>
	<Menu.SubTrigger>
		<Search class="size-4" />
		Search for cheapest
	</Menu.SubTrigger>
	<Menu.SubContent>
		<SearchMenuForm
			attributes={searchAttributes}
			bind:enabledIds={cheapestEnabled}
			bind:variance={cheapestVariance}
		>
			{#snippet footer()}
				<Button
					disabled={cheapestEnabled.length === 0}
					onclick={(event) => {
						event.stopPropagation();
						goto(cheapestSearchPath(module, statistics, cheapestEnabled, cheapestVariance));
					}}
				>
					Search
				</Button>
			{/snippet}
		</SearchMenuForm>
	</Menu.SubContent>
</Menu.Sub>
<Menu.Sub>
	<Menu.SubTrigger>
		<Search class="size-4" />
		Search for historic
	</Menu.SubTrigger>
	<Menu.SubContent>
		<SearchMenuForm
			attributes={searchAttributes}
			bind:enabledIds={historicEnabled}
			bind:variance={historicVariance}
		>
			{#snippet footer()}
				<Button
					disabled={historicEnabled.length === 0}
					onclick={(event) => {
						event.stopPropagation();
						goto(historicSearchPath(module, statistics, historicEnabled, historicVariance));
					}}
				>
					Search
				</Button>
			{/snippet}
		</SearchMenuForm>
	</Menu.SubContent>
</Menu.Sub>
{#if signedIn}
	<Menu.Separator />
	<Menu.Sub>
		<Menu.SubTrigger>
			<Layers class="size-4" />
			Collections
		</Menu.SubTrigger>
		<Menu.SubContent class="max-h-60 max-w-64 overflow-y-auto">
			{#each collections as collection (collection.id)}
				<Menu.Item closeOnSelect={false} onclick={() => toggleCollection(collection)}>
					<Plus
						class="size-3.5 transition-all duration-500 {collection.collection_module_id !== null
							? 'rotate-45 text-red-500'
							: 'text-green-500'}"
					/>
					<span class="min-w-0 truncate">{collection.name}</span>
					<a
						href="/collections/{collection.slug}"
						onclick={(event) => event.stopPropagation()}
						class="ml-auto text-muted-foreground hover:text-foreground"
						aria-label="Open collection"
					>
						<SquareArrowOutUpRight class="size-3" />
					</a>
				</Menu.Item>
			{/each}
			{#if collections.length > 0}
				<Menu.Separator />
			{/if}
			<Menu.Item closeOnSelect={false} onclick={createCollectionWithModule}>
				<Plus class="size-3.5 text-green-500" />
				Create collection
			</Menu.Item>
		</Menu.SubContent>
	</Menu.Sub>
	{#if benchedEntry !== null}
		<Menu.Item onclick={() => removeFromWorkbench(benchedEntry.id)}>
			<FlaskConical class="size-4" />
			Remove from workbench
		</Menu.Item>
	{:else}
		<Menu.Item onclick={() => addToWorkbench(module.id)}>
			<FlaskConical class="size-4" />
			Add to workbench
		</Menu.Item>
	{/if}
{/if}
<Menu.Separator />
<Menu.Item onclick={() => copyPageLink(module)}>
	<ExternalLink class="size-4" />
	Share module
</Menu.Item>
{#if module.contract}
	<Menu.Item
		onclick={() =>
			fetch('/ui/contract', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ contract_id: module.contract?.id }),
				redirect: 'manual'
			})}
	>
		<ExternalLink class="size-4" />
		Open contract in game
	</Menu.Item>
{/if}
<Menu.Separator />
<Menu.Item onclick={() => copyPyfa(module)}>
	<Copy class="size-4" />
	Copy Pyfa stats
</Menu.Item>
<Menu.Item onclick={() => copyItemLink(module)}>
	<Copy class="size-4" />
	Copy item link
</Menu.Item>
{#if module.contract}
	<Menu.Item onclick={() => copyContractLink(module)}>
		<Copy class="size-4" />
		Copy contract link
	</Menu.Item>
{/if}
<Menu.Separator />
<Menu.Item onclick={() => copyImageLink(module)}>
	<ImageIcon class="size-4" />
	Copy image link
</Menu.Item>
<Menu.Item>
	{#snippet child({ props })}
		<a {...props} href="/og/module/{module.id}" download="{module.slug}.png">
			<ImageIcon class="size-4" />
			Download image
		</a>
	{/snippet}
</Menu.Item>
{#if isAdmin}
	<Menu.Separator />
	<Menu.Item onclick={estimateValue}>
		<Sparkles class="size-4" />
		Estimate value
	</Menu.Item>
{/if}
