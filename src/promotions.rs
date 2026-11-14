//! Promoted modules (no legacy counterpart): a donation whose in-game
//! reason names a module (`promote <item id>`, the item id alone, or the
//! link to the module's page) buys that module days at the top of
//! the for-sale browser's default order, at [`DAILY_COST`] per day. The
//! paid time is a running end date on the module, so donating again for
//! the same module extends it. The listing side lives in
//! `modules::search`; this module holds the price, the reason code and
//! the write.

use std::collections::HashMap;

use sqlx::{PgConnection, PgPool};

use crate::modules::view::{ModuleDetail, module_id_from_slug};

/// One day at the top of the browser: a quarter of a premium month, so
/// promoting a single module for a few days stays cheap next to the
/// module's price while a month of it costs well over premium.
pub const DAILY_COST: f64 = 25_000_000.0;

/// The word that opens the reason code, followed by the module's item
/// id (`promote 1047593018621`).
pub const REASON_KEYWORD: &str = "promote";

/// The path every module page link carries in front of its slug
/// (`https://mutamarket.com/modules/<slug>-<item id>`).
const MODULE_LINK_PATH: &str = "/modules/";

/// What a donation bought, for the confirmation mail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Promotion {
    pub module_id: i64,
    pub days: i32,
    /// What the days cost; the rest of the donation goes to premium.
    pub cost: f64,
    /// The new end of the module's promotion, in epoch seconds.
    pub until: i64,
}

/// The module a donation reason asks to promote: the item id or the
/// link to the module's page, alone or after the keyword, case and
/// surrounding whitespace ignored. Anything else is an ordinary
/// donation, and so is a number or link that names no module for sale
/// (see [`promote`]), so a stray number in a gift's note costs nothing.
pub fn module_from_reason(reason: &str) -> Option<i64> {
    let mut words = reason.split_whitespace();
    let first = words.next()?;
    let target = if first.eq_ignore_ascii_case(REASON_KEYWORD) {
        words.next()?
    } else {
        first
    };
    let module_id = target.parse().ok().or_else(|| module_from_link(target))?;
    words.next().is_none().then_some(module_id)
}

/// The item id out of a module page link, with or without scheme and
/// host, query or fragment. The slug resolves like the page route does,
/// so the short `/modules/<item id>` form works too.
fn module_from_link(link: &str) -> Option<i64> {
    let (_, rest) = link.split_once(MODULE_LINK_PATH)?;
    let slug = rest.split(['/', '?', '#']).next()?;
    module_id_from_slug(slug)
}

/// The whole days an amount pays for.
pub fn days_for(amount: f64) -> i32 {
    (amount / DAILY_COST).floor() as i32
}

/// Promotes a module with a donation, extending a running promotion and
/// restarting a lapsed one from now. `None` when the amount does not
/// cover a day or the module is not for sale (no contract and no public
/// listing): the caller then treats the donation as an ordinary one, so
/// nobody pays for a card the browser would not show. The clock keeps
/// running when the listing ends early.
pub async fn promote(
    conn: &mut PgConnection,
    module_id: i64,
    donation_id: i64,
    amount: f64,
) -> sqlx::Result<Option<Promotion>> {
    let days = days_for(amount);
    if days < 1 {
        return Ok(None);
    }
    let until: Option<i64> = sqlx::query_scalar(
        "update modules m
         set promoted_until = greatest(coalesce(m.promoted_until, now()), now())
                              + make_interval(days => $2)
         where m.id = $1
           and (m.latest_contract_id is not null
                or exists (select 1 from public_assets pa where pa.module_id = m.id))
         returning extract(epoch from m.promoted_until)::bigint",
    )
    .bind(module_id)
    .bind(days)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(until) = until else {
        return Ok(None);
    };
    sqlx::query("insert into module_promotions (module_id, donation_id, days) values ($1, $2, $3)")
        .bind(module_id)
        .bind(donation_id)
        .bind(days)
        .execute(&mut *conn)
        .await?;
    Ok(Some(Promotion {
        module_id,
        days,
        cost: f64::from(days) * DAILY_COST,
        until,
    }))
}

/// Marks the cards whose promotion is running with its end date; the
/// others keep the key absent.
pub async fn attach_promotions(pool: &PgPool, modules: &mut [ModuleDetail]) -> sqlx::Result<()> {
    let ids: Vec<i64> = modules.iter().map(|module| module.id).collect();
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "select id, to_char(promoted_until at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
         from modules where id = any($1) and promoted_until > now()",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let mut by_module: HashMap<i64, String> = rows.into_iter().collect();
    for module in modules {
        module.promoted_until = by_module.remove(&module.id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reason_code_is_an_item_id_with_or_without_the_keyword() {
        assert_eq!(
            module_from_reason("promote 1047593018621"),
            Some(1_047_593_018_621)
        );
        assert_eq!(module_from_reason("  Promote   42 "), Some(42));
        assert_eq!(module_from_reason("1047593018621"), Some(1_047_593_018_621));
        assert_eq!(module_from_reason(" 42 "), Some(42));
    }

    #[test]
    fn a_module_link_is_a_reason_code_with_or_without_the_keyword() {
        for reason in [
            "https://mutamarket.com/modules/50mn-abyssal-microwarpdrive-1047593018621",
            "mutamarket.com/modules/50mn-abyssal-microwarpdrive-1047593018621",
            "https://mutamarket.com/modules/1047593018621",
            "/modules/1047593018621",
            " https://mutamarket.com/modules/abyssal-heat-sink-1047593018621?ref=x#top ",
            "https://mutamarket.com/modules/abyssal-heat-sink-1047593018621/",
            "promote https://mutamarket.com/modules/abyssal-heat-sink-1047593018621",
        ] {
            assert_eq!(
                module_from_reason(reason),
                Some(1_047_593_018_621),
                "reason: {reason:?}"
            );
        }
    }

    #[test]
    fn other_reasons_are_ordinary_donations() {
        for reason in [
            "",
            "promote",
            "promote abc",
            "promote 42 please",
            "thanks for the site",
            "42 thanks",
            "4.2",
            "promoted 42",
            // Links that name no module.
            "https://mutamarket.com/modules",
            "https://mutamarket.com/modules/",
            "https://mutamarket.com/modules/type/47408",
            "https://mutamarket.com/collections/best-42",
            "https://mutamarket.com/modules/heat-sink-42 thanks",
        ] {
            assert_eq!(module_from_reason(reason), None, "reason: {reason:?}");
        }
    }

    #[test]
    fn an_amount_buys_whole_days() {
        assert_eq!(days_for(DAILY_COST - 1.0), 0);
        assert_eq!(days_for(DAILY_COST), 1);
        assert_eq!(days_for(DAILY_COST * 3.5), 3);
    }
}
