//! The premium payment math, ported from the legacy
//! `App\Services\PremiumService`: a donation amount (plus the
//! character's carried-over payment rest) buys whole months, yearly
//! blocks first at a discount, and the paid-until date extends from the
//! current expiry — or restarts from now when it already lapsed.
//!
//! The month arithmetic mirrors PHP's `DateTime::modify('+n months')`
//! (Carbon `addMonths`), which *overflows* a too-long day into the next
//! month (Jan 31 + 1 month = Mar 3) instead of clamping like Postgres
//! `+ interval '1 month'` — hence the pure civil-date math here.

use sqlx::{PgConnection, PgPool};

use crate::notifications::format_isk;

/// One month of premium: the legacy `app.premium_cost` default
/// (100M ISK), env-overridable via `APP_PREMIUM_COST` like legacy.
pub const DEFAULT_MONTHLY_COST: f64 = 100_000_000.0;

/// Twelve months at a discount (two months free): the legacy
/// `app.premium_yearly_cost` default (1B ISK), env-overridable via
/// `APP_PREMIUM_YEARLY_COST`.
pub const DEFAULT_YEARLY_COST: f64 = 1_000_000_000.0;

/// The service character donations are sent to in-game, the legacy
/// `app.premium_character` config default.
pub const PREMIUM_CHARACTER_NAME: &str = "MutaMate";

/// The legacy `app.premium_character`: env-overridable via
/// `APP_PREMIUM_CHARACTER` like legacy, with the "MutaMate" default.
pub fn premium_character_name() -> String {
    std::env::var("APP_PREMIUM_CHARACTER").unwrap_or_else(|_| PREMIUM_CHARACTER_NAME.to_owned())
}

/// Months bought by one yearly block.
pub const MONTHS_PER_YEAR: i32 = 12;

/// The two price points, the legacy `app.premium_cost` /
/// `app.premium_yearly_cost` config pair.
#[derive(Debug, Clone, Copy)]
pub struct PremiumCosts {
    pub monthly: f64,
    pub yearly: f64,
}

impl PremiumCosts {
    /// The legacy env overrides (`APP_PREMIUM_COST`,
    /// `APP_PREMIUM_YEARLY_COST`) with the config defaults.
    pub fn from_env() -> Self {
        let read = |name: &str, default: f64| {
            std::env::var(name).ok().and_then(|value| value.parse().ok()).unwrap_or(default)
        };
        Self {
            monthly: read("APP_PREMIUM_COST", DEFAULT_MONTHLY_COST),
            yearly: read("APP_PREMIUM_YEARLY_COST", DEFAULT_YEARLY_COST),
        }
    }
}

/// The legacy `PremiumPaymentResult`, plus the refreshed paid-until the
/// confirmation mail prints (`Some` exactly when months were bought).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PremiumUpdate {
    pub months_paid: i32,
    pub rest_amount: f64,
    /// Zero once a month was bought, otherwise the top-up still missing
    /// (legacy quirk: the carried rest counts toward it, so a paid rest
    /// makes this smaller than `monthly - amount`).
    pub amount_needed_for_next_month: f64,
    /// The new `premium_paid_until` (unix seconds) when months were
    /// bought; `None` leaves the stored value untouched.
    pub paid_until: Option<i64>,
}

/// The whole payment application, pure: the legacy
/// `PremiumService::addPremiumToCharacter` body without the model write.
pub fn apply_payment(
    costs: PremiumCosts,
    paid_until: Option<i64>,
    payment_rest: f64,
    amount: f64,
    now: i64,
) -> PremiumUpdate {
    let total_available = amount + payment_rest;
    let months_paid = months_paid(costs, total_available);
    let rest_amount = rest_amount(costs, total_available, months_paid);
    let amount_needed_for_next_month =
        if months_paid > 0 { 0.0 } else { costs.monthly - total_available };

    let paid_until = (months_paid > 0).then(|| {
        // Legacy: extend a live subscription from its expiry, restart a
        // lapsed (or absent) one from now.
        let base = match paid_until {
            Some(until) if until >= now => until,
            _ => now,
        };
        add_months_php(base, months_paid)
    });

    PremiumUpdate { months_paid, rest_amount, amount_needed_for_next_month, paid_until }
}

