import { describe, expect, it } from 'vitest';
import { abyssalBySlug, abyssalSlug } from './abyssals';

describe('abyssalSlug', () => {
	it('turns the type id into the legacy name slug', () => {
		expect(abyssalSlug(47702)).toBe('abyssal-stasis-webifier');
		expect(abyssalSlug(47408)).toBe('50mn-abyssal-microwarpdrive');
	});

	it('falls back to the bare id for unknown types', () => {
		expect(abyssalSlug(123456)).toBe('123456');
	});
});

describe('abyssalBySlug', () => {
	it('resolves name slugs and bare ids', () => {
		expect(abyssalBySlug('abyssal-stasis-webifier')?.id).toBe(47702);
		expect(abyssalBySlug('47702')?.name).toBe('Abyssal Stasis Webifier');
	});

	it('returns null for unknown or empty segments', () => {
		expect(abyssalBySlug('not-a-type')).toBeNull();
		expect(abyssalBySlug(null)).toBeNull();
		expect(abyssalBySlug('')).toBeNull();
	});
});
