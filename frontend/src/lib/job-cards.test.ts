import { describe, expect, it } from 'vitest';

import { JOB_CARDS, JOB_CARD_ORDER, progressFraction } from './job-cards';

describe('job card configs', () => {
	it('cover every job exactly once, in the bento order', () => {
		expect([...JOB_CARD_ORDER].sort()).toEqual(Object.keys(JOB_CARDS).sort());
	});
});

describe('progressFraction', () => {
	it('parses the fan-out progress lines', () => {
		expect(progressFraction('region 2/70 (id 10000002): 153 contracts so far')).toBeCloseTo(
			2 / 70
		);
		expect(progressFraction('character 3/3 (id 9): 1 modules imported so far')).toBe(1);
		expect(progressFraction('running…')).toBeNull();
		expect(progressFraction(null)).toBeNull();
		expect(progressFraction('weird 4/0 line')).toBeNull();
	});
});
