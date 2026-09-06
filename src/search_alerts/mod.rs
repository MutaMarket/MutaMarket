//! Saved search alerts, a rewrite addition (the legacy application had no
//! way to follow a query): a signed-in user saves the module browser's
//! filter query from the bell button, and the `search-alerts` scheduler
//! job re-runs every saved query against the listings that appeared
//! since the alert's last check. Each run queues at most one
//! notification per alert, carrying the new matches, through the same
//! outbox as the offer notifications (Discord when linked, EVE mail
//! otherwise).
//!
//! Alerts are a premium feature: saving needs premium, and an alert
//! whose account lost premium pauses (its window keeps closing, so
//! nothing piles up) until premium returns.
//!
//! "Appeared" is read off `public_module_ownerships`, the one row per
//! (character, module) that exists exactly while a module is on a
//! public contract or a published asset. Each alert keeps a
//! `checked_until` instant; a run looks at the rows created after it and
//! up to the run's cutoff, and the search's own for-sale filter decides
//! which of them the saved query would list. The windows never overlap,
//! so nothing is reported twice, and a seller renewing a contract
//! updates the existing row in place, so renewals stay quiet: only a
//! module that (re)appears on the market, or one listed by another
//! character, fires again.

use std::collections::HashMap;

use sqlx::{PgPool, Row};

use crate::modules::search::{self, Search, SearchError};
use crate::modules::view::{build_query_path, module_slug, parse_query_ui};
use crate::mutation::reference::ReferenceData;

/// How many alerts one account may hold: keeps the job's per-alert
/// query fan-out bounded and nudges users towards specific searches.
pub const MAX_ALERTS_PER_USER: i64 = 10;

/// The outbox `kind` of an alert notification.
pub const KIND: &str = "search-alert";

/// How old a listing row must be before a run closes its window over
/// it. `created_at` is stamped when the inserting transaction starts,
/// not when it commits, so a row can become visible after a run already
/// moved `checked_until` past its timestamp and would then never be
/// seen. Holding the cutoff this far behind the clock covers any import
/// transaction; it delays every alert by the same amount.
pub const CANDIDATE_MATURITY_SECONDS: f64 = 5.0 * 60.0;

/// How many matches a notification lists by name; the rest is a count
/// plus the link to the search, keeping EVE mails under their body cap.
const MAX_LISTED_MATCHES: usize = 10;

/// One saved alert as the settings page and the browser's bell see it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SearchAlert {
    pub id: i64,
    /// The normalized query path, without prefix (`type/47408/goldbar`).
    pub query: String,
    /// The query's type filter, for the settings list's label.
    #[serde(rename = "type")]
    pub type_filter: Option<AlertType>,
    /// ISO-8601 UTC instants.
    pub created_at: String,
    pub last_notified_at: Option<String>,
    pub notified_count: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AlertType {
    pub id: i64,
    pub name: String,
}

