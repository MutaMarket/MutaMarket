import { describe, expect, it } from 'vitest';
import { REVIEW_ACTIONS, shortcutAction, statusLabel } from './contract-review';
import { toHistoricContractLink } from './export';

describe('review actions', () => {
	it('offers the legacy three statuses in order', () => {
		expect(REVIEW_ACTIONS.map((action) => action.status)).toEqual([
			'completed',
			'failed',
			'unknown',
		]);
		expect(REVIEW_ACTIONS.map((action) => action.variant)).toEqual([
			'default',
			'destructive',
			'outline',
		]);
	});

	it('labels statuses like the legacy locale', () => {
		expect(statusLabel('completed')).toBe('Completed');
		expect(statusLabel('failed')).toBe('Failed');
		expect(statusLabel('unknown')).toBe('Unknown');
		expect(statusLabel('outstanding')).toBe('outstanding');
	});
});

describe('shortcutAction', () => {
	it('maps the legacy magic keys', () => {
		expect(shortcutAction({ shiftKey: true, key: 'C' })).toBe('completed');
		expect(shortcutAction({ shiftKey: true, key: 'X' })).toBe('failed');
		expect(shortcutAction({ shiftKey: true, key: 'L' })).toBe('copy-link');
	});

	it('ignores everything else', () => {
		expect(shortcutAction({ shiftKey: false, key: 'C' })).toBeNull();
		expect(shortcutAction({ shiftKey: true, key: 'Q' })).toBeNull();
		expect(shortcutAction({ shiftKey: false, key: 'Enter' })).toBeNull();
	});
});

describe('toHistoricContractLink', () => {
	// Intl's currency form separates "ISK" with a non-breaking space,
	// exactly like the legacy links did.
	it('builds the legacy in-game link for a bare contract', () => {
		expect(toHistoricContractLink({ id: 987654, price: 150000000 })).toBe(
			'<url=contract:30000142//987654>Contract 987654 ISK 150,000,000</url>',
		);
	});

	it('treats a missing price as zero', () => {
		expect(toHistoricContractLink({ id: 5, price: null })).toBe(
			'<url=contract:30000142//5>Contract 5 ISK 0</url>',
		);
	});
});
