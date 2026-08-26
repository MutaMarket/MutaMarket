<script lang="ts">
	// The create-collection dialog, the legacy CreateCollectionDialog.vue:
	// name, description and visibility, posting to the collections store
	// and following its redirect to the fresh collection.
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import * as RadioGroup from '$lib/components/ui/radio-group';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	const VISIBILITY_HELP: Record<string, string> = {
		public: 'Public collections are visible to everyone and they might be featured on the homepage.',
		private: 'Private collections are only visible to you.',
		unlisted: 'Unlisted collections are visible to anyone with the link.'
	};

	let name = $state('');
	let description = $state('');
	let visibility = $state('private');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		error = null;
		try {
			const response = await fetch('/collections', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ name, description: description || null, visibility }),
				redirect: 'follow'
			});
			if (response.redirected) {
				open = false;
				await goto(new URL(response.url).pathname);
				return;
			}
			const body: { message?: string; errors?: Record<string, string[]> } = await response
				.json()
				.catch(() => ({}));
			error =
				Object.values(body.errors ?? {})[0]?.[0] ?? body.message ?? 'Could not create the collection.';
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content>
		<Dialog.Title>Create a new collection</Dialog.Title>
		<Dialog.Description>
			Create a new collection to organize your assets. You can add assets to this collection later.
		</Dialog.Description>
		<form class="grid gap-4" onsubmit={submit}>
			<div class="grid gap-1.5">
				<Label for="collection-name">Name</Label>
				<Input id="collection-name" bind:value={name} type="text" />
			</div>
			<div class="grid gap-1.5">
				<Label for="collection-description">Description</Label>
				<textarea
					id="collection-description"
					bind:value={description}
					rows="3"
					class="w-full resize-none rounded-md border border-border bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
				></textarea>
			</div>
			<div class="grid gap-1.5">
				<Label>Visibility</Label>
				<RadioGroup.Root bind:value={visibility} class="flex gap-4">
					{#each ['private', 'unlisted', 'public'] as option (option)}
						<label class="flex items-center gap-2 text-sm capitalize">
							<RadioGroup.Item value={option} />
							{option}
						</label>
					{/each}
				</RadioGroup.Root>
				<p class="text-xs text-muted-foreground">{VISIBILITY_HELP[visibility]}</p>
			</div>
			{#if error}
				<p class="text-sm text-negative">{error}</p>
			{/if}
			<Dialog.Footer>
				<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={submitting || name.trim() === ''}>Create Collection</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