/// Why a query could not be saved.
#[derive(Debug)]
pub enum CreateError {
    /// The query needs a type filter: without one an alert would fire on
    /// every listing of the market.
    MissingType,
    /// The account already holds [`MAX_ALERTS_PER_USER`] alerts.
    TooMany,
    /// The query did not parse (unknown type, bad attribute...).
    Search(SearchError),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for CreateError {
    fn from(error: sqlx::Error) -> Self {
        CreateError::Db(error)
    }
}

/// The saved form of a browser query: the page, the sort and the
/// account-relative options (`with-personal-modules`, the personal
/// page's `without-*` exclusions, the character page's `created`) are
/// dropped, since an alert has no page and no viewer, and the remaining
/// options are re-emitted in the builder's canonical order so the same
/// search always saves as the same string.
pub fn normalize_query(query: &str) -> String {
    let mut ui = parse_query_ui(query);
    ui.page = 1;
    ui.sort = None;
    ui.with_personal_modules = false;
    ui.created = false;
    ui.without_fitted = false;
    ui.without_assets = false;
    ui.without_contracts = false;
    build_query_path("", &ui).trim_start_matches('/').to_owned()
}

/// Saves the query for the user, or answers the alert that already
/// holds the same normalized query. The bool is `true` when a row was
/// created.
pub async fn create(
    pool: &PgPool,
    reference: &ReferenceData,
    user_id: i64,
    query: &str,
) -> Result<(SearchAlert, bool), CreateError> {
    let normalized = normalize_query(query);
    let search = search::parse(pool, reference, &normalized)
        .await
        .map_err(CreateError::Search)?;
    let Some(type_filter) = &search.type_filter else {
        return Err(CreateError::MissingType);
    };

    if let Some(existing) = find_by_query(pool, user_id, &normalized).await? {
        return Ok((existing, false));
    }

    let count: i64 = sqlx::query_scalar("select count(*) from search_alerts where user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    if count >= MAX_ALERTS_PER_USER {
        return Err(CreateError::TooMany);
    }

    // A concurrent save of the same query loses the unique race and
    // reads the winner back.
    sqlx::query(
        "insert into search_alerts (user_id, query, type_id) values ($1, $2, $3)
         on conflict (user_id, query) do nothing",
    )
    .bind(user_id)
    .bind(&normalized)
    .bind(type_filter.id)
    .execute(pool)
    .await?;

    let alert = find_by_query(pool, user_id, &normalized)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    Ok((alert, true))
}

/// The SELECT list every alert read shares.
const ALERT_COLUMNS: &str = "a.id, a.query, a.type_id, t.name as type_name,
        to_char(a.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') as created_at,
        to_char(a.last_notified_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
            as last_notified_at,
        a.notified_count";

fn alert_from_row(row: &sqlx::postgres::PgRow) -> SearchAlert {
    let type_id: Option<i64> = row.get("type_id");
    let type_name: Option<String> = row.get("type_name");
    SearchAlert {
        id: row.get("id"),
        query: row.get("query"),
        type_filter: type_id
            .zip(type_name)
            .map(|(id, name)| AlertType { id, name }),
        created_at: row.get("created_at"),
        last_notified_at: row.get("last_notified_at"),
        notified_count: row.get("notified_count"),
    }
}

async fn find_by_query(
    pool: &PgPool,
    user_id: i64,
    query: &str,
) -> sqlx::Result<Option<SearchAlert>> {
    // The column list is a private constant, no user input.
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "select {ALERT_COLUMNS} from search_alerts a
         left join types t on t.id = a.type_id
         where a.user_id = $1 and a.query = $2"
    )))
    .bind(user_id)
    .bind(query)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(alert_from_row))
}

/// The user's alerts, newest first.
pub async fn list(pool: &PgPool, user_id: i64) -> sqlx::Result<Vec<SearchAlert>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "select {ALERT_COLUMNS} from search_alerts a
         left join types t on t.id = a.type_id
         where a.user_id = $1
         order by a.id desc"
    )))
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(alert_from_row).collect())
}

/// Deletes the user's alert; `false` when no such alert belongs to them.
pub async fn delete(pool: &PgPool, user_id: i64, alert_id: i64) -> sqlx::Result<bool> {
    let deleted = sqlx::query("delete from search_alerts where id = $1 and user_id = $2")
        .bind(alert_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(deleted.rows_affected() > 0)
}

/// What one job run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RunStats {
    /// Alerts checked.
    pub alerts: i64,
    /// Alerts that got a notification queued.
    pub notified: i64,
    /// New matches reported across all alerts.
    pub matches: i64,
    /// Alerts skipped because their account holds no premium.
    pub paused: i64,
}

/// The `search-alerts` job: checks every alert against the listings
/// that appeared since its last check and queues one notification per
/// alert with the matches. See [`CANDIDATE_MATURITY_SECONDS`].
pub async fn run(pool: &PgPool, reference: &ReferenceData) -> sqlx::Result<RunStats> {
    run_with_maturity(pool, reference, CANDIDATE_MATURITY_SECONDS).await
}

