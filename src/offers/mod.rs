//! The offers domain: message threads between two characters about one
//! module, ported from the legacy `OfferController`, `MessageController`
//! and the `LeaveOffer`/`SendMessage` actions.
//!
//! An offer is a conversation, not a bid ledger: one active thread per
//! (sender character, receiver character, module), soft-deleted once
//! both sides leave. Deliberate divergence: offers carry an explicit
//! `price` (see the offers migration).

use sqlx::PgPool;

/// Why creating an offer was refused, with the legacy flash texts.
#[derive(Debug)]
pub enum CreateOfferError {
    /// The receiver's user has blocked the sender (`OfferPolicy::create`).
    Blocked,
    /// An active offer by this sender for this module and receiver
    /// already exists (the `lockForUpdate` duplicate check).
    Duplicate,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for CreateOfferError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error)
    }
}

/// The legacy `OfferController::store`: guarded by the block list, one
/// active offer per (sender, receiver, module), created together with
/// its first message in one transaction. Returns the new offer id.
pub async fn create_offer(
    pool: &PgPool,
    sender_character: i64,
    receiver_character: i64,
    module_id: i64,
    price: f64,
    message: &str,
) -> Result<i64, CreateOfferError> {
    let blocked: bool = sqlx::query_scalar(
        "select exists(
             select 1 from blocked_users b
             join characters receiver on receiver.user_id = b.blocker_id
             join characters sender on sender.user_id = b.blocked_id
             where receiver.id = $1 and sender.id = $2)",
    )
    .bind(receiver_character)
    .bind(sender_character)
    .fetch_one(pool)
    .await?;
    if blocked {
        return Err(CreateOfferError::Blocked);
    }

    let mut tx = pool.begin().await?;

    let existing: Option<i64> = sqlx::query_scalar(
        "select id from offers
         where sender_id = $1 and receiver_id = $2 and module_id = $3
           and deleted_at is null and left_by_sender_at is null
         for update",
    )
    .bind(sender_character)
    .bind(receiver_character)
    .bind(module_id)
    .fetch_optional(&mut *tx)
    .await?;
    if existing.is_some() {
        return Err(CreateOfferError::Duplicate);
    }

    let offer_id: i64 = sqlx::query_scalar(
        "insert into offers (sender_id, receiver_id, module_id, price)
         values ($1, $2, $3, $4) returning id",
    )
    .bind(sender_character)
    .bind(receiver_character)
    .bind(module_id)
    .bind(price)
    .fetch_one(&mut *tx)
    .await?;

    // The first message ships with the offer; like legacy it counts as
    // already notified (the OfferReceived notification covers it).
    sqlx::query(
        "insert into messages (offer_id, sender_id, receiver_id, content, notified_at)
         values ($1, $2, $3, $4, now())",
    )
    .bind(offer_id)
    .bind(sender_character)
    .bind(receiver_character)
    .bind(message)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(offer_id)
}

/// One offer row as the domain sees it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OfferRow {
    pub id: i64,
    pub sender_id: i64,
    pub receiver_id: i64,
    pub module_id: i64,
    pub price: f64,
    pub left_by_sender: bool,
    pub left_by_receiver: bool,
}

/// The live (not soft-deleted) offer, if any.
pub async fn offer(pool: &PgPool, offer_id: i64) -> sqlx::Result<Option<OfferRow>> {
    sqlx::query_as(
        "select id, sender_id, receiver_id, module_id, price,
                left_by_sender_at is not null as left_by_sender,
                left_by_receiver_at is not null as left_by_receiver
         from offers where id = $1 and deleted_at is null",
    )
    .bind(offer_id)
    .fetch_optional(pool)
    .await
}

