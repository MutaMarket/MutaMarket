<script lang="ts">
	// One attribute row of a module card, mirroring the legacy
	// Grid/Attribute.vue: 36px attribute icon spanning two rows, the name
	// over the value+difference cluster (the score column always occupies
	// column 3, so the cluster flows to the second row), then the roll bar
	// in the mode the display settings select (attribute-bar.svelte).
	import AttributeBar from './attribute-bar.svelte';
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
			<AttributeBar {attribute} mode={settings.attribute_bar_mode} />
		</div>
	{/if}
</div>
