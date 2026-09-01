import { describe, expect, it } from 'vitest';
import { teaserModules } from './teaser-modules';
import type { ModuleDetail } from './types';

const base = {
	id: 42,
	type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
	creator: null,
	mutated_attributes: [
		{
			id: 20,
			name: 'speedFactor',
			display_name: 'Maximum Velocity Bonus',
			value: 5,
			base_value: 5,
			fraction: 0.1,
			fraction_type: 0.1,
			fraction_absolute: 0.1,
			bar: 1,
			is_derived: false,
			unit: null,
			is_virtual: false,
			type_band: null,
		},
	],
	source_type: null,
	mutaplasmid: null,
	contract: {
		id: 1,
		type: 'item_exchange',
		price: 1,
		asking_for_items: false,
		plex_count: 0,
		non_abyssal_modules_count: 0,
		abyssal_modules_count: 1,
		issuer: null,
		date_issued: null,
		date_expired: null,
	},
	estimated_value: 100,
	estimated_value_updated_at: null,
	public_asset: null,
	slug: '50mn-abyssal-microwarpdrive-42',
	average_fraction: 0.1,
} satisfies ModuleDetail;

describe('teaserModules', () => {
	it('is deterministic and strips ownership', () => {
		const first = teaserModules(base);
		const second = teaserModules(base);
		expect(first).toEqual(second);
		expect(first).toHaveLength(6);
		for (const teaser of first) {
			expect(teaser.id).toBeLessThan(0);
			expect(teaser.contract).toBeNull();
			expect(teaser.training_module).toBeUndefined();
		}
	});

	it('wiggles values within the legacy band', () => {
		for (const teaser of teaserModules(base)) {
			const value = teaser.mutated_attributes[0].value;
			expect(value).toBeGreaterThanOrEqual(5 * 0.85);
			expect(value).toBeLessThanOrEqual(5 * 1.15);
		}
	});
});
