import { describe, expect, it } from 'vitest';

import {
	BODY_CAPTURE_BYTES,
	callerLabel,
	failureAt,
	failureClass,
	failureLabel,
	filterFailures,
	formatBody,
	jobName,
	truncationNote,
} from './admin-failures';
import type { EsiFailureSummary } from './admin-types';

function failure(overrides: Partial<EsiFailureSummary> = {}): EsiFailureSummary {
	return {
		id: 1,
		occurred_at: '2026-08-30 14:04:10.757061+00',
		endpoint: 'contracts/public',
		method: 'GET',
		url: 'https://esi.evetech.net/latest/contracts/public/10000002/',
		status: 500,
		error_kind: null,
		error_message: 'Internal error',
		duration_ms: 120,
		authenticated: false,
		caller: 'job:region-contracts',
		...overrides,
	};
}

describe('failureClass', () => {
	it('uses the error chart series keys so a row matches its column', () => {
		expect(failureClass(failure({ status: 404 }))).toBe('client_errors');
		expect(failureClass(failure({ status: 420 }))).toBe('client_errors');
		expect(failureClass(failure({ status: 500 }))).toBe('server_errors');
		expect(failureClass(failure({ status: 503 }))).toBe('server_errors');
		expect(failureClass(failure({ status: null }))).toBe('transport_errors');
	});
});

describe('failureLabel', () => {
	it('shows the status, or why nothing came back', () => {
		expect(failureLabel(failure({ status: 500 }))).toBe('500');
		expect(failureLabel(failure({ status: null, error_kind: 'timeout' }))).toBe(
			'no response · timeout',
		);
		expect(failureLabel(failure({ status: null, error_kind: null }))).toBe('no response');
	});
});

describe('callerLabel and jobName', () => {
	it('reads a job failure back to its job', () => {
		expect(callerLabel(failure())).toBe('job region-contracts');
		expect(jobName(failure())).toBe('region-contracts');
	});

	it('reads a handler failure back to its route', () => {
		const handled = failure({ caller: 'http:POST /modules' });
		expect(callerLabel(handled)).toBe('POST /modules');
		expect(jobName(handled)).toBeNull();
	});

	it('has nothing to say for a call outside both', () => {
		expect(callerLabel(failure({ caller: null }))).toBeNull();
		expect(jobName(failure({ caller: null }))).toBeNull();
	});
});

describe('formatBody', () => {
	it('pretty-prints ESI json and leaves anything else alone', () => {
		expect(formatBody('{"error":"nope"}')).toBe('{\n  "error": "nope"\n}');
		expect(formatBody('<html>502 Bad Gateway</html>')).toBe('<html>502 Bad Gateway</html>');
	});

	it('has nothing to show for an absent or empty body', () => {
		expect(formatBody(null)).toBeNull();
		expect(formatBody('   ')).toBeNull();
	});
});

describe('truncationNote', () => {
	it('says what is not being shown when the body was capped', () => {
		const stored = 'x'.repeat(BODY_CAPTURE_BYTES);
		expect(truncationNote(stored, BODY_CAPTURE_BYTES * 512)).toBe(
			'showing the first 8.0 KB of 4.0 MB',
		);
	});

	it('is silent for a whole body', () => {
		expect(truncationNote('{}', 2)).toBeNull();
		expect(truncationNote(null, 100)).toBeNull();
		expect(truncationNote('{}', null)).toBeNull();
	});
});

describe('failureAt', () => {
	it('reads the API timestamp, offset suffix and all', () => {
		expect(failureAt(failure())).toBeCloseTo(Date.parse('2026-08-30T14:04:10.757Z') / 1000, 0);
	});
});

describe('filterFailures', () => {
	const minute = Math.floor(Date.parse('2026-08-30T14:04:00Z') / 1000);
	const rows = [
		failure({ id: 1, status: 500, endpoint: 'contracts/public' }),
		failure({ id: 2, status: 404, endpoint: 'characters/assets' }),
		failure({ id: 3, status: null, error_kind: 'timeout' }),
		failure({ id: 4, occurred_at: '2026-08-30 15:30:00+00' }),
	];

	it('narrows to a minute, an endpoint and a class', () => {
		expect(filterFailures(rows, { minute }).map((f) => f.id)).toEqual([1, 2, 3]);
		expect(filterFailures(rows, { endpoint: 'characters/assets' }).map((f) => f.id)).toEqual([2]);
		expect(filterFailures(rows, { class: 'transport_errors' }).map((f) => f.id)).toEqual([3]);
	});

	it('combines the filters', () => {
		expect(filterFailures(rows, { minute, class: 'client_errors' }).map((f) => f.id)).toEqual([2]);
	});

	it('returns everything for an empty filter', () => {
		expect(filterFailures(rows, {})).toHaveLength(4);
	});
});