/// How many months the amount covers, yearly blocks first — the legacy
/// `calculateMonthsPaid` loop.
fn months_paid(costs: PremiumCosts, mut total: f64) -> i32 {
    let mut months = 0;
    while total >= costs.monthly {
        if total >= costs.yearly {
            months += MONTHS_PER_YEAR;
            total -= costs.yearly;
        } else {
            let from_remainder = (total / costs.monthly).floor() as i32;
            months += from_remainder;
            total -= f64::from(from_remainder) * costs.monthly;
        }
    }
    months
}

/// What is left after yearly and monthly pricing — the legacy
/// `calculateRestAmount`, recomputed from the full total.
fn rest_amount(costs: PremiumCosts, mut total: f64, months_paid: i32) -> f64 {
    let mut yearly_blocks = 0;
    while total >= costs.yearly {
        yearly_blocks += 1;
        total -= costs.yearly;
    }
    let remaining_months = months_paid - yearly_blocks * MONTHS_PER_YEAR;
    total -= f64::from(remaining_months) * costs.monthly;
    total.max(0.0)
}

/// Applies a payment to the character row: the legacy service's model
/// write (`premium_paid_until` only moves when months were bought;
/// `premium_paid_total` counts the raw amount, not the rest). Takes a
/// connection so donation creation can run it inside its transaction.
pub async fn add_premium_to_character(
    conn: &mut PgConnection,
    character_id: i64,
    amount: f64,
    costs: PremiumCosts,
) -> sqlx::Result<PremiumUpdate> {
    let (paid_until, payment_rest): (Option<i64>, f64) = sqlx::query_as(
        "select extract(epoch from premium_paid_until)::bigint, premium_payment_rest
         from characters where id = $1",
    )
    .bind(character_id)
    .fetch_one(&mut *conn)
    .await?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|now| now.as_secs() as i64)
        .unwrap_or(0);
    let update = apply_payment(costs, paid_until, payment_rest, amount, now);

    sqlx::query(
        "update characters
         set premium_paid_until = coalesce(to_timestamp($2), premium_paid_until),
             premium_payment_rest = $3,
             premium_paid_total = premium_paid_total + $4,
             updated_at = now()
         where id = $1",
    )
    .bind(character_id)
    .bind(update.paid_until.map(|until| until as f64))
    .bind(update.rest_amount)
    .bind(amount)
    .execute(&mut *conn)
    .await?;

    Ok(update)
}

/// `Y-m-d` of a unix timestamp, the confirmation mail's date format.
pub fn format_ymd(unix: i64) -> String {
    let (year, month, day) = civil_from_days(unix.div_euclid(SECONDS_PER_DAY));
    format!("{year:04}-{month:02}-{day:02}")
}

/// Outbox kind of the expiry notice.
pub const PREMIUM_EXPIRED_KIND: &str = "premium-expired";

/// The legacy `PremiumExpired::getSubject`.
pub const PREMIUM_EXPIRED_SUBJECT: &str = "Your premium subscription has expired";

/// The legacy `RemoveExpiredPremiumCommand`: clear every lapsed
/// `premium_paid_until` and queue the expiry notice — but, faithfully,
/// only for characters that belong to a user; an ownerless character
/// keeps its expired timestamp (`whereHas('user')`). Legacy only
/// notified in production; here the outbox's delivery mode covers the
/// environment split, so the notice is always queued. Returns how many
/// characters expired.
pub async fn remove_expired_premium(pool: &PgPool, costs: PremiumCosts) -> sqlx::Result<i64> {
    let expired: Vec<(i64, String, i64)> = sqlx::query_as(
        "select c.id, c.name, c.user_id from characters c
         join users u on u.id = c.user_id
         where c.premium_paid_until is not null and c.premium_paid_until < now()
         order by c.id",
    )
    .fetch_all(pool)
    .await?;

    for (character_id, name, user_id) in &expired {
        // One transaction per character: once the timestamp is cleared
        // the sweep predicate no longer matches, so a notice queued
        // outside the transaction could be lost forever on a partial
        // failure (the pattern donations::create_donation set).
        let mut tx = pool.begin().await?;
        sqlx::query(
            "update characters set premium_paid_until = null, updated_at = now() where id = $1",
        )
        .bind(character_id)
        .execute(tx.as_mut())
        .await?;

        let (subject, body) = premium_expired_mail(name, costs);
        crate::notifications::queue_on(
            tx.as_mut(),
            *user_id,
            PREMIUM_EXPIRED_KIND,
            &subject,
            &body,
            serde_json::json!({ "character_id": character_id }),
        )
        .await?;
        tx.commit().await?;
    }

    Ok(expired.len() as i64)
}

