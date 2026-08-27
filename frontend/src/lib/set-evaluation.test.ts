import { describe, expect, it } from 'vitest';
import { setEvaluation, stackingModifier } from './set-evaluation';
import type { ModuleDetail } from './types';

function damageModule(
	typeName: string,
	rateOfFire: number,
	damage: number,
	damageAttribute = 'Damage Modifier'
): ModuleDetail {
	const attribute = (display_name: string, value: number) => ({
		id: 0,
		name: display_name,
		display_name,
		value,
		base_value: value,
		fraction: 0,
		fraction_type: 0,
		fraction_absolute: 0,
		bar: 0,
		is_derived: false,
		unit: null,
		is_virtual: false,
		type_band: null
	});
	return {
		id: 1,
		type: { id: 1, name: typeName },
		creator: null,
		mutated_attributes: [
			attribute('Rate Of Fire Bonus', rateOfFire),
			attribute(damageAttribute, damage)
		],
		source_type: null,
		mutaplasmid: null,
		contract: null,
		estimated_value: null,
		estimated_value_updated_at: null,
		public_asset: null,
		slug: 'x-1',
		average_fraction: null
	};
}

describe('setEvaluation', () => {
	it('matches the legacy single-module gyrostabilizer math', () => {
		// RoF bonus 0.895 (10.5% faster), damage modifier 1.1.
		const evaluation = setEvaluation([damageModule('Abyssal Gyrostabilizer', 0.895, 1.1)]);
		// (1 + 0.1) / (1 - 0.105) at full strength for the first slot.
		expect(evaluation).toBeCloseTo((1.1 / 0.895) * 100 - 100, 9);
	});

	it('applies the stacking penalty to the second module', () => {
		const first = damageModule('Abyssal Heat Sink', 0.9, 1.1);
		const second = damageModule('Abyssal Heat Sink', 0.9, 1.1);
		const evaluation = setEvaluation([first, second])!;
		const s1 = stackingModifier(1);
		const expected =
			((1 + 0.1) / (1 - 0.1)) * ((1 + 0.1 * s1) / (1 - 0.1 * s1)) * 100 - 100;
		expect(evaluation).toBeCloseTo(expected, 9);
	});

	it('only evaluates damage types and filters mixed sets', () => {
		expect(setEvaluation([damageModule('Abyssal Stasis Webifier', 0.9, 1.1)])).toBeNull();
		expect(setEvaluation([])).toBeNull();
		const mixed = setEvaluation([
			damageModule('Abyssal Ballistic Control System', 0.9, 1.1, 'Missile Damage Bonus'),
			damageModule('Abyssal Heat Sink', 0.5, 2.0)
		])!;
		const pure = setEvaluation([
			damageModule('Abyssal Ballistic Control System', 0.9, 1.1, 'Missile Damage Bonus')
		])!;
		expect(mixed).toBeCloseTo(pure, 9);
	});
});
