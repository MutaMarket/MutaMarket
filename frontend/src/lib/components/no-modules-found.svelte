<script lang="ts">
	// The empty state of every module display, the legacy
	// NoModulesFound.vue card.
	import { TriangleAlert } from '@lucide/svelte';
</script>

<div class="hud-frame col-span-full">
	<div class="flex items-center gap-4 p-6">
		<TriangleAlert class="size-8 shrink-0 text-primary" />
		<div>
			<span class="block text-lg font-medium">No modules found</span>
			<p>Try changing the filters or search query</p>
		</div>
	</div>
</div>