/// The `notifications/premium_expired` blade template, in its in-game
/// variant (the mail is sent by the service character, hence
/// "this character"; the unported Discord channel said "MutaMate").
pub fn premium_expired_mail(character_name: &str, costs: PremiumCosts) -> (String, String) {
    let body = format!(
        "Hello {character_name},\n\n\
         We just wanted to let you know that your premium account has expired, but don't \
         worry! You can still use all the features of the site, but you will not show up as \
         a premium member anymore.\n\n\
         If you want to renew your premium account, you can do so by sending {} ISK per \
         month or {} ISK for a full year (save 2 months!) to this character.\n\n\
         Thank you for supporting us!\n\
         The MutaMarket team",
        format_isk(costs.monthly),
        format_isk(costs.yearly),
    );
    (PREMIUM_EXPIRED_SUBJECT.to_owned(), body)
}

const SECONDS_PER_DAY: i64 = 86_400;

/// PHP `DateTime` month addition on a unix timestamp: keep the day and
/// time, move the month, and overflow a day past the target month's end
/// into the following month (Jan 31 + 1 month = Mar 3).
pub fn add_months_php(unix: i64, months: i32) -> i64 {
    let days = unix.div_euclid(SECONDS_PER_DAY);
    let seconds_of_day = unix.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);

    let total_months = year * 12 + i64::from(month) - 1 + i64::from(months);
    let mut year = total_months.div_euclid(12);
    let mut month = (total_months.rem_euclid(12) + 1) as u32;
    let mut day = day;
    // The PHP overflow: Feb 31 becomes Mar 3, not Feb 28.
    while day > days_in_month(year, month) {
        day -= days_in_month(year, month);
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }

    days_from_civil(year, month, day) * SECONDS_PER_DAY + seconds_of_day
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
    }
}

