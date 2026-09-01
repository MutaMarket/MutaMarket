// Fake "similar" modules for the blurred premium teaser, ported from the
// legacy useTeaserModules composable: deterministic wiggles so the SSR
// render and the hydration produce identical cards.
import type { ModuleDetail } from './types';

/** Deterministic pseudo-random in [0, 1). */
function pseudoRandom(seed: number): number {
	const value = Math.sin(seed + 1) * 10_000;
	return value - Math.floor(value);
}

export function teaserModules(module: ModuleDetail, count = 6): ModuleDetail[] {
	return Array.from({ length: count }, (_, index) => {
		const clone = JSON.parse(JSON.stringify(module)) as ModuleDetail;

		const wiggled = clone.mutated_attributes.map((attribute, attributeIndex) => {
			const factor = 0.85 + pseudoRandom(index * 31 + attributeIndex * 7) * 0.3;
			return {
				...attribute,
				value: attribute.value * factor,
				fraction: attribute.fraction * factor,
			};
		});

		return {
			...clone,
			id: -(index + 1),
			mutated_attributes: wiggled,
			estimated_value:
				clone.estimated_value !== null
					? clone.estimated_value * (0.7 + pseudoRandom(index) * 0.6)
					: clone.estimated_value,
			contract: null,
			public_asset: null,
			training_module: undefined,
		};
	});
}
