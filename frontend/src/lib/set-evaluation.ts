// The workbench set evaluation, a faithful port of the legacy
// DamageCalculator.getSetEvaluation: the combined DPS increase of
// stacking the benched damage modules of one type, with EVE's stacking
// penalty applied per slot.
import type { ModuleDetail } from './types';

/** EVE's stacking penalty: S(u) = e^-(u/2.67)^2. */
export function stackingModifier(index: number): number {
	return Math.exp(-((index / 2.67) ** 2));
}

const TURRET_TYPES = new Set([
	'Abyssal Gyrostabilizer',
	'Abyssal Heat Sink',
	'Abyssal Entropic Radiation Sink',
	'Abyssal Magnetic Field Stabilizer',
	'Abyssal Vorton Tuning System'
]);

const MISSILE_TYPES = new Set(['Abyssal Ballistic Control System']);

/**
 * The combined DPS increase in percent for the set, or null when the
 * leading type is not a damage module. Only modules matching the first
 * module's type participate, like legacy.
 */
export function setEvaluation(set: ModuleDetail[]): number | null {
	if (set.length === 0) return null;
	const resultType = set[0].type.name;
	const applicable = set.filter((module) => module.type.name === resultType);

	if (TURRET_TYPES.has(resultType)) {
		return calculateDps(applicable, 'Damage Modifier');
	}
	if (MISSILE_TYPES.has(resultType)) {
		return calculateDps(applicable, 'Missile Damage Bonus');
	}
	return null;
}

function attributeValue(module: ModuleDetail, displayName: string): number {
	// Case-insensitive: the legacy helper matched "Rate Of Fire Bonus"
	// against display names that read "Rate of Fire Bonus" here.
	const wanted = displayName.toLowerCase();
	return (
		module.mutated_attributes.find(
			(attribute) => attribute.display_name.toLowerCase() === wanted
		)?.value ?? 0
	);
}

function calculateDps(modules: ModuleDetail[], damageAttribute: string): number {
	const rateOfFire = modules
		.map((module) => {
			const value = attributeValue(module, 'Rate Of Fire Bonus');
			return value === 0 ? 0 : 1 - value;
		})
		.sort((a, b) => b - a);
	const damage = modules
		.map((module) => attributeValue(module, damageAttribute) - 1)
		.sort((a, b) => b - a);

	const dps = rateOfFire.map(
		(rate, index) =>
			(1 + damage[index] * stackingModifier(index)) / (1 - rate * stackingModifier(index))
	);
	return dps.reduce((a, b) => a * b, 1) * 100 - 100;
}
