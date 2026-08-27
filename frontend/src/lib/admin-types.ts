// Response shapes of the admin dashboard endpoints
// (src/server/admin.rs in the Rust crate).

export interface SchedulerRun {
	started_at: string;
	finished_at: string | null;
	outcome: string | null;
	summary: string | null;
	error: string | null;
	/** The job's headline metric for this run (what the cards chart). */
	items: number | null;
	duration_seconds: number | null;
	/** Named per-run sub-metrics for the multi-line cards. */
	metrics: Record<string, number> | null;
}

export interface SchedulerJob {
	name: string;
	interval_seconds: number;
	downtime_guarded: boolean;
	paused: boolean;
	running: boolean;
	/** Unix seconds of the next scheduled tick; null while loops are off. */
	next_run_at: number | null;
	/** The in-flight run's live progress line, if it reported one. */
	progress: string | null;
	last_runs: SchedulerRun[];
}

export interface DatabaseCounts {
	modules: number;
	modules_without_estimate: number;
	contracts: number;
	contract_items: number;
	characters: number;
	users: number;
	assets: number;
	public_ownerships: number;
	market_history_days: number;
}

export interface MetricSample {
	/** Unix seconds. */
	taken_at: number;
	value: number;
}

export interface SchedulerStatus {
	enabled: boolean;
	in_downtime: boolean;
	database: DatabaseCounts;
	/** Recorded series of the last day, keyed by metric name. */
	jobs: SchedulerJob[];
}

/** Process/container telemetry; the Linux-only readings are null on
 * native dev hosts. */
/** The windowed vitals history (/api/admin/metrics). */
export interface MetricsHistory {
	window: string;
	step_seconds: number;
	series: Record<string, MetricSample[]>;
}

export interface SystemStats {
	disk_used_bytes: number | null;
	disk_total_bytes: number | null;
	memory_total_bytes: number | null;
	memory_rss_bytes: number | null;
	memory_current_bytes: number | null;
	memory_limit_bytes: number | null;
	cpu_seconds: number | null;
	cpu_cores: number | null;
	network_rx_bytes: number | null;
	network_tx_bytes: number | null;
	uptime_seconds: number | null;
	database_size_bytes: number | null;
}

export interface TelemetryCounts {
	requests: number;
	success: number;
	client_errors: number;
	server_errors: number;
	transport_errors: number;
	total_ms: number;
}

export interface TelemetryBucket {
	/** Unix seconds of the minute's start. */
	minute_start: number;
	endpoints: Record<string, TelemetryCounts>;
}

export interface TelemetrySnapshot {
	window_minutes: number;
	buckets: TelemetryBucket[];
}

// Guests land on the login page (401), non-admins on the 403 error page.