/// [`run`] with an explicit maturity, so tests can close the window on
/// rows they just inserted.
pub async fn run_with_maturity(
    pool: &PgPool,
    reference: &ReferenceData,
    maturity_seconds: f64,
) -> sqlx::Result<RunStats> {
    let mut stats = RunStats::default();

    // One cutoff for the whole run: every alert's window closes at it.
    let cutoff: String = sqlx::query_scalar("select (now() - make_interval(secs => $1))::text")
        .bind(maturity_seconds)
        .fetch_one(pool)
        .await?;

    // Each alert with whether its account currently holds premium (the
    // legacy PremiumMiddleware rule: admins pass, else a premium
    // character).
    let alerts: Vec<(i64, i64, String, bool)> = sqlx::query_as(
        "select a.id, a.user_id, a.query,
                u.is_admin or exists (select 1 from characters c
                                      where c.user_id = u.id and c.premium_paid_until > now())
         from search_alerts a
         join users u on u.id = a.user_id
         order by a.id",
    )
    .fetch_all(pool)
    .await?;

    // Parsed once per distinct query; a query that no longer parses (its
    // type vanished from the SDE) is skipped and logged, not fatal.
    let mut parsed: HashMap<String, Option<Search>> = HashMap::new();

    for (alert_id, user_id, query, premium) in alerts {
        stats.alerts += 1;

        if !premium {
            stats.paused += 1;
            advance_window(pool, alert_id, &cutoff).await?;
            continue;
        }

        let candidates = candidates_for(pool, alert_id, &cutoff).await?;
        if candidates.is_empty() {
            advance_window(pool, alert_id, &cutoff).await?;
            continue;
        }

        if !parsed.contains_key(&query) {
            let search = match search::parse(pool, reference, &query).await {
                Ok(search) => Some(search),
                Err(SearchError::Db(error)) => return Err(error),
                Err(error) => {
                    tracing::warn!(
                        "search alert {alert_id} query {query:?} no longer parses: {error}"
                    );
                    None
                }
            };
            parsed.insert(query.clone(), search);
        }
        let Some(search) = parsed.get(&query).and_then(Option::as_ref) else {
            advance_window(pool, alert_id, &cutoff).await?;
            continue;
        };

        let matched = search::matching_module_ids(pool, search, &candidates).await?;
        if !matched.is_empty() {
            notify(pool, alert_id, user_id, &query, &matched).await?;
            stats.notified += 1;
            stats.matches += matched.len() as i64;
        }
        advance_window(pool, alert_id, &cutoff).await?;
    }

    Ok(stats)
}

/// The modules whose listing rows were created inside the alert's open
/// window, newest module first, each once.
async fn candidates_for(pool: &PgPool, alert_id: i64, cutoff: &str) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select distinct o.module_id
         from public_module_ownerships o
         where o.created_at > (select a.checked_until from search_alerts a where a.id = $1)
           and o.created_at <= $2::timestamptz
         order by o.module_id desc",
    )
    .bind(alert_id)
    .bind(cutoff)
    .fetch_all(pool)
    .await
}

/// Closes the alert's window at the cutoff. Never moves it backwards: a
/// cutoff older than the watermark (an alert saved within the maturity
/// margin) would otherwise reopen listings that existed before the
/// alert.
async fn advance_window(pool: &PgPool, alert_id: i64, cutoff: &str) -> sqlx::Result<()> {
    sqlx::query(
        "update search_alerts set checked_until = greatest(checked_until, $2::timestamptz)
         where id = $1",
    )
    .bind(alert_id)
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(())
}

/// A reported match: the module and its type name for the link text.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub module_id: i64,
    pub type_name: String,
}

