<script lang="ts">
	// The advertisement management page, the legacy
	// Admin/AdvertisementPage: the sidebar rotation's entries with their
	// derived status, scheduling and priority, plus create/edit/toggle/
	// delete. Divergence: images are referenced by URL - the legacy file
	// upload needs a public-disk story the rewrite does not have yet.
	import { Megaphone, Pencil, Plus, Trash2 } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { notifyError, notifySuccess } from '$lib/toast';
	import type { PageProps } from './$types';
	import type { AdminAdvertisement } from './+page.server';

	let { data }: PageProps = $props();

	const STATUS_STYLES: Record<string, string> = {
		live: 'bg-primary/15 text-primary',
		scheduled: 'bg-blue-500/15 text-blue-400',
		expired: 'bg-amber-500/15 text-amber-400',
		inactive: 'bg-card-2 text-muted-foreground'
	};

	let editing = $state<AdminAdvertisement | null>(null);
	let creating = $state(false);
	let confirmingDelete = $state<AdminAdvertisement | null>(null);
	let submitting = $state(false);

	// The dialog's form state, shared by create and edit.
	let form = $state({
		name: '',
		description: '',
		image_url: '',
		link: '',
		priority: 0,
		starts_at: '',
		expires_at: '',
		active: true
	});

	function openCreate() {
		form = {
			name: '',
			description: '',
			image_url: '',
			link: '',
			priority: 0,
			starts_at: '',
			expires_at: '',
			active: true
		};
		editing = null;
		creating = true;
	}

	/** timestamptz text -> the datetime-local input format. */
	function toLocalInput(timestamp: string | null): string {
		if (timestamp === null) return '';
		const parsed = new Date(timestamp.replace(' ', 'T'));
		if (Number.isNaN(parsed.getTime())) return '';
		const pad = (value: number) => String(value).padStart(2, '0');
		return `${parsed.getFullYear()}-${pad(parsed.getMonth() + 1)}-${pad(parsed.getDate())}T${pad(parsed.getHours())}:${pad(parsed.getMinutes())}`;
	}

	function openEdit(advertisement: AdminAdvertisement) {
		form = {
			name: advertisement.name,
			description: advertisement.description ?? '',
			image_url: advertisement.image_url ?? '',
			link: advertisement.link ?? '',
			priority: advertisement.priority,
			starts_at: toLocalInput(advertisement.starts_at),
			expires_at: toLocalInput(advertisement.expires_at),
			active: advertisement.active
		};
		editing = advertisement;
		creating = true;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		try {
			const payload = {
				name: form.name,
				description: form.description || null,
				image_url: form.image_url,
				link: form.link || null,
				priority: form.priority,
				starts_at: form.starts_at === '' ? null : new Date(form.starts_at).toISOString(),
				expires_at: form.expires_at === '' ? null : new Date(form.expires_at).toISOString(),
				active: form.active
			};
			const response = await fetch(
				editing === null
					? '/api/admin/advertisements'
					: `/api/admin/advertisements/${editing.id}`,
				{
					method: editing === null ? 'POST' : 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify(payload)
				}
			);
			if (response.ok) {
				notifySuccess(
					editing === null ? 'Advertisement created!' : 'Advertisement updated!',
					`${form.name} has been ${editing === null ? 'added' : 'saved'}.`
				);
				creating = false;
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

	async function toggle(advertisement: AdminAdvertisement) {
		await fetch(`/api/admin/advertisements/${advertisement.id}/toggle`, { method: 'PATCH' });
		notifySuccess(
			'Advertisement updated!',
			advertisement.active ? 'Advertisement deactivated.' : 'Advertisement activated.'
		);
		await invalidateAll();
	}

	async function destroy() {
		if (confirmingDelete === null) return;
		await fetch(`/api/admin/advertisements/${confirmingDelete.id}`, { method: 'DELETE' });
		notifySuccess('Advertisement deleted!', `${confirmingDelete.name} has been removed.`);
		confirmingDelete = null;
		await invalidateAll();
	}

	function schedule(advertisement: AdminAdvertisement): string {
		const from = advertisement.starts_at?.slice(0, 16).replace('T', ' ') ?? null;
		const to = advertisement.expires_at?.slice(0, 16).replace('T', ' ') ?? null;
		if (from === null && to === null) return 'Always';
		return `${from ?? '…'} → ${to ?? '…'}`;
	}
</script>

<svelte:head><title>Advertisements - MutaMarket</title></svelte:head>

<PageHeader title="Advertisements" subtitle="The sidebar rotation, scheduled and prioritized">
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<Megaphone class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
	{#snippet actions()}
		<div class="flex items-center gap-3">
			<a class="text-sm text-muted-foreground hover:text-foreground" href="/admin">Dashboard</a>
			<Button class="h-8 gap-2" onclick={openCreate}>
				<Plus class="size-4" />
				New advertisement
			</Button>
		</div>
	{/snippet}
</PageHeader>

{#if data.advertisements.length === 0}
	<div class="hud-frame p-6">
		<span class="block text-lg font-medium">No advertisements yet</span>
		<p class="text-muted-foreground">Create one to start the sidebar rotation.</p>
	</div>
{:else}
	<ul class="flex flex-col gap-2">
		{#each data.advertisements as advertisement (advertisement.id)}
			<li class="flex items-center gap-4 rounded-lg border border-border bg-card p-3">
				{#if advertisement.image_url}
					<img
						src={advertisement.image_url}
						alt=""
						class="size-12 shrink-0 rounded object-cover"
					/>
				{:else}
					<div class="grid size-12 shrink-0 place-items-center rounded bg-card-2">
						<Megaphone class="size-5 text-muted-foreground" />
					</div>
				{/if}
				<div class="min-w-0 grow">
					<span class="flex items-center gap-2">
						<span class="truncate text-sm font-medium">{advertisement.name}</span>
						<span
							class="rounded-full px-2 py-0.5 text-[10px] uppercase {STATUS_STYLES[
								advertisement.status
							]}"
						>
							{advertisement.status}
						</span>
					</span>
					<span class="block truncate text-xs text-muted-foreground">
						{schedule(advertisement)} · priority {advertisement.priority}
						{#if advertisement.link}
							· {advertisement.link}
						{/if}
					</span>
				</div>
				<Switch
					checked={advertisement.active}
					onCheckedChange={() => toggle(advertisement)}
				/>
				<Button
					variant="ghost"
					size="icon"
					class="size-8"
					aria-label="Edit advertisement"
					onclick={() => openEdit(advertisement)}
				>
					<Pencil class="size-4" />
				</Button>
				<Button
					variant="ghost"
					size="icon"
					class="size-8 text-muted-foreground hover:text-red-500"
					aria-label="Delete advertisement"
					onclick={() => (confirmingDelete = advertisement)}
				>
					<Trash2 class="size-4" />
				</Button>
			</li>
		{/each}
	</ul>
{/if}

<Dialog.Root bind:open={creating}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{editing === null ? 'New advertisement' : `Edit ${editing.name}`}</Dialog.Title>
			<Dialog.Description>
				Shown in the sidebar rotation at 250×300; higher priority rotates first.
			</Dialog.Description>
		</Dialog.Header>
		<form class="flex flex-col gap-3" onsubmit={submit}>
			<div class="flex flex-col gap-1.5">
				<Label for="ad-name">Name</Label>
				<Input id="ad-name" bind:value={form.name} required />
			</div>
			<div class="flex flex-col gap-1.5">
				<Label for="ad-image">Image URL</Label>
				<Input
					id="ad-image"
					bind:value={form.image_url}
					placeholder="https://…"
					required
					type="url"
				/>
				{#if form.image_url.startsWith('http')}
					<img
						src={form.image_url}
						alt="Preview"
						class="mt-1 aspect-[250/300] w-24 rounded object-cover"
					/>
				{/if}
			</div>
			<div class="flex flex-col gap-1.5">
				<Label for="ad-link">Link</Label>
				<Input id="ad-link" bind:value={form.link} placeholder="https://…" type="url" />
			</div>
			<div class="flex flex-col gap-1.5">
				<Label for="ad-description">Description</Label>
				<Input id="ad-description" bind:value={form.description} />
			</div>
			<div class="grid grid-cols-3 gap-3">
				<div class="flex flex-col gap-1.5">
					<Label for="ad-priority">Priority</Label>
					<Input id="ad-priority" bind:value={form.priority} type="number" min="0" />
				</div>
				<div class="flex flex-col gap-1.5">
					<Label for="ad-starts">Starts</Label>
					<Input id="ad-starts" bind:value={form.starts_at} type="datetime-local" />
				</div>
				<div class="flex flex-col gap-1.5">
					<Label for="ad-expires">Expires</Label>
					<Input id="ad-expires" bind:value={form.expires_at} type="datetime-local" />
				</div>
			</div>
			<div class="flex items-center gap-2">
				<Switch id="ad-active" bind:checked={form.active} />
				<Label for="ad-active">Active</Label>
			</div>
			<Dialog.Footer>
				<Button type="button" variant="secondary" onclick={() => (creating = false)}>
					Cancel
				</Button>
				<Button type="submit" disabled={submitting || form.name === '' || form.image_url === ''}>
					{editing === null ? 'Create' : 'Save'}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root open={confirmingDelete !== null} onOpenChange={(open) => !open && (confirmingDelete = null)}>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>Delete {confirmingDelete?.name}?</Dialog.Title>
			<Dialog.Description>The advertisement leaves the rotation permanently.</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="secondary" onclick={() => (confirmingDelete = null)}>Cancel</Button>
			<Button variant="destructive" onclick={destroy}>Delete</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
