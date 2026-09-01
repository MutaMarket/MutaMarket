<script lang="ts">
	// Request activity: the live minute, traffic over the window split by
	// whether the request carried a session, who is making the most of
	// it, and how many of them come back month over month.
	//
	// Every number here excludes the console's own polls, which would
	// otherwise dwarf real traffic and put whoever has this page open at
	// the top of the leaderboard.
	import TelemetryChart from '$lib/components/telemetry-chart.svelte';
	import { apply, live, subscribe } from '$lib/admin-live.svelte';
	import {
		ACTIVITY_WINDOWS,
		COHORT_SERIES,
		TRAFFIC_SERIES,
		USERS_SERIES,
		bucketLabel,
		cohortBuckets,
		requestsPerUser,
		signedInShare,
		tickPredicate,
		trafficBuckets,
		userBuckets,
		type ActivityWindow,
	} from '$lib/admin-activity';
	import { compact } from '$lib/admin-vitals';
	import type { ActivityHistory } from '$lib/admin-types';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	$effect(() => {
		apply(data.live);
	});
	$effect(() => subscribe(['activity']));

	const snapshot = $derived(live.activity ?? data.live.activity ?? null);

	// --- The window toggle -------------------------------------------------

	let activityWindow = $state<ActivityWindow>('24h');
	let history = $state<ActivityHistory | null>(null);

	$effect(() => {
		const wanted = activityWindow;
		void (async () => {
			const response = await fetch(`/api/admin/activity?window=${wanted}`);
			if (response.ok) {
				history = await response.json();
			}
		})();
	});

	const report = $derived(history ?? data.history);
	const now = $derived(live.now);

	/** Buckets the window holds, so a quiet one still takes its place. */
	const bucketCount = $derived(
		report.step_seconds >= 86_400 ? 30 : activityWindow === '24h' ? 24 : 24 * 7,
	);
	const traffic = $derived(trafficBuckets(report.traffic, report.step_seconds, now, bucketCount));
	const users = $derived(userBuckets(report.daily_users, bucketCount === 24 ? 7 : 30, now));
	const cohorts = $derived(cohortBuckets(report.months));

	const share = $derived(signedInShare(report.totals));
	const perUser = $derived(requestsPerUser(report.totals));

	const tiles = $derived([
		['Requests', compact(report.totals.requests)],
		['Signed in', share === null ? '—' : `${share.toFixed(0)}%`],
		['Page views', compact(report.totals.page_views)],
		['Active users', compact(report.totals.active_users)],
		['New this month', compact(report.totals.new_users)],
		['Requests / user', perUser === null ? '—' : perUser.toFixed(0)],
	] as const);

	const liveSeries = $derived(TRAFFIC_SERIES);
	const liveBuckets = $derived(
		(snapshot?.buckets ?? []).map((bucket) => ({
			minuteStart: bucket.minute_start,
			values: { signed_in: bucket.signed_in, anonymous: bucket.anonymous },
		})),
	);
</script>

<svelte:head><title>Activity - Admin - MutaMarket</title></svelte:head>