async fn notify(
    pool: &PgPool,
    alert_id: i64,
    user_id: i64,
    query: &str,
    module_ids: &[i64],
) -> sqlx::Result<()> {
    let matches: Vec<(i64, String)> = sqlx::query_as(
        "select m.id, t.name from modules m
         join types t on t.id = m.type_id
         where m.id = any($1)
         order by m.id desc",
    )
    .bind(module_ids)
    .fetch_all(pool)
    .await?;
    let matches: Vec<Match> = matches
        .into_iter()
        .map(|(module_id, type_name)| Match {
            module_id,
            type_name,
        })
        .collect();

    // The notify pick, else the account's first character, like the
    // unread-message notifications.
    let receiver_name: Option<String> = sqlx::query_scalar(
        "select coalesce(nc_char.name, min_char.name)
         from users u
         left join notify_characters nc on nc.user_id = u.id
         left join characters nc_char on nc_char.id = nc.character_id and nc_char.user_id = u.id
         left join lateral (select name from characters where user_id = u.id
                            order by id limit 1) min_char on true
         where u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    let (subject, body) = alert_mail(receiver_name.as_deref().unwrap_or(""), query, &matches);
    crate::notifications::queue(
        pool,
        user_id,
        KIND,
        &subject,
        &body,
        serde_json::json!({
            "alert_id": alert_id,
            "query": query,
            "module_ids": module_ids,
            "discord": alert_discord(query, &matches),
        }),
    )
    .await?;

    sqlx::query(
        "update search_alerts
         set last_notified_at = now(), notified_count = notified_count + $2
         where id = $1",
    )
    .bind(alert_id)
    .bind(module_ids.len() as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// The sort the notification link opens the search with: newest listing
/// first, so the reported matches sit at the top of the page.
const NOTIFICATION_SORT: (&str, bool) = ("date-added", true);

/// The browser page of the saved search, sorted newest listing first.
pub fn search_url(query: &str) -> String {
    let mut ui = parse_query_ui(query);
    ui.sort = Some((NOTIFICATION_SORT.0.to_owned(), NOTIFICATION_SORT.1));
    format!(
        "{}{}",
        crate::notifications::site_origin(),
        build_query_path("modules", &ui)
    )
}

fn module_url(matched: &Match) -> String {
    format!(
        "{}/modules/{}",
        crate::notifications::site_origin(),
        module_slug(&matched.type_name, matched.module_id)
    )
}

/// The label of a match: its type name, and a short id suffix so a
/// batch of the same type still reads as distinct listings.
fn match_label(matched: &Match) -> String {
    format!("{} ({})", matched.type_name, matched.module_id)
}

/// The EVE mail of an alert run: the first [`MAX_LISTED_MATCHES`]
/// matches as in-game links, the rest as a count, and the link to the
/// saved search.
pub fn alert_mail(receiver_name: &str, query: &str, matches: &[Match]) -> (String, String) {
    let count = matches.len();
    let subject = if count == 1 {
        "New module matching your search alert".to_owned()
    } else {
        format!("{count} new modules matching your search alert")
    };
    let mut list: Vec<String> = matches
        .iter()
        .take(MAX_LISTED_MATCHES)
        .map(|matched| {
            format!(
                "<a href=\"{}\">{}</a>",
                module_url(matched),
                match_label(matched)
            )
        })
        .collect();
    if count > MAX_LISTED_MATCHES {
        list.push(format!("... and {} more", count - MAX_LISTED_MATCHES));
    }
    let body = format!(
        "Hello {receiver_name},\n\n\
         {noun} matching one of your search alerts {verb} listed on the market:\n\n\
         {list}\n\n\
         <a href=\"{search}\">View the search on MutaMarket</a>\n\n\
         Best regards,\nMutaMarket",
        noun = if count == 1 {
            "A new module".to_owned()
        } else {
            format!("{count} new modules")
        },
        verb = if count == 1 { "was" } else { "were" },
        list = list.join("\n"),
        search = search_url(query),
    );
    (subject, body)
}

/// The Discord message of an alert run, shaped like the offer embeds.
pub fn alert_discord(query: &str, matches: &[Match]) -> serde_json::Value {
    let count = matches.len();
    let content = if count == 1 {
        "A new module matches one of your search alerts!".to_owned()
    } else {
        format!("{count} new modules match one of your search alerts!")
    };
    let mut lines: Vec<String> = matches
        .iter()
        .take(MAX_LISTED_MATCHES)
        .map(|matched| format!("[{}]({})", match_label(matched), module_url(matched)))
        .collect();
    if count > MAX_LISTED_MATCHES {
        lines.push(format!("... and {} more", count - MAX_LISTED_MATCHES));
    }
    let mut message = serde_json::json!({
        "content": content,
        "embed": {
            "title": if count == 1 { "New search alert match".to_owned() } else { format!("{count} new search alert matches") },
            "description": lines.join("\n"),
            "url": search_url(query),
            "color": crate::notifications::DISCORD_EMBED_COLOR,
        }
    });
    // One match shows its card, like the offer embed.
    if let [only] = matches {
        message["embed"]["image"] = serde_json::json!({
            "url": format!("{}/og/module/{}.png", crate::notifications::site_origin(), only.module_id),
        });
    }
    message
}

#[cfg(test)]
mod tests {
    use super::{Match, alert_discord, alert_mail, normalize_query};

    #[test]
    fn normalizing_drops_page_sort_and_viewer_options() {
        assert_eq!(
            normalize_query(
                "type/47408/sort/price/asc/goldbar/with-personal-modules/without-fitted/page/3"
            ),
            "type/47408/goldbar"
        );
        assert_eq!(normalize_query("goldbar/type/47408"), "type/47408/goldbar");
        assert_eq!(normalize_query(""), "");
        assert_eq!(
            normalize_query("type/47408/attributes/cpu/10-20/contract-price/1000000/in-jita"),
            "type/47408/attributes/cpu/10-20/contract-price/1000000.00/in-jita"
        );
    }

    #[test]
    fn mail_lists_matches_and_the_search_link() {
        // SAFETY: the suite is single-threaded (RUST_TEST_THREADS=1).
        unsafe { std::env::set_var("STACK_ORIGIN", "https://mutamarket.com") };
        let matches = vec![
            Match {
                module_id: 42,
                type_name: "Abyssal Heat Sink".to_owned(),
            },
            Match {
                module_id: 41,
                type_name: "Abyssal Heat Sink".to_owned(),
            },
        ];
        let (subject, body) = alert_mail("Pilot", "type/2048/goldbar", &matches);
        assert_eq!(subject, "2 new modules matching your search alert");
        assert!(body.starts_with("Hello Pilot,\n\n2 new modules matching"));
        assert!(body.contains(
            "<a href=\"https://mutamarket.com/modules/abyssal-heat-sink-42\">Abyssal Heat Sink (42)</a>"
        ));
        assert!(body.contains(
            "<a href=\"https://mutamarket.com/modules/type/2048/sort/date-added/desc/goldbar\">"
        ));

        let (subject, _) = alert_mail("Pilot", "type/2048", &matches[..1]);
        assert_eq!(subject, "New module matching your search alert");
    }

    #[test]
    fn discord_embeds_one_match_with_its_card() {
        unsafe { std::env::set_var("STACK_ORIGIN", "https://mutamarket.com") };
        let one = vec![Match {
            module_id: 42,
            type_name: "Abyssal Heat Sink".to_owned(),
        }];
        let message = alert_discord("type/2048", &one);
        assert_eq!(
            message["content"],
            "A new module matches one of your search alerts!"
        );
        assert_eq!(
            message["embed"]["url"],
            "https://mutamarket.com/modules/type/2048/sort/date-added/desc"
        );
        assert_eq!(
            message["embed"]["image"]["url"],
            "https://mutamarket.com/og/module/42.png"
        );

        let many: Vec<Match> = (0..12)
            .map(|index| Match {
                module_id: index,
                type_name: "X".to_owned(),
            })
            .collect();
        let message = alert_discord("type/2048", &many);
        assert_eq!(message["embed"]["title"], "12 new search alert matches");
        assert!(message["embed"]["image"].is_null());
        assert!(
            message["embed"]["description"]
                .as_str()
                .expect("description")
                .ends_with("... and 2 more")
        );
    }
}
