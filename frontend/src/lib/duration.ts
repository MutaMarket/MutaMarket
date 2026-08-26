// Small time helpers of the admin scheduler page.

/** A job cadence as prose: "every 5 min", "hourly", "daily". */
export function humanizeInterval(seconds: number): string {
	if (seconds === 7 * 24 * 3600) return 'weekly';
	if (seconds % (24 * 3600) === 0 && seconds > 24 * 3600) return `every ${seconds / 86_400} d`;
	if (seconds === 24 * 3600) return 'daily';
	if (seconds === 3600) return 'hourly';
	if (seconds % 3600 === 0) return `every ${seconds / 3600} h`;
	if (seconds >= 60) return `every ${Math.round(seconds / 60)} min`;
	return `every ${seconds} s`;
}

/** Seconds relative to now as prose: "3 min ago", "in 12 min", "just now". */
export function relativeTime(deltaSeconds: number): string {
	const magnitude = Math.abs(deltaSeconds);
	if (magnitude < 5) return 'just now';

	let amount: string;
	if (magnitude < 60) {
		amount = `${Math.round(magnitude)} s`;
	} else if (magnitude < 3600) {
		amount = `${Math.round(magnitude / 60)} min`;
	} else if (magnitude < 24 * 3600) {
		amount = `${Math.round(magnitude / 3600)} h`;
	} else {
		amount = `${Math.round(magnitude / (24 * 3600))} d`;
	}

	return deltaSeconds < 0 ? `${amount} ago` : `in ${amount}`;
}

/** Parses the API's `timestamptz::text` format into unix seconds. */
export function parseDbTimestamp(text: string): number {
	// "2026-08-25 14:30:00.123+00" - make it ISO for Date.parse: T
	// separator, and the bare Postgres offset needs its minutes.
	const iso = text.replace(' ', 'T').replace(/([+-]\d{2})$/, '$1:00');
	const parsed = Date.parse(iso);
	return Number.isNaN(parsed) ? 0 : parsed / 1000;
}