/// The user's character ids (the legacy `getCharacterIds()`).
pub async fn user_character_ids(pool: &PgPool, user_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("select id from characters where user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

impl OfferRow {
    /// Whether one of the user's characters takes part.
    pub fn involves(&self, character_ids: &[i64]) -> bool {
        character_ids.contains(&self.sender_id) || character_ids.contains(&self.receiver_id)
    }

    /// The user's side of the thread (the legacy `getCharacter`).
    pub fn own_character(&self, character_ids: &[i64]) -> Option<i64> {
        if character_ids.contains(&self.sender_id) {
            Some(self.sender_id)
        } else if character_ids.contains(&self.receiver_id) {
            Some(self.receiver_id)
        } else {
            None
        }
    }

    /// The counterpart of [`Self::own_character`].
    pub fn other_character(&self, character_ids: &[i64]) -> Option<i64> {
        self.own_character(character_ids).map(|own| {
            if own == self.sender_id { self.receiver_id } else { self.sender_id }
        })
    }
}

/// The legacy `SendMessage::handle`: appends to the thread, addressed
/// from the user's side to the other. Returns the message id, or `None`
/// when the user is not part of the offer.
pub async fn send_message(
    pool: &PgPool,
    offer: &OfferRow,
    user_id: i64,
    content: &str,
) -> sqlx::Result<Option<i64>> {
    let characters = user_character_ids(pool, user_id).await?;
    let (Some(sender), Some(receiver)) =
        (offer.own_character(&characters), offer.other_character(&characters))
    else {
        return Ok(None);
    };

    let id = sqlx::query_scalar(
        "insert into messages (offer_id, sender_id, receiver_id, content)
         values ($1, $2, $3, $4) returning id",
    )
    .bind(offer.id)
    .bind(sender)
    .bind(receiver)
    .bind(content)
    .fetch_one(pool)
    .await?;
    Ok(Some(id))
}

/// The legacy `LeaveOffer::handle`: a self-offer (both characters mine)
/// is deleted outright; otherwise the leaving side is stamped and its
/// unread messages marked read, and the offer is deleted once both
/// sides have left. Returns false when the user is not part of it.
pub async fn leave_offer(pool: &PgPool, offer: &OfferRow, user_id: i64) -> sqlx::Result<bool> {
    let characters = user_character_ids(pool, user_id).await?;
    if !offer.involves(&characters) {
        return Ok(false);
    }

    if characters.contains(&offer.sender_id) && characters.contains(&offer.receiver_id) {
        sqlx::query("update offers set deleted_at = now(), updated_at = now() where id = $1")
            .bind(offer.id)
            .execute(pool)
            .await?;
        return Ok(true);
    }

    let is_sender = characters.contains(&offer.sender_id);
    let column = if is_sender { "left_by_sender_at" } else { "left_by_receiver_at" };
    sqlx::query(&format!(
        "update offers set {column} = now(), updated_at = now() where id = $1"
    ))
    .bind(offer.id)
    .execute(pool)
    .await?;

    let leaving = if is_sender { offer.sender_id } else { offer.receiver_id };
    sqlx::query(
        "update messages set read_at = now() where offer_id = $1 and receiver_id = $2 and read_at is null",
    )
    .bind(offer.id)
    .bind(leaving)
    .execute(pool)
    .await?;

    sqlx::query(
        "update offers set deleted_at = now()
         where id = $1 and left_by_sender_at is not null and left_by_receiver_at is not null",
    )
    .bind(offer.id)
    .execute(pool)
    .await?;
    Ok(true)
}

/// The legacy `CreateBlockedUserAction::handle`: records the block, then
/// leaves every live offer between the two users — in each direction the
/// RECEIVER of the offer leaves it (the legacy passes `$user`, the
/// receiving side, to `LeaveOffer`), so senders keep seeing their sent
/// threads marked left by the other side. Divergence: legacy wrapped
/// this in one DB transaction; here the insert commits first and the
/// leaves run through the ported [`leave_offer`] statements.
pub async fn block_user(
    pool: &PgPool,
    blocker_user_id: i64,
    blocked_user_id: i64,
) -> sqlx::Result<()> {
    sqlx::query("insert into blocked_users (blocker_id, blocked_id) values ($1, $2)")
        .bind(blocker_user_id)
        .bind(blocked_user_id)
        .execute(pool)
        .await?;

    for (sender_user, receiver_user) in
        [(blocked_user_id, blocker_user_id), (blocker_user_id, blocked_user_id)]
    {
        let offers: Vec<OfferRow> = sqlx::query_as(
            "select o.id, o.sender_id, o.receiver_id, o.module_id, o.price,
                    o.left_by_sender_at is not null as left_by_sender,
                    o.left_by_receiver_at is not null as left_by_receiver
             from offers o
             join characters sc on sc.id = o.sender_id
             join characters rc on rc.id = o.receiver_id
             where sc.user_id = $1 and rc.user_id = $2 and o.deleted_at is null",
        )
        .bind(sender_user)
        .bind(receiver_user)
        .fetch_all(pool)
        .await?;
        for offer in offers {
            leave_offer(pool, &offer, receiver_user).await?;
        }
    }
    Ok(())
}

/// Whether the user already blocks the other (the legacy
/// `StoreBlockedUserRequest::authorize` guard, inverted).
pub async fn is_blocked(
    pool: &PgPool,
    blocker_user_id: i64,
    blocked_user_id: i64,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "select exists(select 1 from blocked_users where blocker_id = $1 and blocked_id = $2)",
    )
    .bind(blocker_user_id)
    .bind(blocked_user_id)
    .fetch_one(pool)
    .await
}

/// One offer of the index listing: the thread heads the legacy
/// `OfferController::index` renders, newest conversation first.
#[derive(Debug, sqlx::FromRow)]
pub struct OfferListRow {
    pub id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    pub receiver_id: i64,
    pub receiver_name: String,
    pub module_id: i64,
    pub module_type_id: i64,
    pub module_type_name: String,
    pub price: f64,
    pub latest_content: String,
    pub latest_sender_id: i64,
    /// ISO-8601 UTC, for display.
    pub latest_created_at: String,
    pub latest_read: bool,
    pub created_at: String,
}

/// Offers involving the user's characters that their side has not left,
/// ordered by the newest message (the legacy index query).
pub async fn offers_for_user(pool: &PgPool, user_id: i64) -> sqlx::Result<Vec<OfferListRow>> {
    sqlx::query_as(
        "select o.id, o.sender_id, sc.name as sender_name,
                o.receiver_id, rc.name as receiver_name,
                o.module_id, mo.type_id as module_type_id, t.name as module_type_name,
                o.price,
                to_char(o.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') as created_at,
                m.content as latest_content, m.sender_id as latest_sender_id,
                to_char(m.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') as latest_created_at,
                (m.read_at is not null) as latest_read
         from offers o
         join characters sc on sc.id = o.sender_id
         join characters rc on rc.id = o.receiver_id
         join modules mo on mo.id = o.module_id
         join types t on t.id = mo.type_id
         cross join lateral (
             select content, sender_id, created_at, read_at
             from messages where offer_id = o.id
             order by created_at desc, id desc limit 1
         ) m
         where o.deleted_at is null
           and ((sc.user_id = $1 and o.left_by_sender_at is null)
             or (rc.user_id = $1 and o.left_by_receiver_at is null))
         order by m.created_at desc",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// One message of a thread.
#[derive(Debug, sqlx::FromRow)]
pub struct MessageRow {
    pub id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    pub receiver_id: i64,
    pub receiver_name: String,
    pub content: String,
    /// ISO-8601 UTC, for display.
    pub created_at: String,
}

/// The thread, oldest first (the legacy show page order).
pub async fn offer_messages(pool: &PgPool, offer_id: i64) -> sqlx::Result<Vec<MessageRow>> {
    sqlx::query_as(
        "select m.id, m.sender_id, sc.name as sender_name,
                m.receiver_id, rc.name as receiver_name,
                m.content,
                to_char(m.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') as created_at
         from messages m
         join characters sc on sc.id = m.sender_id
         join characters rc on rc.id = m.receiver_id
         where m.offer_id = $1
         order by m.created_at, m.id",
    )
    .bind(offer_id)
    .fetch_all(pool)
    .await
}

/// The legacy `Message::markAsRead`: everything addressed to the user's
/// characters in this thread.
pub async fn mark_read(pool: &PgPool, offer_id: i64, user_id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "update messages set read_at = now()
         where offer_id = $1 and read_at is null
           and receiver_id in (select id from characters where user_id = $2)",
    )
    .bind(offer_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Unread messages addressed to the user, for the nav indicator.
pub async fn unread_count(pool: &PgPool, user_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "select count(*) from messages m
         join characters c on c.id = m.receiver_id
         join offers o on o.id = m.offer_id
         where c.user_id = $1 and m.read_at is null and o.deleted_at is null
           and (case when o.receiver_id = m.receiver_id
                     then o.left_by_receiver_at else o.left_by_sender_at end) is null",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// The user's active offers on the given modules (the legacy
/// `withLatestOfferMadeByUser`): sender is one of the user's characters,
/// their side not left, and the receiver still owns the public asset
/// backing the module (the `latest_offer` resource guard).
pub async fn active_offers_on_modules(
    pool: &PgPool,
    user_id: i64,
    module_ids: &[i64],
) -> sqlx::Result<std::collections::HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "select distinct on (o.module_id) o.module_id, o.id
         from offers o
         join characters sc on sc.id = o.sender_id
         where sc.user_id = $1 and o.module_id = any($2)
           and o.deleted_at is null and o.left_by_sender_at is null
           and exists(select 1 from public_assets pa
                      where pa.module_id = o.module_id
                        and pa.character_id = o.receiver_id)
         order by o.module_id, o.id desc",
    )
    .bind(user_id)
    .bind(module_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// All modules the user has an active sent offer on (for the card's
/// "Go to offer" swap), regardless of page: (module id, offer id).
pub async fn sent_offer_modules(pool: &PgPool, user_id: i64) -> sqlx::Result<Vec<(i64, i64)>> {
    sqlx::query_as(
        "select distinct on (o.module_id) o.module_id, o.id
         from offers o
         join characters sc on sc.id = o.sender_id
         where sc.user_id = $1
           and o.deleted_at is null and o.left_by_sender_at is null
           and exists(select 1 from public_assets pa
                      where pa.module_id = o.module_id
                        and pa.character_id = o.receiver_id)
         order by o.module_id, o.id desc",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    #[test]
    fn offer_sides_resolve() {
        let offer = super::OfferRow {
            id: 1,
            sender_id: 10,
            receiver_id: 20,
            module_id: 5,
            price: 1_000_000.0,
            left_by_sender: false,
            left_by_receiver: false,
        };
        assert!(offer.involves(&[10]));
        assert!(!offer.involves(&[30]));
        assert_eq!(offer.own_character(&[20, 30]), Some(20));
        assert_eq!(offer.other_character(&[20, 30]), Some(10));
        assert_eq!(offer.own_character(&[30]), None);
    }
}
