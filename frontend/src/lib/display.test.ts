import { describe, expect, it } from 'vitest';

import { defaultDisplaySettings, settingsFromCookies } from './display';

function jar(values: Record<string, string>): (name: string) => string | undefined {
	return (name) => values[name];
}

describe('settingsFromCookies', () => {
	it('falls back to the legacy defaults', () => {
		expect(settingsFromCookies(jar({}))).toEqual({
			display: 'grid',
			attribute_bar_mode: 'default',
			show_attribute_scores: false,
		});
		expect(defaultDisplaySettings().display).toBe('grid');
	});

	it('reads valid values and rejects unknown ones per field', () => {
		const settings = settingsFromCookies(
			jar({ display: 'table', attribute_bar_mode: 'sideways', show_attribute_scores: '1' }),
		);
		expect(settings.display).toBe('table');
		expect(settings.attribute_bar_mode).toBe('default');
		expect(settings.show_attribute_scores).toBe(true);
	});

	it('parses the boolean cookie like the server', () => {
		expect(settingsFromCookies(jar({ show_attribute_scores: 'true' })).show_attribute_scores).toBe(
			true,
		);
		expect(settingsFromCookies(jar({ show_attribute_scores: '0' })).show_attribute_scores).toBe(
			false,
		);
	});
});
