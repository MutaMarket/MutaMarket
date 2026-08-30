// The console's live state, shared by every admin page.
//
// One interval and one `/api/admin/live` request serve the whole
// section: pages declare the sections they draw, the store polls their
// union, and the header keeps ticking while you are on a page that
// draws no charts at all. The jobs section rides a server revision, so
// an unchanged one is neither queried nor reassigned — which is what
// keeps the job cards and their charts from rebuilding every poll.
import type {
	DatabaseCounts,
	SchedulerJob,
	SystemStats,
	TelemetrySnapshot
} from '$lib/admin-types';

/** Live-status poll cadence. */
const POLL_INTERVAL_MS = 5000;

export type LiveSection = 'header' | 'system' | 'telemetry' | 'database' | 'jobs';

export interface AdminHeader {
	enabled: boolean;
	in_downtime: boolean;
	uptime_seconds: number | null;
}

/** One `/api/admin/live` payload; unrequested sections are absent. */
export interface LivePayload {
	header?: AdminHeader;
	system?: SystemStats;
	telemetry?: TelemetrySnapshot;
	database?: DatabaseCounts;
	/** Null when the client's revision still matched. */
	jobs?: SchedulerJob[] | null;
	jobs_revision?: string;
}

/** One /system sample with the moment it was taken. */
interface Sample {
	at: number;
	stats: SystemStats;
}

// Raw state throughout: every section is replaced wholesale and never
// mutated in place, so deep proxying 25 jobs with 20 runs each on every
// poll would be pure cost — and raw state keeps the object identity a
// gated section depends on.
let header = $state.raw<AdminHeader | null>(null);
let system = $state.raw<SystemStats | null>(null);
let telemetry = $state.raw<TelemetrySnapshot | null>(null);
let database = $state.raw<DatabaseCounts | null>(null);
let jobs = $state.raw<SchedulerJob[]>([]);
let previousSample = $state.raw<Sample | null>(null);
let sampleAt = $state(Date.now() / 1000);
let now = $state(Math.floor(Date.now() / 1000));

let jobsRevision: string | null = null;
const subscribers = new Map<LiveSection, number>();
let timer: ReturnType<typeof setInterval> | null = null;

/**
 * The live console state. Read through a getter object so the runes
 * stay in this module and every page sees the same values.
 */
export const live = {
	get header() {
		return header;
	},
	get system() {
		return system;
	},
	get telemetry() {
		return telemetry;
	},
	get database() {
		return database;
	},
	get jobs() {
		return jobs;
	},
	/** The previous /system sample, for the cpu and network rates. */
	get previousSample() {
		return previousSample;
	},
	get currentSample(): Sample | null {
		return system === null ? null : { at: sampleAt, stats: system };
	},
	/** Unix seconds, advanced by the poll; the relative times read it. */
	get now() {
		return now;
	}
};

/** Folds one payload into the store, section by section. */
export function apply(payload: LivePayload): void {
	if (payload.header) header = payload.header;
	if (payload.telemetry) telemetry = payload.telemetry;
	if (payload.database) database = payload.database;
	if (payload.system) {
		previousSample = system === null ? null : { at: sampleAt, stats: system };
		system = payload.system;
		sampleAt = Date.now() / 1000;
	}
	if (payload.jobs_revision !== undefined) {
		jobsRevision = payload.jobs_revision;
	}
	// A null section means the revision matched: keep what we hold, and
	// keep its object identity so nothing downstream re-renders.
	if (payload.jobs) {
		jobs = payload.jobs;
	}
}

function activeSections(): LiveSection[] {
	return [...subscribers.entries()].filter(([, count]) => count > 0).map(([section]) => section);
}

export async function refresh(): Promise<void> {
	const sections = activeSections();
	if (sections.length === 0) return;

	now = Math.floor(Date.now() / 1000);
	const params = new URLSearchParams({ sections: sections.join(',') });
	if (sections.includes('jobs') && jobsRevision !== null) {
		params.set('jobs_revision', jobsRevision);
	}

	try {
		const response = await fetch(`/api/admin/live?${params}`);
		if (response.ok) {
			apply(await response.json());
		}
	} catch {
		// Keep the last state while the API is unreachable.
	}
}

/**
 * Registers the sections a page draws for as long as it is mounted.
 * Call from an `$effect` and return the result, so the interval stops
 * with the last subscriber.
 */
export function subscribe(sections: LiveSection[]): () => void {
	for (const section of sections) {
		subscribers.set(section, (subscribers.get(section) ?? 0) + 1);
	}
	if (timer === null) {
		timer = setInterval(refresh, POLL_INTERVAL_MS);
	}

	return () => {
		for (const section of sections) {
			const count = (subscribers.get(section) ?? 1) - 1;
			if (count > 0) {
				subscribers.set(section, count);
			} else {
				subscribers.delete(section);
			}
		}
		if (activeSections().length === 0 && timer !== null) {
			clearInterval(timer);
			timer = null;
		}
	};
}

/** Test seam: drops every subscriber and the held revision. */
export function reset(): void {
	subscribers.clear();
	if (timer !== null) {
		clearInterval(timer);
		timer = null;
	}
	jobsRevision = null;
	header = null;
	system = null;
	telemetry = null;
	database = null;
	jobs = [];
	previousSample = null;
}
