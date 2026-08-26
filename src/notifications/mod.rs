//! The notification outbox, replacing the legacy notification channels
//! (`App\Channels\EveMail`, `AppDiscordChannel`).
//!
//! Legacy sent EVE mails inline and, outside production, only logged
//! "Would send mail ...". Here every notification is first a persisted
//! `notification_outbox` row; the `notification-delivery` scheduler job
//! drains pending rows and picks the transport from `NOTIFY_DELIVERY`:
//! `esi` really sends the in-game mail, anything else marks the row
//! `simulated`. The dev stack therefore never mails anyone, while the
//! admin dashboard and tests can inspect exactly what would have gone
//! out.
//!
//! Deliberate divergences from legacy: the Discord channel is not
//! ported (it needs Discord account linking), and a user without a
//! `notify_characters` pick falls back to their first character instead
//! of being skipped - the settings page to choose one is not ported
//! yet, and silently dropping every notification would make the feature
//! dead on arrival.

use sqlx::PgPool;

/// The mail scope the sending character's token must carry.
pub const MAIL_SCOPE: &str = "esi-mail.send_mail.v1";

/// Environment switch for the delivery job: `esi` sends real mail.
pub const DELIVERY_ENV: &str = "NOTIFY_DELIVERY";

/// The character the mails are sent as (the legacy
/// `services.eveonline.character_id`, MutaMate).
pub const SENDER_ENV: &str = "NOTIFY_SENDER_CHARACTER_ID";

/// Minutes a message stays unread before its user is notified, the
/// legacy `offers.notify_after_minutes` config.
pub const NOTIFY_AFTER_MINUTES: i64 = 10;

/// The legacy `app:notify-users` scan: users with messages unread and
/// unnotified for [`NOTIFY_AFTER_MINUTES`] get one messages-received
/// notification queued and those messages stamped notified. Returns how
/// many users were notified.
pub async fn queue_unread_message_notifications(pool: &PgPool) -> sqlx::Result<i64> {
    // One row per user due a notification, with their unread threads
    // (offer id + module type name) for the mail body.
    type DueRow = (i64, String, Vec<i64>, Vec<String>);
    let due: Vec<DueRow> = sqlx::query_as(
        "select c.user_id,
                coalesce(nc_char.name, min_char.name, '') as receiver_name,
                array_agg(distinct o.id) as offer_ids,
                array_agg(distinct t.name) as type_names
         from messages m
         join characters c on c.id = m.receiver_id
         join offers o on o.id = m.offer_id and o.deleted_at is null
         join modules mo on mo.id = o.module_id
         join types t on t.id = mo.type_id
         left join notify_characters nc on nc.user_id = c.user_id
         left join characters nc_char on nc_char.id = nc.character_id
         left join lateral (select name from characters where user_id = c.user_id
                            order by id limit 1) min_char on true
         where m.read_at is null and m.notified_at is null
           and m.created_at <= now() - make_interval(mins => $1::int)
         group by c.user_id, nc_char.name, min_char.name",
    )
    .bind(NOTIFY_AFTER_MINUTES)
    .fetch_all(pool)
    .await?;

    let mut notified = 0i64;
    for (user_id, receiver_name, offer_ids, type_names) in &due {
        let offers: Vec<(i64, String)> =
            offer_ids.iter().copied().zip(type_names.iter().cloned()).collect();
        let (subject, body) = messages_received_mail(receiver_name, &offers);
        queue(
            pool,
            *user_id,
            "messages-received",
            &subject,
            &body,
            serde_json::json!({ "offer_ids": offer_ids }),
        )
        .await?;

        sqlx::query(
            "update messages set notified_at = now()
             where read_at is null and notified_at is null
               and receiver_id in (select id from characters where user_id = $1)",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        notified += 1;
    }
    Ok(notified)
}

/// Queues one notification for the user. Returns the outbox row id.
pub async fn queue(
    pool: &PgPool,
    user_id: i64,
    kind: &str,
    subject: &str,
    body: &str,
    payload: serde_json::Value,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "insert into notification_outbox (user_id, kind, subject, body, payload)
         values ($1, $2, $3, $4, $5) returning id",
    )
    .bind(user_id)
    .bind(kind)
    .bind(subject)
    .bind(body)
    .bind(payload)
    .fetch_one(pool)
    .await
}

/// A pending outbox row joined with its recipient character: the user's
/// `notify_characters` pick, falling back to their first character.
#[derive(Debug, sqlx::FromRow)]
pub struct PendingNotification {
    pub id: i64,
    pub user_id: i64,
    pub kind: String,
    pub subject: String,
    pub body: String,
    pub recipient_character_id: Option<i64>,
}

/// The undelivered rows, oldest first.
pub async fn pending(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<PendingNotification>> {
    sqlx::query_as(
        "select o.id, o.user_id, o.kind, o.subject, o.body,
                coalesce(nc.character_id,
                         (select id from characters c
                          where c.user_id = o.user_id order by c.id limit 1))
                    as recipient_character_id
         from notification_outbox o
         left join notify_characters nc on nc.user_id = o.user_id
         where o.delivered_at is null
         order by o.id
         limit $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Stamps a drained row: how it left the outbox, or why it could not.
pub async fn mark_delivered(
    pool: &PgPool,
    id: i64,
    delivery: &str,
    error: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "update notification_outbox
         set delivered_at = now(), delivery = $2, error = $3 where id = $1",
    )
    .bind(id)
    .bind(delivery)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

/// The `notifications/offer-received` blade template, with the price
/// line our offers add.
pub fn offer_received_mail(
    receiver_name: &str,
    sender_name: &str,
    type_id: i64,
    module_id: i64,
    type_name: &str,
    offer_id: i64,
    price: f64,
) -> (String, String) {
    let subject = "New Offer Received".to_owned();
    let body = format!(
        "Hello {receiver_name},\n\n\
         You have received an offer of {price} ISK from {sender_name} for your \
         <a href=\"showinfo:{type_id}//{module_id}\">{type_name}</a> \
         (<a href=\"https://mutamarket.com/offers/{offer_id}\">View on MutaMarket</a>).\n\n\
         Best regards,\nMutaMarket",
        price = format_isk(price),
    );
    (subject, body)
}

/// The `notifications/messages-received` blade template.
pub fn messages_received_mail(receiver_name: &str, offers: &[(i64, String)]) -> (String, String) {
    let subject = "New Messages Received".to_owned();
    let list = offers
        .iter()
        .map(|(id, type_name)| {
            format!("<a href=\"https://mutamarket.com/offers/{id}\">Offer for {type_name}</a>")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "Hello {receiver_name},\n\n\
         You have unread messages at some of your offers. Please visit our website to check them out.\n\n\
         <a href=\"https://mutamarket.com/offers\">View Offers</a>\n\n\
         Offers with unread messages:\n\n{list}\n\n\
         Best regards,\nMutaMarket",
    );
    (subject, body)
}

/// Thousands-separated whole ISK for mail bodies and default texts.
pub fn format_isk(value: f64) -> String {
    let whole = value.round() as i64;
    let digits = whole.abs().to_string();
    let mut grouped = String::new();
    for (index, char) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(char);
    }
    if whole < 0 { format!("-{grouped}") } else { grouped }
}

#[cfg(test)]
mod tests {
    #[test]
    fn isk_groups_thousands() {
        assert_eq!(super::format_isk(1_500_000_000.0), "1,500,000,000");
        assert_eq!(super::format_isk(950.0), "950");
        assert_eq!(super::format_isk(0.4), "0");
    }
}
