//! The raffle domain: prize items admins load with redemption codes,
//! drawn hourly for active users and claimed or declined from the
//! site-wide prize dialog. Ports the legacy `RaffleStatus` enum,
//! `RaffleItem` model and `DrawRaffleWinnerCommand`.

/// A prize whose code was handed out; a past winner like a claim (the
/// legacy `RaffleStatus::PaidOut`).
pub const STATUS_PAID_OUT: i32 = 0;

/// In the pool, waiting to be drawn (the legacy `RaffleStatus::Pending`;
/// the table default).
pub const STATUS_PENDING: i32 = 1;

/// Drawn for a winner who has not claimed or declined yet (the legacy
/// `RaffleStatus::Active`).
pub const STATUS_ACTIVE: i32 = 2;

/// Claimed by its winner; the code shows on their settings page (the
/// legacy `RaffleStatus::Claimed`).
pub const STATUS_CLAIMED: i32 = 3;

/// How recently a user must have been active to win, the legacy
/// `config('raffles.required_active_hours')` (7 days).
pub const REQUIRED_ACTIVE_HOURS: i64 = 7 * 24;

/// Prizes drawn per run, the legacy `config('raffles.items_to_draw')`.
pub const ITEMS_TO_DRAW: i64 = 5;

/// The type-icon URL stored on items created with a type attached, the
/// legacy sprintf in `Admin\RaffleController::store`.
pub fn icon_url(type_id: i64) -> String {
    format!("https://images.evetech.net/types/{type_id}/icon")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DrawStats {
    /// Unclaimed prizes returned to the pool.
    pub reset: u64,
    /// Prizes drawn for a winner this run.
    pub drawn: u64,
    /// Prizes that found no eligible winner.
    pub unclaimed: u64,
}

/// The legacy hourly `app:draw-raffle-winner`.
///
/// A prize claimed or paid out today short-circuits the run: the day has
/// its winner, so every still-active prize returns to the pool and
/// nothing is drawn. Otherwise up to [`ITEMS_TO_DRAW`] prizes are picked
/// at random, already-active ones first (`orderBy('status', 'desc')`),
/// and each draws a random non-admin user who was active within
/// [`REQUIRED_ACTIVE_HOURS`] and holds no active prize. Winners expire
/// at the top of the next hour.
pub async fn draw_winners(pool: &sqlx::PgPool) -> sqlx::Result<DrawStats> {
    let mut stats = DrawStats::default();

    let winner_today: bool = sqlx::query_scalar(
        "select exists (
             select 1 from raffle_items
             where status = any($1)
               and expires_at >= date_trunc('day', now())
               and expires_at < date_trunc('day', now()) + interval '1 day'
         )",
    )
    .bind(vec![STATUS_CLAIMED, STATUS_PAID_OUT])
    .fetch_one(pool)
    .await?;

    if winner_today {
        stats.reset = reset_active(pool).await?;
        return Ok(stats);
    }

    let candidates: Vec<i64> = sqlx::query_scalar(
        "select id from raffle_items
         where status = any($1)
         order by status desc, random()
         limit $2",
    )
    .bind(vec![STATUS_ACTIVE, STATUS_PENDING])
    .bind(ITEMS_TO_DRAW)
    .fetch_all(pool)
    .await?;

    for item_id in candidates {
        // Re-drawn per item, like the legacy per-iteration query: a user
        // who just won is excluded from the next prize by their now
        // active item.
        let winner: Option<i64> = sqlx::query_scalar(
            "select u.id from users u
             where not u.is_admin
               and u.last_active_at between now() - make_interval(hours => $1::int) and now()
               and not exists (
                   select 1 from raffle_items r
                   where r.winner_id = u.id and r.status = $2
               )
             order by random()
             limit 1",
        )
        .bind(REQUIRED_ACTIVE_HOURS)
        .bind(STATUS_ACTIVE)
        .fetch_optional(pool)
        .await?;

        let Some(winner_id) = winner else {
            stats.unclaimed += 1;
            continue;
        };

        sqlx::query(
            "update raffle_items
             set status = $1, winner_id = $2,
                 expires_at = date_trunc('hour', now()) + interval '1 hour',
                 updated_at = now()
             where id = $3",
        )
        .bind(STATUS_ACTIVE)
        .bind(winner_id)
        .bind(item_id)
        .execute(pool)
        .await?;
        stats.drawn += 1;
    }

    Ok(stats)
}

/// Returns every drawn-but-unclaimed prize to the pool.
async fn reset_active(pool: &sqlx::PgPool) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "update raffle_items
         set status = $1, winner_id = null, expires_at = null, updated_at = now()
         where status = $2",
    )
    .bind(STATUS_PENDING)
    .bind(STATUS_ACTIVE)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
