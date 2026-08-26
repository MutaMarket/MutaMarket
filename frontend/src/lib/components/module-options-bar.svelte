<script lang="ts">
	// The bar above the browser grid switching display mode, attribute bar
	// mode and attribute scores. Mutates the shared settings state (instant
	// re-render of every card) and persists the cookies in the background
	// for the next visit, like the legacy ModuleOptions.vue — restyled as
	// segmented controls: one pill per mutually exclusive group, a switch
	// for the binary score overlay.
	import { LayoutGrid, List, Table2 } from '@lucide/svelte';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import {
		ATTRIBUTE_BAR_MODES,
		saveDisplaySettings,
		type DisplaySettings
	} from '$lib/display';

	let { settings }: { settings: DisplaySettings } = $props();

	function apply(change: Partial<DisplaySettings>) {
		Object.assign(settings, change);
		saveDisplaySettings({ ...settings });
	}

	const views = [
		{ value: 'grid', label: 'Grid view', icon: LayoutGrid, ready: true },
		{ value: 'list', label: 'List view (coming soon)', icon: List, ready: false },
		{ value: 'table', label: 'Table view (coming soon)', icon: Table2, ready: false }
	] as const;

	const barModeLabels: Record<string, string> = {
		default: 'Default',
		type: 'Type',
		absolute: 'Absolute',
		none: 'None'
	};

	const SEGMENT =
		'flex h-7 items-center gap-1.5 rounded-[5px] px-2.5 text-xs transition-colors ' +
		'disabled:cursor-not-allowed disabled:opacity-40';
	const ACTIVE = 'bg-primary text-primary-foreground';
	const IDLE = 'text-muted-foreground hover:text-foreground';
</script>

<Tooltip.Provider delayDuration={300}>
	<div
		class="mb-2 flex flex-wrap items-center gap-x-6 gap-y-2 rounded-lg border border-border bg-card-1 px-3 py-2"
	>
		<div class="flex items-center gap-2">
			<span class="hud-label">View</span>
			<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
				{#each views as view (view.value)}
					<Tooltip.Root>
						<Tooltip.Trigger>
							{#snippet child({ props })}
								<button
									{...props}
									type="button"
									class="{SEGMENT} {settings.display === view.value ? ACTIVE : IDLE}"
									disabled={!view.ready}
									aria-label={view.label}
									onclick={() => apply({ display: view.value })}
								>
									<view.icon class="size-3.5" />
								</button>
							{/snippet}
						</Tooltip.Trigger>
						<Tooltip.Content>{view.label}</Tooltip.Content>
					</Tooltip.Root>
				{/each}
			</div>
		</div>

		<div class="flex items-center gap-2">
			<span class="hud-label">Roll bars</span>
			<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
				{#each ATTRIBUTE_BAR_MODES as mode (mode)}
					<button
						type="button"
						class="{SEGMENT} {settings.attribute_bar_mode === mode ? ACTIVE : IDLE}"
						onclick={() => apply({ attribute_bar_mode: mode })}
					>
						{barModeLabels[mode]}
					</button>
				{/each}
			</div>
		</div>

		<div class="flex items-center gap-2">
			<Switch
				id="show-scores"
				checked={settings.show_attribute_scores}
				onCheckedChange={(on) => apply({ show_attribute_scores: on })}
			/>
			<Label for="show-scores" class="hud-label cursor-pointer">Scores</Label>
		</div>
	</div>
</Tooltip.Provider>