/// Days since 1970-01-01 to (year, month, day); Howard Hinnant's civil
/// calendar algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// The inverse of [`civil_from_days`].
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const COSTS: PremiumCosts =
        PremiumCosts { monthly: DEFAULT_MONTHLY_COST, yearly: DEFAULT_YEARLY_COST };

    fn unix(year: i64, month: u32, day: u32, seconds_of_day: i64) -> i64 {
        days_from_civil(year, month, day) * SECONDS_PER_DAY + seconds_of_day
    }

    #[test]
    fn civil_round_trips() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2026, 8, 28)), (2026, 8, 28));
        assert_eq!(civil_from_days(days_from_civil(2000, 2, 29)), (2000, 2, 29));
    }

    #[test]
    fn php_month_addition_overflows_short_months() {
        // Jan 31 + 1 month: Feb 31 does not exist, PHP lands on Mar 3
        // (non-leap) / Mar 2 (leap) — never on Feb 28.
        assert_eq!(add_months_php(unix(2026, 1, 31, 3_600), 1), unix(2026, 3, 3, 3_600));
        assert_eq!(add_months_php(unix(2024, 1, 31, 0), 1), unix(2024, 3, 2, 0));
        // Ordinary additions keep the day, across year ends too.
        assert_eq!(add_months_php(unix(2026, 8, 28, 43_210), 1), unix(2026, 9, 28, 43_210));
        assert_eq!(add_months_php(unix(2026, 11, 15, 0), 3), unix(2027, 2, 15, 0));
        assert_eq!(add_months_php(unix(2026, 5, 31, 0), 12), unix(2027, 5, 31, 0));
    }

    #[test]
    fn a_single_month_is_bought_exactly() {
        let now = unix(2026, 8, 28, 0);
        let update = apply_payment(COSTS, None, 0.0, DEFAULT_MONTHLY_COST, now);
        assert_eq!(update.months_paid, 1);
        assert_eq!(update.rest_amount, 0.0);
        assert_eq!(update.amount_needed_for_next_month, 0.0);
        assert_eq!(update.paid_until, Some(unix(2026, 9, 28, 0)));
    }

    #[test]
    fn rests_carry_over_between_payments() {
        let now = unix(2026, 8, 28, 0);
        // 150M: one month, 50M held back.
        let update = apply_payment(COSTS, None, 0.0, 150_000_000.0, now);
        assert_eq!(update.months_paid, 1);
        assert_eq!(update.rest_amount, 50_000_000.0);

        // A later 60M tops the 50M rest up to 110M: another month, 10M
        // rest.
        let update = apply_payment(COSTS, update.paid_until, update.rest_amount, 60_000_000.0, now);
        assert_eq!(update.months_paid, 1);
        assert_eq!(update.rest_amount, 10_000_000.0);
        assert_eq!(update.paid_until, Some(unix(2026, 10, 28, 0)));
    }

    #[test]
    fn under_a_month_only_accumulates() {
        let now = unix(2026, 8, 28, 0);
        let update = apply_payment(COSTS, None, 0.0, 40_000_000.0, now);
        assert_eq!(update.months_paid, 0);
        assert_eq!(update.rest_amount, 40_000_000.0);
        assert_eq!(update.amount_needed_for_next_month, 60_000_000.0);
        assert_eq!(update.paid_until, None);

        // The carried rest counts toward the missing amount.
        let update = apply_payment(COSTS, None, 40_000_000.0, 25_000_000.0, now);
        assert_eq!(update.months_paid, 0);
        assert_eq!(update.rest_amount, 65_000_000.0);
        assert_eq!(update.amount_needed_for_next_month, 35_000_000.0);
    }

    #[test]
    fn yearly_blocks_apply_the_discount() {
        let now = unix(2026, 8, 28, 0);
        // Exactly one year.
        let update = apply_payment(COSTS, None, 0.0, DEFAULT_YEARLY_COST, now);
        assert_eq!(update.months_paid, 12);
        assert_eq!(update.rest_amount, 0.0);
        assert_eq!(update.paid_until, Some(unix(2027, 8, 28, 0)));

        // 2.35B: two yearly blocks, three loose months, 50M rest.
        let update = apply_payment(COSTS, None, 0.0, 2_350_000_000.0, now);
        assert_eq!(update.months_paid, 27);
        assert_eq!(update.rest_amount, 50_000_000.0);
        assert_eq!(update.paid_until, Some(unix(2028, 11, 28, 0)));

        // 999,999,999.99 stays nine loose months, no discount.
        let update = apply_payment(COSTS, None, 0.0, 999_999_999.99, now);
        assert_eq!(update.months_paid, 9);
        assert!((update.rest_amount - 99_999_999.99).abs() < 1e-3);
    }

    #[test]
    fn live_premium_extends_and_lapsed_premium_restarts() {
        let now = unix(2026, 8, 28, 0);
        // Live until Sep 10: a month lands on Oct 10.
        let update =
            apply_payment(COSTS, Some(unix(2026, 9, 10, 0)), 0.0, DEFAULT_MONTHLY_COST, now);
        assert_eq!(update.paid_until, Some(unix(2026, 10, 10, 0)));

        // Expired in July: the month restarts from now.
        let update =
            apply_payment(COSTS, Some(unix(2026, 7, 1, 0)), 0.0, DEFAULT_MONTHLY_COST, now);
        assert_eq!(update.paid_until, Some(unix(2026, 9, 28, 0)));
    }
}
