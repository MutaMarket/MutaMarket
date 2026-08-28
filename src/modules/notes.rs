//! Personal module notes and per-collection notes, the legacy
//! `StoreNotesAction` and `StoreCollectionNotesAction`: bulk upserts
//! where an entry with empty content deletes the note instead.

use sqlx::PgPool;

/// One `{module_id, content}` entry of the bulk payload.
#[derive(Debug, Clone)]
pub struct NoteEntry {
    pub module_id: i64,
    pub content: Option<String>,
}

/// The legacy split condition, PHP's `empty($note['content'])`: a null or
/// missing content deletes the note, and so do the empty string and the
/// literal string `"0"` (PHP treats `"0"` as falsy - a ported quirk, so a
/// note cannot hold just the character zero).
fn deletes_note(content: Option<&str>) -> bool {
    matches!(content, None | Some("") | Some("0"))
}

/// Splits entries the legacy way and dedupes upserts by module id keeping
/// the last occurrence: MySQL's `ON DUPLICATE KEY` applies batch rows one
/// by one, while Postgres' `ON CONFLICT` refuses to touch the same row
/// twice in one statement.
fn split(entries: &[NoteEntry]) -> (Vec<(i64, &str)>, Vec<i64>) {
    let mut upserts: Vec<(i64, &str)> = Vec::new();
    let mut deletions: Vec<i64> = Vec::new();

    for entry in entries {
        match entry.content.as_deref() {
            content if deletes_note(content) => deletions.push(entry.module_id),
            Some(content) => {
                upserts.retain(|(module_id, _)| *module_id != entry.module_id);
                upserts.push((entry.module_id, content));
            }
            None => unreachable!("deletes_note covers None"),
        }
    }

    (upserts, deletions)
}

/// The legacy `StoreNotesAction::handle`: upsert the non-empty entries on
/// (user, module), delete the notes of the empty ones.
pub async fn store_notes(pool: &PgPool, user_id: i64, entries: &[NoteEntry]) -> sqlx::Result<()> {
    let (upserts, deletions) = split(entries);

    let mut tx = pool.begin().await?;
    for (module_id, content) in upserts {
        sqlx::query(
            "insert into notes (user_id, module_id, content)
             values ($1, $2, $3)
             on conflict (user_id, module_id)
             do update set content = excluded.content, updated_at = now()",
        )
        .bind(user_id)
        .bind(module_id)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("delete from notes where user_id = $1 and module_id = any($2)")
        .bind(user_id)
        .bind(&deletions)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// The legacy `StoreCollectionNotesAction::handle`: like [`store_notes`]
/// but keyed on (collection, module), with every row written under the
/// collection owner's user id (`$collection->character->user_id`) - not
/// the acting user's. The conflict update leaves user_id untouched, like
/// the legacy upsert column list.
pub async fn store_collection_notes(
    pool: &PgPool,
    collection_id: i64,
    entries: &[NoteEntry],
) -> sqlx::Result<()> {
    let owner_user_id: Option<i64> = sqlx::query_scalar(
        "select c.user_id from collections col
         join characters c on c.id = col.character_id
         where col.id = $1",
    )
    .bind(collection_id)
    .fetch_one(pool)
    .await?;

    let (upserts, deletions) = split(entries);

    let mut tx = pool.begin().await?;
    for (module_id, content) in upserts {
        sqlx::query(
            "insert into collection_notes (collection_id, user_id, module_id, content)
             values ($1, $2, $3, $4)
             on conflict (collection_id, module_id)
             do update set content = excluded.content, updated_at = now()",
        )
        .bind(collection_id)
        .bind(owner_user_id)
        .bind(module_id)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("delete from collection_notes where collection_id = $1 and module_id = any($2)")
        .bind(collection_id)
        .bind(&deletions)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn php_empty_semantics() {
        assert!(deletes_note(None));
        assert!(deletes_note(Some("")));
        assert!(deletes_note(Some("0")), "PHP empty('0') is true - the ported quirk");
        assert!(!deletes_note(Some("0.0")));
        assert!(!deletes_note(Some(" ")));
        assert!(!deletes_note(Some("a note")));
    }

    #[test]
    fn split_keeps_the_last_duplicate() {
        let entries = [
            NoteEntry { module_id: 1, content: Some("first".into()) },
            NoteEntry { module_id: 2, content: Some("keep".into()) },
            NoteEntry { module_id: 1, content: Some("second".into()) },
            NoteEntry { module_id: 3, content: Some("".into()) },
        ];
        let (upserts, deletions) = split(&entries);
        assert_eq!(upserts, vec![(2, "keep"), (1, "second")]);
        assert_eq!(deletions, vec![3]);
    }
}
