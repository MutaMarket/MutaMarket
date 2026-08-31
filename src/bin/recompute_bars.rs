//! One-off rescore of stored attribute bars.
//!
//! Bars are written during module ingest, so a change to the bar rules
//! only reaches modules that happen to be re-ingested afterwards. This
//! rescores what is already stored, without touching ESI: a bar depends
//! only on the attribute's final value and the module's mutaplasmid and
//! source type, all of which are in the database.
//!
//! It exists for the `Exigent` and `Radical` change (see `WEAK_MUTATORS`
//! in `src/mutation/bars.rs`), and stays because the next bar rule change
//! will need it too.
//!
//! Usage:
//!   `cargo run --bin recompute_bars`             every module
//!   `cargo run --bin recompute_bars Exigent`     mutaplasmids matching a prefix
//!   `cargo run --bin recompute_bars --dry-run`   report without writing

use std::collections::HashMap;

use mutamarket::db;
use mutamarket::mutation::reference::{ContextCache, ReferenceData};
use mutamarket::mutation::resolve_bar;
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let prefix = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_default();

    let pool = db::connect().await?;
    db::migrate(&pool).await?;
    let reference = ReferenceData::from_tables(db::reference::load_reference(&pool).await?);
    let mut contexts = ContextCache::new(&reference);

    // One pass over every stored roll of the selected modules. The filter
    // is a prefix on the mutaplasmid name so a rule change can be applied
    // to just the grades it touched.
    let rows = sqlx::query(
        "select ma.module_id, ma.attribute_id, ma.value, ma.bar,
                m.mutaplasmid_id, m.source_type_id
         from mutated_attributes ma
         join modules m on m.id = ma.module_id
         join mutaplasmids mp on mp.id = m.mutaplasmid_id
         where $1 = '' or mp.name like $1 || '%'
         order by ma.module_id, ma.attribute_id",
    )
    .bind(&prefix)
    .fetch_all(&pool)
    .await?;

    println!(
        "rescoring {} stored rolls{}",
        rows.len(),
        if prefix.is_empty() {
            String::new()
        } else {
            format!(" on mutaplasmids starting with {prefix:?}")
        },
    );

    let mut changes: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
    let mut missing_context = 0;

    for row in &rows {
        let module_id: i64 = row.get("module_id");
        let attribute_id: i64 = row.get("attribute_id");
        let stored: i16 = row.get("bar");
        let Some(context) = contexts.context(row.get("mutaplasmid_id"), row.get("source_type_id"))
        else {
            missing_context += 1;
            continue;
        };

        let rescored = resolve_bar(context, attribute_id, row.get("value")).as_int();
        if rescored != i64::from(stored) {
            changes
                .entry(module_id)
                .or_default()
                .push((attribute_id, rescored));
        }
    }

    let attribute_count: usize = changes.values().map(Vec::len).sum();
    println!(
        "{attribute_count} rolls on {} modules change{}",
        changes.len(),
        if missing_context > 0 {
            format!(" ({missing_context} skipped: no mutation context)")
        } else {
            String::new()
        },
    );

    if dry_run || changes.is_empty() {
        if dry_run {
            println!("dry run: nothing written");
        }
        return Ok(());
    }

    let mut transaction = pool.begin().await?;
    for (module_id, attributes) in &changes {
        for (attribute_id, bar) in attributes {
            sqlx::query(
                "update mutated_attributes set bar = $3
                 where module_id = $1 and attribute_id = $2",
            )
            .bind(module_id)
            .bind(attribute_id)
            .bind(*bar as i16)
            .execute(&mut *transaction)
            .await?;
        }
    }
    transaction.commit().await?;

    println!("written");
    Ok(())
}
