<script lang="ts">
	// The recommended-gear management page, the legacy Admin/GearItemPage:
	// an in-place create/edit form card above the list card (editing
	// scrolls back up to the form), plus toggle and delete. Divergence:
	// images are referenced by URL like the advertisements port - the
	// legacy file upload needs a public-disk story the rewrite does not
	// have yet.
	import { Pencil, ShoppingBag, Trash2 } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { notifyError, notifySuccess } from '$lib/toast';
	import type { PageProps } from './$types';
	import type { AdminGearItem } from './+page.server';

	let { data }: PageProps = $props();

	let editing = $state<AdminGearItem | null>(null);
	let confirmingDelete = $state<AdminGearItem | null>(null);
	let submitting = $state(false);

	// The form card's state, shared by create and edit.
	let form = $state({
		name: '',
		description: '',
		image_url: '',
		link: '',
		priority: 0,
		active: true
	});

	function resetForm() {
		form = { name: '', description: '', image_url: '', link: '', priority: 0, active: true };
	}

	function startEdit(gearItem: AdminGearItem) {
		form = {
			name: gearItem.name,
			description: gearItem.description ?? '',
			image_url: gearItem.image_url ?? '',
			link: gearItem.link,
			priority: gearItem.priority,
			active: gearItem.active
		};
		editing = gearItem;
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	function cancelEdit() {
		editing = null;
		resetForm();
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		try {
			const payload = {
				name: form.name,
				description: form.description || null,
				image_url: form.image_url,
				link: form.link,
				priority: form.priority,
				active: form.active
			};
			const response = await fetch(
				editing === null ? '/api/admin/gear-items' : `/api/admin/gear-items/${editing.id}`,
				{
					method: editing === null ? 'POST' : 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify(payload)
				}
			);
			if (response.ok) {
				notifySuccess(
					editing === null ? 'Gear item created!' : 'Gear item updated!',
					`${form.name} has been ${editing === null ? 'added' : 'saved'}.`
				);
				cancelEdit();
				await invalidateAll();
			} else {
				const body = (await response.json().catch(() => null)) as {
					errors?: Record<string, string[]>;
					message?: string;
				} | null;
				const first = body?.errors ? Object.values(body.errors)[0]?.[0] : undefined;
				notifyError('Not saved', first ?? body?.message ?? 'Something went wrong.');
			}
		} finally {
			submitting = false;
		}
	}

	async function toggle(gearItem: AdminGearItem) {
		await fetch(`/api/admin/gear-items/${gearItem.id}/toggle`, { method: 'PATCH' });
		notifySuccess(
			'Gear item updated!',
			gearItem.active ? 'Gear item deactivated.' : 'Gear item activated.'
		);
		await invalidateAll();
	}

	async function destroy() {
		if (confirmingDelete === null) return;
		await fetch(`/api/admin/gear-items/${confirmingDelete.id}`, { method: 'DELETE' });
		notifySuccess('Gear item deleted!', `${confirmingDelete.name} has been removed.`);
		confirmingDelete = null;
		await invalidateAll();
	}
</script>

<svelte:head><title>Admin - Gear Items - MutaMarket</title></svelte:head>

<PageHeader title="Gear Items" subtitle="The recommended-gear rotation in the sidebar">
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<ShoppingBag class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
	{#snippet actions()}
		<a class="text-sm text-muted-foreground hover:text-foreground" href="/admin">Dashboard</a>
	{/snippet}
</PageHeader>

<!-- The form card: create, or edit the selected item in place. -->
<div class="mb-6 rounded-lg border border-border bg-card p-4">
	<div class="mb-3">
		<span class="block text-lg font-medium">
			{editing === null ? 'Create Gear Item' : 'Edit Gear Item'}
		</span>
		<p class="text-sm text-muted-foreground">
			{editing === null
				? 'Add a product with an affiliate link to the sidebar'
				: 'Update the selected gear item'}
		</p>
	</div>
	<form class="grid gap-4" onsubmit={submit}>
		<div class="flex flex-col gap-1.5">
			<Label for="gear-name">Name</Label>
			<Input id="gear-name" bind:value={form.name} placeholder="e.g. Logitech G502 Mouse" required />
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="gear-description">Description (optional)</Label>
			<textarea
				id="gear-description"
				bind:value={form.description}
				placeholder="Short blurb shown under the product name"
				rows="2"
				class="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-xs transition-colors placeholder:text-muted-foreground focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-hidden"
			></textarea>
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="gear-link">Affiliate link</Label>
			<Input id="gear-link" bind:value={form.link} placeholder="https://geni.us/..." required type="url" />
			<p class="text-xs text-muted-foreground">
				The geni.us (or other affiliate) URL visitors are sent to
			</p>
		</div>
		<div class="flex flex-col gap-1.5">
			<Label for="gear-image">Image URL</Label>
			<Input id="gear-image" bind:value={form.image_url} placeholder="https://…" required type="url" />
			{#if form.image_url.startsWith('http')}
				<img
					src={form.image_url}
					alt="Current gear item"
					class="mt-1 aspect-square w-24 rounded-lg object-cover"
				/>
			{/if}
		</div>
		<div class="flex flex-col gap-1.5 sm:max-w-48">
			<Label for="gear-priority">Priority</Label>
			<Input id="gear-priority" bind:value={form.priority} type="number" min="0" />
			<p class="text-xs text-muted-foreground">Higher numbers appear first in the sidebar</p>
		</div>
		<div class="flex items-center gap-2">
			<Switch id="gear-active" bind:checked={form.active} />
			<Label for="gear-active">Active</Label>
		</div>
		<div class="flex gap-2">
			<Button type="submit" disabled={submitting || form.name === '' || form.image_url === '' || form.link === ''}>
				{editing === null ? 'Create Gear Item' : 'Save Changes'}
			</Button>
			{#if editing !== null}
				<Button type="button" variant="outline" onclick={cancelEdit}>Cancel</Button>
			{/if}
		</div>
	</form>
</div>

<!-- The list card. -->
<div class="rounded-lg border border-border bg-card p-4">
	<div class="mb-3">
		<span class="block text-lg font-medium">Gear Items</span>
		<p class="text-sm text-muted-foreground">{data.gearItems.length} total</p>
	</div>
	{#if data.gearItems.length === 0}
		<p class="text-sm text-muted-foreground">No gear items yet. Create one above.</p>
	{:else}
		<ul class="flex flex-col">
			{#each data.gearItems as gearItem (gearItem.id)}
				<li class="flex flex-wrap items-center gap-4 border-b border-border py-3 last:border-b-0 last:pb-0 first:pt-0">
					{#if gearItem.image_url}
						<img
							src={gearItem.image_url}
							alt={gearItem.name}
							class="aspect-square w-12 rounded object-cover"
						/>
					{:else}
						<div class="grid size-12 place-items-center rounded bg-card-2">
							<ShoppingBag class="size-5 text-muted-foreground" />
						</div>
					{/if}
					<div class="min-w-40 flex-1">
						<span class="font-medium">{gearItem.name}</span>
						<a
							href={gearItem.link}
							class="block max-w-64 truncate text-xs text-muted-foreground hover:underline"
							rel="noopener noreferrer"
							target="_blank"
						>
							{gearItem.link}
						</a>
					</div>
					<div class="text-xs text-muted-foreground">Priority: {gearItem.priority}</div>
					<div class="flex items-center gap-2">
						<Switch checked={gearItem.active} onCheckedChange={() => toggle(gearItem)} />
						<Button
							variant="ghost"
							size="icon"
							class="size-8"
							aria-label="Edit gear item"
							onclick={() => startEdit(gearItem)}
						>
							<Pencil class="size-4" />
						</Button>
						<Button
							variant="ghost"
							size="icon"
							class="size-8 text-muted-foreground hover:text-red-500"
							aria-label="Delete gear item"
							onclick={() => (confirmingDelete = gearItem)}
						>
							<Trash2 class="size-4" />
						</Button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<Dialog.Root open={confirmingDelete !== null} onOpenChange={(open) => !open && (confirmingDelete = null)}>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>Delete "{confirmingDelete?.name}"?</Dialog.Title>
			<Dialog.Description>The gear item leaves the rotation permanently.</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="secondary" onclick={() => (confirmingDelete = null)}>Cancel</Button>
			<Button variant="destructive" onclick={destroy}>Delete</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
