<script lang="ts">
	// The toast stack, mounted once in the layout.
	import { dismiss, toasts } from '$lib/toast.svelte';
</script>

{#if toasts().length > 0}
	<div class="fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2">
		{#each toasts() as toast (toast.id)}
			<button
				class="rounded-md border border-border bg-popover p-3 text-left shadow-lg"
				onclick={() => dismiss(toast.id)}
			>
				<p
					class="text-sm font-semibold {toast.kind === 'success'
						? 'text-positive'
						: 'text-negative'}"
				>
					{toast.title}
				</p>
				<p class="mt-0.5 text-xs text-muted-foreground">{toast.body}</p>
			</button>
		{/each}
	</div>
{/if}
