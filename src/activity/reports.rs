//! The console's activity reads: traffic over a window, the route
//! roll-up, the top users, and the monthly cohort series.
//!
//! Every month and day boundary is UTC, matching the recorder, which
//! derives its buckets from unix seconds.

use serde_json::{Value, json};
use sqlx::{PgPool, Row};

use super::PAGE_VIEW_ROUTE;

/// The windows the page's toggle may request: (label, days, bucket
/// seconds). Hourly buckets up to a week, daily beyond.
pub const ACTIVITY_WINDOWS: [(&str, i64, i64); 3] =
    [("24h", 1, 3600), ("7d", 7, 3600), ("30d", 30, 86_400)];

/// Routes listed in the roll-up.
const ROUTES_SHOWN: i64 = 25;

/// Users listed in the leaderboard.
const USERS_SHOWN: i64 = 20;

/// Months on the cohort chart, independent of the traffic window.
const MONTHS_SHOWN: i32 = 24;

/// The whole payload of `GET /api/admin/activity`.
pub async fn history(pool: &PgPool, label: &str, days: i64, step: i64) -> sqlx::Result<Value> {
    Ok(json!({
        "window": label,
        "step_seconds": step,
        "traffic": traffic(pool, days, step).await?,
        "routes": routes(pool, days).await?,
        "top_users": top_users(pool, days).await?,
        "daily_users": daily_users(pool, days).await?,
        "months": months(pool).await?,
        "totals": totals(pool, days).await?,
    }))
}

/// Requests per bucket, split by whether they carried a session.
async fn traffic(pool: &PgPool, days: i64, step: i64) -> sqlx::Result<Vec<Value>> {
    let rows = sqlx::query(
        "select (floor(extract(epoch from hour) / $2) * $2)::bigint as at,
                (sum(requests) filter (where signed_in))::bigint as signed_in,
                (sum(requests) filter (where not signed_in))::bigint as anonymous
         from activity_hours
         where hour >= now() - make_interval(days => $1::int)
         group by 1 order by 1",
    )
    .bind(days as i32)
    .bind(step)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "at": row.get::<i64, _>("at"),
                "signed_in": row.get::<Option<i64>, _>("signed_in").unwrap_or(0),
                "anonymous": row.get::<Option<i64>, _>("anonymous").unwrap_or(0),
            })
        })
        .collect())
}

/// The busiest routes, with their signed-in share and average latency.
async fn routes(pool: &PgPool, days: i64) -> sqlx::Result<Vec<Value>> {
    let rows = sqlx::query(
        "select route,
                sum(requests)::bigint as requests,
                (sum(requests) filter (where signed_in))::bigint as signed_in,
                sum(errors)::bigint as errors,
                sum(total_ms)::float8 / nullif(sum(requests), 0) as average_ms
         from activity_hours
         where hour >= now() - make_interval(days => $1::int)
         group by route order by sum(requests) desc limit $2",
    )
    .bind(days as i32)
    .bind(ROUTES_SHOWN)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "route": row.get::<String, _>("route"),
                "requests": row.get::<Option<i64>, _>("requests").unwrap_or(0),
                "signed_in": row.get::<Option<i64>, _>("signed_in").unwrap_or(0),
                "errors": row.get::<Option<i64>, _>("errors").unwrap_or(0),
                "average_ms": row.get::<Option<f64>, _>("average_ms").unwrap_or(0.0),
            })
        })
        .collect())
}

/// The leaderboard: who made the most requests over the window.
async fn top_users(pool: &PgPool, days: i64) -> sqlx::Result<Vec<Value>> {
    let rows = sqlx::query(
        "select d.user_id, u.name, sum(d.requests)::bigint as requests,
                count(*) as active_days,
                (u.created_at at time zone 'UTC')::date::text as created_at,
                max(d.day)::text as last_active_day
         from user_activity_days d join users u on u.id = d.user_id
         where d.day >= (now() - make_interval(days => $1::int))::date
         group by d.user_id, u.name, u.created_at
         order by sum(d.requests) desc limit $2",
    )
    .bind(days as i32)
    .bind(USERS_SHOWN)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "user_id": row.get::<i64, _>("user_id"),
                "name": row.get::<String, _>("name"),
                "requests": row.get::<Option<i64>, _>("requests").unwrap_or(0),
                "active_days": row.get::<i64, _>("active_days"),
                "created_at": row.get::<String, _>("created_at"),
                "last_active_day": row.get::<String, _>("last_active_day"),
            })
        })
        .collect())
}

