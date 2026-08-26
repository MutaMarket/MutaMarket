import { describe, expect, it } from 'vitest';
import { IMPORT_REFRESH_MIN_MS, importRefreshGate } from './asset-import-stream';
import type { AssetImportView } from './types';

function view(status: string): AssetImportView {
	return {
		id: 1,
		character_id: 2,
		status,
		step: 'searching_abyssal_modules',
		assets_count: 100,
		assets_corporation_count: 0,
		abyssal_modules_count: 10,
		abyssal_modules_imported_count: 3,
		abyssal_modules_failed_count: 0,
		updated_seconds_ago: 0
	};
}

describe('importRefreshGate', () => {
	it('streams on the first active update, then throttles', () => {
		let clock = 0;
		const gate = importRefreshGate(() => clock);
		expect(gate(view('in_progress'))).toBe('stream');
		clock += IMPORT_REFRESH_MIN_MS - 1;
		expect(gate(view('in_progress'))).toBeNull();
		clock += 1;
		expect(gate(view('in_progress'))).toBe('stream');
	});

	it('always reports completion, regardless of the throttle', () => {
		const gate = importRefreshGate(() => 0);
		expect(gate(view('in_progress'))).toBe('stream');
		expect(gate(view('completed'))).toBe('completed');
	});

	it('ignores failed imports and cleared state', () => {
		const gate = importRefreshGate(() => 0);
		expect(gate(view('failed'))).toBeNull();
		expect(gate(null)).toBeNull();
	});
});
