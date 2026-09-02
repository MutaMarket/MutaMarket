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

/** The admin-authorized character the background features act through. */
export interface ServiceCharacter {
  character: { id: number; name: string | null; scopes: string[] } | null;
  source: 'authorized' | 'env' | null;
}

/** One captured ESI failure, as the list and the live section serve it. */
export interface EsiFailureSummary {
  id: number;
  occurred_at: string;
  /** The telemetry bucket key, e.g. 'contracts/public'. */
  endpoint: string;
  method: string;
  url: string;
  /** Null when no response arrived at all. */
  status: number | null;
  /** 'timeout' | 'connect' | 'decode' | 'body' | 'request', set only
   * when no response arrived. */
  error_kind: string | null;
  error_message: string | null;
  duration_ms: number;
  /** A token was sent. Which one is deliberately not recorded. */
  authenticated: boolean;
  caller: string | null;
}

/** The detail behind one summary; the bodies do not ride the poll. */
export interface EsiFailureDetail extends EsiFailureSummary {
  scheduler_run_id: number | null;
  response_headers: Record<string, string> | null;
  response_body: string | null;
  /** Length before truncation, so the page can say what it omits. */
  response_bytes: number | null;
  request_body: string | null;
  request_bytes: number | null;
}

export interface FailuresSection {
  captured: EsiFailureSummary[];
  /** The row cap and age bound the table is kept under. */
  keep: number;
  retention_days: number;
}

/** The live request-activity window, served from memory. */
export interface ActivitySnapshot {
  window_minutes: number;
  buckets: { minute_start: number; signed_in: number; anonymous: number }[];
  hour: { requests: number; signed_in: number; anonymous: number; users: number };
}

/** The windowed activity report (/api/admin/activity). */
export interface ActivityHistory {
  window: string;
  step_seconds: number;
  traffic: { at: number; signed_in: number; anonymous: number }[];
  routes: {
    route: string;
    requests: number;
    signed_in: number;
    errors: number;
    average_ms: number;
  }[];
  top_users: {
    user_id: number;
    name: string;
    requests: number;
    active_days: number;
    created_at: string;
    last_active_day: string;
  }[];
  daily_users: { day: string; users: number; requests: number }[];
  months: {
    month: string;
    active_users: number;
    new_users: number;
    returning_users: number;
    /** Registrations that month, from users.created_at alone. The gap
     * to new_users is sign-up churn. */
    signed_up: number;
  }[];
  totals: {
    requests: number;
    signed_in_requests: number;
    /** nav-state loads: the closest thing to a page view. */
    page_views: number;
    active_users: number;
    new_users: number;
  };
}
