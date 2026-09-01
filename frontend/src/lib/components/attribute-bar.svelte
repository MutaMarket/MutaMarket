<script lang="ts">
	// The roll bar of one attribute in the selected mode, the legacy
	// Bars/Bar.vue, BarTypeNormalized.vue and BarAbsolute.vue — shared by
	// the grid card rows and the list view cells.
	import { attributeVariant } from '$lib/attributes';
	import type { AttributeBarMode } from '$lib/display';
	import type { ModuleAttributeView } from '$lib/types';

	let { attribute, mode }: { attribute: ModuleAttributeView; mode: AttributeBarMode } = $props();

	/** Tick marks of the absolute bar, like the legacy component. */
	const ABSOLUTE_BAR_STEPS = 20;

	const variant = $derived(attributeVariant(attribute));

	const variantFill: Record<string, string> = {
		gold: 'attribute-gold',
		diamond: 'attribute-diamond',
		brown: 'attribute-brown',
		positive: 'attribute-positive',
		'positive-derived': 'attribute-positive-derived',
		'negative-derived': 'attribute-negative-derived',
		negative: 'attribute-negative',
	};

	const absoluteFill = $derived(
		attribute.bar === 1
			? 'attribute-gold'
			: attribute.bar === 2
				? 'attribute-diamond'
				: attribute.bar === -1
					? 'attribute-brown'
					: 'attribute-absolute',
	);

	const defaultWidth = $derived(Math.min(Math.abs(attribute.fraction) * 50, 50));
	const typeWidth = $derived(Math.min(Math.abs(attribute.fraction_type) * 50, 50));
	const absoluteWidth = $derived(Math.min(Math.max(attribute.fraction_absolute * 100, 0), 100));
</script>

{#if mode === 'type'}
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
{:else if mode === 'absolute'}
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
{:else if mode === 'default'}
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