<!-- The window's headline numbers. -->
<section class="mb-8">
	<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
		{#each tiles as [label, value] (label)}
			<div class="hud-panel px-3 py-2.5">
				<div class="text-sm font-semibold text-foreground tabular-nums">{value}</div>
				<div class="truncate text-xs text-muted-foreground">{label}</div>
			</div>
		{/each}
	</div>
</section>

<!-- Live: the last hour, from the in-memory recorder. -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Live // Last hour</h2>
	<TelemetryChart
		title="Requests / minute"
		headline={compact(snapshot?.hour.requests ?? 0)}
		headlineClass="text-primary"
		sub={`${snapshot?.hour.users ?? 0} signed-in ${(snapshot?.hour.users ?? 0) === 1 ? 'user' : 'users'} · the console's own polls are not counted`}
		series={liveSeries}
		minutes={liveBuckets}
		emptyText="No requests in the last hour. The console's own polls are excluded, so an idle site reads as empty here."
	/>
</section>

<!-- The windowed report. -->
<section class="mb-8">
	<div class="mb-3 flex items-center gap-4">
		<h2 class="hud-label">Traffic // {report.window}</h2>
		<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
			{#each ACTIVITY_WINDOWS as option (option)}
				<button
					type="button"
					class="flex h-6 items-center rounded-[5px] px-2.5 text-xs transition-colors {activityWindow ===
					option
						? 'bg-primary text-primary-foreground'
						: 'text-muted-foreground hover:text-foreground'}"
					onclick={() => (activityWindow = option)}
				>
					{option}
				</button>
			{/each}
		</div>
	</div>
	<div class="grid gap-3 xl:grid-cols-2">
		<TelemetryChart
			title="Requests"
			headline={compact(report.totals.requests)}
			sub="signed in vs anonymous"
			series={TRAFFIC_SERIES}
			minutes={traffic}
			emptyText="No traffic recorded in this window."
			formatLabel={bucketLabel(report.step_seconds)}
			tickAt={tickPredicate(traffic.length)}
		/>
		<TelemetryChart
			title="Active users / day"
			headline={compact(report.totals.active_users)}
			sub="distinct signed-in users"
			series={USERS_SERIES}
			minutes={users}
			emptyText="No signed-in activity recorded yet."
			formatLabel={bucketLabel(86_400)}
			tickAt={tickPredicate(users.length)}
		/>
	</div>
</section>

<!-- New versus returning, always over the same 24 months. -->
<section class="mb-8">
	<h2 class="hud-label mb-3">Cohorts // New vs returning</h2>
	<TelemetryChart
		title="Active users / month"
		headline={compact(report.months.at(-1)?.active_users ?? 0)}
		sub="this month · returning is everyone active who registered earlier"
		series={COHORT_SERIES}
		minutes={cohorts}
		emptyText="No monthly activity recorded yet."
		formatLabel={bucketLabel(2_592_000)}
		tickAt={tickPredicate(cohorts.length, 12)}
	/>
</section>

<div class="grid gap-8 xl:grid-cols-2">
	<!-- Who is making the requests. -->
	<section>
		<h2 class="hud-label mb-3">Users // Most requests</h2>
		<div class="hud-frame divide-y divide-border">
			{#each report.top_users as user, index (user.user_id)}
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5">
					<span class="w-5 shrink-0 text-xs text-muted-foreground tabular-nums">
						{index + 1}
					</span>
					<span class="min-w-0 truncate text-sm font-medium">{user.name}</span>
					<span class="text-xs text-muted-foreground">
						{user.active_days}
						{user.active_days === 1 ? 'day' : 'days'} · since {user.created_at}
					</span>
					<span class="ml-auto text-sm tabular-nums">
						{user.requests.toLocaleString('en-US')}
					</span>
				</div>
			{:else}
				<p class="px-4 py-3 text-sm text-muted-foreground">No signed-in activity in this window.</p>
			{/each}
		</div>
	</section>

	<!-- What they asked for. -->
	<section>
		<h2 class="hud-label mb-3">Routes // Busiest</h2>
		<div class="hud-frame divide-y divide-border">
			{#each report.routes as route (route.route)}
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5">
					<span class="min-w-0 truncate font-mono text-xs">{route.route}</span>
					{#if route.errors > 0}
						<span class="shrink-0 text-xs text-negative tabular-nums">
							{route.errors.toLocaleString('en-US')} err
						</span>
					{/if}
					<span class="ml-auto shrink-0 text-xs text-muted-foreground tabular-nums">
						{route.average_ms.toFixed(0)} ms
					</span>
					<span class="w-20 shrink-0 text-right text-sm tabular-nums">
						{route.requests.toLocaleString('en-US')}
					</span>
				</div>
			{:else}
				<p class="px-4 py-3 text-sm text-muted-foreground">No traffic in this window.</p>
			{/each}
		</div>
	</section>
</div>
