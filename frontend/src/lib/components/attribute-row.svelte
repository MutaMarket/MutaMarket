<script lang="ts">
	// One attribute row of a module card, mirroring the legacy
	// Grid/Attribute.vue: 36px attribute icon spanning two rows, the name
	// over the value+difference cluster (the score column always occupies
	// column 3, so the cluster flows to the second row), then the roll bar
	// in the mode the display settings select (legacy Bars/*.vue).
	import GameImage from './game-image.svelte';
	import {
		attributeFormattedDifference,
		attributeFormattedValue,
		attributeScoreClass,
		attributeScoreLabel,
		attributeVariant
	} from '$lib/attributes';
	import type { DisplaySettings } from '$lib/display';
	import type { ModuleAttributeView } from '$lib/types';

	let {
		attribute,
		settings
	}: { attribute: ModuleAttributeView; settings: DisplaySettings } = $props();

	/** Tick marks of the absolute bar, like the legacy component. */
	const ABSOLUTE_BAR_STEPS = 20;

	const variant = $derived(attributeVariant(attribute));
	const displayName = $derived(
		attribute.display_name === '' ? attribute.name : attribute.display_name
	);

	const variantText: Record<string, string> = {
		gold: 'text-gold',
		diamond: 'text-diamond',
		brown: 'text-brown',
		positive: 'text-positive',
		'positive-derived': 'text-positive-derived',
		'negative-derived': 'text-negative-derived',
		negative: 'text-negative'
	};

	const variantFill: Record<string, string> = {
		gold: 'attribute-gold',
		diamond: 'attribute-diamond',
		brown: 'attribute-brown',
		positive: 'attribute-positive',
		'positive-derived': 'attribute-positive-derived',
		'negative-derived': 'attribute-negative-derived',
		negative: 'attribute-negative'
	};

	const absoluteFill = $derived(
		attribute.bar === 1
			? 'attribute-gold'
			: attribute.bar === 2
				? 'attribute-diamond'
				: attribute.bar === -1
					? 'attribute-brown'
					: 'attribute-absolute'
	);

	const defaultWidth = $derived(Math.min(Math.abs(attribute.fraction) * 50, 50));
	const typeWidth = $derived(Math.min(Math.abs(attribute.fraction_type) * 50, 50));
	const absoluteWidth = $derived(Math.min(Math.max(attribute.fraction_absolute * 100, 0), 100));
</script>

<div class="grid grid-cols-[36px_1fr_auto] content-center items-center gap-x-2 bg-card-2 px-2 py-1">
	<GameImage
		src="/img/icons/{attribute.id}.png"
		alt={attribute.name}
		class="row-span-2 size-8"
	/>
	<div class="text-xs text-muted-foreground">{displayName}</div>
	<div class="flex gap-1 text-sm text-foreground">
		<span>{attributeFormattedValue(attribute)}</span>
		<span class={variantText[variant]}>{attributeFormattedDifference(attribute)}</span>
	</div>
	<div class="col-start-3 row-span-2 row-start-1">
		{#if settings.show_attribute_scores}
			<span class="inline-block text-sm font-medium {attributeScoreClass(attribute)}"
				>{attributeScoreLabel(attribute)}</span
			>
		{/if}
	</div>
	{#if settings.attribute_bar_mode !== 'none'}
		<div class="col-span-full my-1">
			{#if settings.attribute_bar_mode === 'type'}
				<!-- The fill shows fraction_type against the whole type's range; the
				     band highlights the share the module's own mutaplasmid can roll. -->
				<div class="relative h-[3px] bg-background">
					{#if attribute.type_band}
						<div
							class="absolute top-0 bottom-0 left-0 bg-white/25"
							style="left: {50 - attribute.type_band[0] * 50}%; right: {50 -
								attribute.type_band[1] * 50}%"
						></div>
					{/if}
					{#if attribute.fraction_type >= 0}
						<div
							class="absolute left-1/2 h-full origin-left border-r border-white"
							style="width: {typeWidth}%"
						>
							<div class="h-full w-full {variantFill[variant]}"></div>
						</div>
					{:else}
						<div
							class="absolute right-1/2 h-full origin-right border-l border-white"
							style="width: {typeWidth}%"
						>
							<div class="h-full w-full {variantFill[variant]}"></div>
						</div>
					{/if}
				</div>
			{:else if settings.attribute_bar_mode === 'absolute'}
				<!-- Left origin over the whole 0..1 absolute fraction with tick marks. -->
				<div class="relative h-[3px] bg-card">
					{#each { length: ABSOLUTE_BAR_STEPS } as _, index (index)}
						<div
							class="absolute h-full w-[1px] bg-card-2"
							style="left: {((index + 1) * 100) / (ABSOLUTE_BAR_STEPS + 1)}%"
						></div>
					{/each}
					<div
						class="absolute left-0 h-full origin-left border-r border-white"
						style="width: {absoluteWidth}%"
					>
						<div class="h-full w-full {absoluteFill}"></div>
					</div>
				</div>
			{:else}
				<!-- Center origin: positive rolls grow right, negative left. -->
				<div class="relative h-[3px] bg-card-1">
					{#if attribute.fraction > 0}
						<div
							class="absolute left-1/2 h-full origin-left border-r border-white"
							style="width: {defaultWidth}%"
						>
							<div class="h-full w-full {variantFill[variant]}"></div>
						</div>
					{:else}
						<div
							class="absolute right-1/2 h-full origin-right border-l border-white"
							style="width: {defaultWidth}%"
						>
							<div class="h-full w-full {variantFill[variant]}"></div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>
