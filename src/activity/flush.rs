//! Writing the recorder's buffer to Postgres, and bounding what it has
//! written.

use sqlx::PgPool;

use super::{ActivityRecorder, Pending};

/// Hours of route counts kept. Thirteen months so a month can be
/// compared against the same month a year earlier.
pub const ACTIVITY_HOURS_KEEP: &str = "13 months";

/// User days kept. Twenty-five months so the twenty-four month cohort
/// chart always has a full domain.
pub const USER_ACTIVITY_KEEP: &str = "25 months";

/// Drains the recorder into the aggregate tables and prunes what has
/// aged out. Returns (route buckets, user days) written.
pub async fn flush(pool: &PgPool, activity: &ActivityRecorder) -> sqlx::Result<(usize, usize)> {
    let pending = activity.drain();
    let written = (pending.routes.len(), pending.users.len());

    if !pending.is_empty() {
        write(pool, pending).await?;
    }
    prune(pool).await?;
    Ok(written)
}

/// Both upserts in one transaction. The map keys are unique by
/// construction, so no statement can hit the same row twice.
async fn write(pool: &PgPool, pending: Pending) -> sqlx::Result<()> {
    let mut transaction = pool.begin().await?;

    if !pending.routes.is_empty() {
        let mut hours = Vec::with_capacity(pending.routes.len());
        let mut routes = Vec::with_capacity(pending.routes.len());
        let mut signed_in = Vec::with_capacity(pending.routes.len());
        let mut requests = Vec::with_capacity(pending.routes.len());
        let mut errors = Vec::with_capacity(pending.routes.len());
        let mut total_ms = Vec::with_capacity(pending.routes.len());
        for ((hour, route, session), counts) in pending.routes {
            hours.push(hour * 3600);
            routes.push(route);
            signed_in.push(session);
            requests.push(counts.requests as i64);
            errors.push(counts.errors as i64);
            total_ms.push(counts.total_ms as i64);
        }

        sqlx::query(
            "insert into activity_hours (hour, route, signed_in, requests, errors, total_ms)
             select to_timestamp(hour), route, signed_in, requests, errors, total_ms
             from unnest($1::bigint[], $2::text[], $3::bool[], $4::bigint[], $5::bigint[],
                         $6::bigint[])
                  as t(hour, route, signed_in, requests, errors, total_ms)
             on conflict (hour, route, signed_in) do update set
                 requests = activity_hours.requests + excluded.requests,
                 errors   = activity_hours.errors   + excluded.errors,
                 total_ms = activity_hours.total_ms + excluded.total_ms",
        )
        .bind(&hours)
        .bind(&routes)
        .bind(&signed_in)
        .bind(&requests)
        .bind(&errors)
        .bind(&total_ms)
        .execute(&mut *transaction)
        .await?;
    }

    if !pending.users.is_empty() {
        let mut days = Vec::with_capacity(pending.users.len());
        let mut user_ids = Vec::with_capacity(pending.users.len());
        let mut requests = Vec::with_capacity(pending.users.len());
        for ((day, user_id), count) in pending.users {
            days.push(day * 86_400);
            user_ids.push(user_id);
            requests.push(count as i64);
        }

        // A user deleted between the request and the flush would violate
        // the foreign key; their counts are not worth failing over.
        sqlx::query(
            "insert into user_activity_days (user_id, day, requests)
             select user_id, to_timestamp(day)::date, requests
             from unnest($1::bigint[], $2::bigint[], $3::bigint[]) as t(user_id, day, requests)
             where exists (select 1 from users where users.id = t.user_id)
             on conflict (user_id, day) do update set
                 requests = user_activity_days.requests + excluded.requests",
        )
        .bind(&user_ids)
        .bind(&days)
        .bind(&requests)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await
}

async fn prune(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(&format!(
        "delete from activity_hours where hour < now() - interval '{ACTIVITY_HOURS_KEEP}'"
    ))
    .execute(pool)
    .await?;

    sqlx::query(&format!(
        "delete from user_activity_days
         where day < (now() - interval '{USER_ACTIVITY_KEEP}')::date"
    ))
    .execute(pool)
    .await?;

    Ok(())
}
