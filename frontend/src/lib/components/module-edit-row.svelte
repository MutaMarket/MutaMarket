<script lang="ts">
	// One card row for the note / collection-note / asking-price edit
	// modes, the legacy Grid/Note.vue, Grid/CollectionNote.vue and
	// Grid/AskingPrice.vue collapsed into one: it shows what is stored,
	// and swaps to a field while its mode is being edited.
	import { Coins, NotebookPen } from '@lucide/svelte';
	import { Input } from '$lib/components/ui/input';
	import { toIskCompact } from '$lib/format-number';
	import {
		collectionNote,
		draftValue,
		editSession,
		note,
		parsePrice,
		setDraft,
		showsEditRow,
		type EditMode
	} from '$lib/module-edits';
	import type { ModuleDetail } from '$lib/types';

	let {
		module,
		mode,
		allowed
	}: {
		module: ModuleDetail;
		mode: EditMode;
		/** Whether this viewer may edit this mode on this module. */
		allowed: boolean;
	} = $props();

	const session = $derived($editSession);
	const visible = $derived(showsEditRow(mode, module, session, allowed));
	const editing = $derived(session !== null && session.mode === mode && allowed);
	const value = $derived(session === null ? '' : draftValue(session, module));

	const stored = $derived(
		mode === 'note' ? (note(module)?.content ?? '') : (collectionNote(module)?.content ?? '')
	);
	const price = $derived(parsePrice(value));
</script>

{#if visible}
	<div class="grid h-[50px] grid-cols-[36px_1fr] items-center border-t border-border bg-card px-2">
		<div class="grid place-items-center {mode === 'price' ? 'text-amber-500' : 'text-lime-500'}">
			{#if mode === 'price'}
				<Coins stroke-width={1} class="h-[1em] w-[1em]" />
			{:else}
				<NotebookPen stroke-width={1} class="h-[1em] w-[1em]" />
			{/if}
		</div>
		{#if editing && mode === 'price'}
			<div class="grid grid-cols-[1fr_auto] items-center gap-2">
				<Input
					{value}
					inputmode="numeric"
					aria-label="Asking price"
					placeholder="Asking price"
					class="h-8 text-right"
					oninput={(event) => setDraft(module, event.currentTarget.value)}
				/>
				<span class="text-xs whitespace-nowrap text-muted-foreground">
					{price === null ? 'Not a number' : price > 0 ? toIskCompact(price) : 'No price'}
				</span>
			</div>
		{:else if editing}
			<textarea
				{value}
				rows="1"
				aria-label={mode === 'note' ? 'Note' : 'Collection note'}
				placeholder="Add a note"
				class="w-full resize-none border border-border bg-background px-2 py-1 text-sm focus:outline-none"
				oninput={(event) => setDraft(module, event.currentTarget.value)}
			></textarea>
		{:else}
			<p class="truncate text-center text-sm">{stored}</p>
		{/if}
	</div>
{/if}
