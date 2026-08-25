<script lang="ts">
	// The bar above the browser grid switching display mode, attribute bar
	// mode and attribute scores. Mutates the shared settings state (instant
	// re-render of every card) and persists the cookies in the background
	// for the next visit, like the legacy ModuleOptions.vue.
	import { Button } from '$lib/components/ui/button';
	import {
		ATTRIBUTE_BAR_MODES,
		DISPLAY_VALUES,
		saveDisplaySettings,
		type DisplaySettings
	} from '$lib/display';

	let { settings }: { settings: DisplaySettings } = $props();

	function apply(change: Partial<DisplaySettings>) {
		Object.assign(settings, change);
		saveDisplaySettings({ ...settings });
	}

	const displayLabels: Record<string, string> = { grid: 'Grid', list: 'List', table: 'Table' };
	const barModeLabels: Record<string, string> = {
		default: 'Default',
		type: 'Type',
		absolute: 'Absolute',
		none: 'None'
	};
</script>

<div class="mb-2 flex flex-wrap items-center gap-4 rounded-lg border border-border bg-card-1 p-2">
	<div class="flex items-center gap-1">
		<span class="mr-1 text-xs text-muted-foreground">View</span>
		{#each DISPLAY_VALUES as display (display)}
			<Button
				variant={settings.display === display ? 'default' : 'outline'}
				size="sm"
				class="h-7 px-2 text-xs"
				disabled={display !== 'grid'}
				title={display !== 'grid' ? 'Coming soon' : undefined}
				onclick={() => apply({ display })}
			>
				{displayLabels[display]}
			</Button>
		{/each}
	</div>
	<div class="flex items-center gap-1">
		<span class="mr-1 text-xs text-muted-foreground">Bars</span>
		{#each ATTRIBUTE_BAR_MODES as mode (mode)}
			<Button
				variant={settings.attribute_bar_mode === mode ? 'default' : 'outline'}
				size="sm"
				class="h-7 px-2 text-xs"
				onclick={() => apply({ attribute_bar_mode: mode })}
			>
				{barModeLabels[mode]}
			</Button>
		{/each}
	</div>
	<div class="flex items-center gap-1">
		<span class="mr-1 text-xs text-muted-foreground">Scores</span>
		{#each [false, true] as scores (scores)}
			<Button
				variant={settings.show_attribute_scores === scores ? 'default' : 'outline'}
				size="sm"
				class="h-7 px-2 text-xs"
				onclick={() => apply({ show_attribute_scores: scores })}
			>
				{scores ? 'On' : 'Off'}
			</Button>
		{/each}
	</div>
</div>
