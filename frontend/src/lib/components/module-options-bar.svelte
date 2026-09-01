<script lang="ts">
	// The bar above the browser grid switching display mode, attribute bar
	// mode and attribute scores. Mutates the shared settings state (instant
	// re-render of every card) and persists the cookies in the background
	// for the next visit, like the legacy ModuleOptions.vue — restyled as
	// segmented controls: one pill per mutually exclusive group, a switch
	// for the binary score overlay.
	import { ChevronLeft, ChevronRight, LayoutGrid, List, Table2 } from '@lucide/svelte';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { ATTRIBUTE_BAR_MODES, saveDisplaySettings, type DisplaySettings } from '$lib/display';
	import { buildQueryPath, type UiSearch } from '$lib/query';

	let {
		settings,
		search,
		prefix,
	}: { settings: DisplaySettings; search: UiSearch; prefix: string } = $props();

	const previousPage = $derived(buildQueryPath(prefix, { ...search, page: search.page - 1 }));
	const nextPage = $derived(buildQueryPath(prefix, { ...search, page: search.page + 1 }));

	function apply(change: Partial<DisplaySettings>) {
		Object.assign(settings, change);
		saveDisplaySettings({ ...settings });
	}

	const views = [
		{ value: 'grid', label: 'Grid view', icon: LayoutGrid },
		{ value: 'list', label: 'List view', icon: List },
		{ value: 'table', label: 'Table view', icon: Table2 },
	] as const;

	const barModeLabels: Record<string, string> = {
		default: 'Default',
		type: 'Type',
		absolute: 'Absolute',
		none: 'None',
	};

	const SEGMENT = 'flex h-7 items-center gap-1.5 rounded-[5px] px-2.5 text-xs transition-colors';
	const ACTIVE = 'bg-primary text-primary-foreground';
	const IDLE = 'text-muted-foreground hover:text-foreground';
</script>

<Tooltip.Provider delayDuration={300}>
	<div class="mb-2 flex flex-wrap items-center gap-x-6 gap-y-2 px-1 py-1">
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

		<!-- Deliberate divergence from the legacy ModuleOptions (which
		     always shows every control): the table view renders no roll
		     bars and always shows scores, so both controls hide there. -->
		{#if settings.display !== 'table'}
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
		{/if}

		<!-- The legacy prev/next pagination on the bar's right edge. -->
		<div class="ml-auto flex items-center gap-1">
			<span class="hud-label mr-1">Page {search.page}</span>
			{#if search.page > 1}
				<a
					href={previousPage}
					aria-label="Previous page"
					class="flex size-7 items-center justify-center rounded-[7px] border border-border bg-card-2 text-muted-foreground hover:text-foreground"
				>
					<ChevronLeft class="size-4" />
				</a>
			{:else}
				<span
					aria-disabled="true"
					class="flex size-7 items-center justify-center rounded-[7px] border border-border bg-card-2 text-muted-foreground/40"
				>
					<ChevronLeft class="size-4" />
				</span>
			{/if}
			<a
				href={nextPage}
				aria-label="Next page"
				class="flex size-7 items-center justify-center rounded-[7px] border border-border bg-card-2 text-muted-foreground hover:text-foreground"
			>
				<ChevronRight class="size-4" />
			</a>
		</div>
	</div>
</Tooltip.Provider>