/// Distinct signed-in users per day, and what they asked for.
async fn daily_users(pool: &PgPool, days: i64) -> sqlx::Result<Vec<Value>> {
    let rows = sqlx::query(
        "select day::text as day, count(*) as users, sum(requests)::bigint as requests
         from user_activity_days
         where day >= (now() - make_interval(days => $1::int))::date
         group by day order by day",
    )
    .bind(days as i32)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "day": row.get::<String, _>("day"),
                "users": row.get::<i64, _>("users"),
                "requests": row.get::<Option<i64>, _>("requests").unwrap_or(0),
            })
        })
        .collect())
}

/// New versus returning per month.
///
/// `active_users` is everyone with activity in the month; `new_users`
/// are the active ones who registered in it and `returning_users` the
/// rest. `signed_up` counts registrations from `users.created_at` alone,
/// so the gap between it and `new_users` is sign-up churn — people who
/// registered and never came back. Both ship for that reason.
///
/// Months with no activity are present as zeros, so the chart's domain
/// does not move as data arrives.
async fn months(pool: &PgPool) -> sqlx::Result<Vec<Value>> {
    let rows = sqlx::query(
        "with months as (
             select generate_series(
                 date_trunc('month', (now() at time zone 'UTC')) - make_interval(months => $1 - 1),
                 date_trunc('month', (now() at time zone 'UTC')),
                 interval '1 month')::date as month
         ),
         active as (
             select date_trunc('month', d.day)::date as month, d.user_id,
                    date_trunc('month', (u.created_at at time zone 'UTC'))::date as joined
             from user_activity_days d join users u on u.id = d.user_id
             group by 1, 2, 3
         )
         select to_char(m.month, 'YYYY-MM') as month,
                count(a.user_id) as active_users,
                count(a.user_id) filter (where a.joined = m.month) as new_users,
                count(a.user_id) filter (where a.joined < m.month) as returning_users,
                (select count(*) from users u2
                 where date_trunc('month', (u2.created_at at time zone 'UTC'))::date = m.month)
                    as signed_up
         from months m left join active a on a.month = m.month
         group by m.month order by m.month",
    )
    .bind(MONTHS_SHOWN)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "month": row.get::<String, _>("month"),
                "active_users": row.get::<i64, _>("active_users"),
                "new_users": row.get::<i64, _>("new_users"),
                "returning_users": row.get::<i64, _>("returning_users"),
                "signed_up": row.get::<i64, _>("signed_up"),
            })
        })
        .collect())
}

/// The stat row above the charts.
async fn totals(pool: &PgPool, days: i64) -> sqlx::Result<Value> {
    let row = sqlx::query(
        "select coalesce(sum(requests), 0)::bigint as requests,
                coalesce(sum(requests) filter (where signed_in), 0)::bigint as signed_in_requests,
                coalesce(sum(requests) filter (where route = $2), 0)::bigint as page_views
         from activity_hours
         where hour >= now() - make_interval(days => $1::int)",
    )
    .bind(days as i32)
    .bind(PAGE_VIEW_ROUTE)
    .fetch_one(pool)
    .await?;

    let users = sqlx::query(
        "select count(distinct user_id) as active_users
         from user_activity_days
         where day >= (now() - make_interval(days => $1::int))::date",
    )
    .bind(days as i32)
    .fetch_one(pool)
    .await?;

    let new_users: i64 = sqlx::query_scalar(
        "select count(*) from users
         where date_trunc('month', (created_at at time zone 'UTC'))
               = date_trunc('month', (now() at time zone 'UTC'))",
    )
    .fetch_one(pool)
    .await?;

    Ok(json!({
        "requests": row.get::<i64, _>("requests"),
        "signed_in_requests": row.get::<i64, _>("signed_in_requests"),
        "page_views": row.get::<i64, _>("page_views"),
        "active_users": users.get::<i64, _>("active_users"),
        "new_users": new_users,
    }))
}
