//! Per-user module pricing, the legacy `StoreBulkModulePricings`: a bulk
//! upsert where a zero or negative price deletes the row instead.

use sqlx::PgPool;

/// One `{module_id, price}` entry of the bulk payload.
#[derive(Debug, Clone, Copy)]
pub struct PricingEntry {
    pub module_id: i64,
    pub price: f64,
}

/// The legacy `noPriceSet`: zero or negative prices delete the pricing.
/// (A null price cannot reach here — the request requires `numeric`.)
fn no_price_set(price: f64) -> bool {
    price <= 0.0
}

/// The legacy `StoreBulkModulePricings::handle`: upsert the priced
/// entries on (module, user), delete the rows of the unpriced ones.
/// Duplicate module ids keep the last occurrence, like MySQL's row-by-row
/// `ON DUPLICATE KEY` (Postgres' `ON CONFLICT` refuses to touch one row
/// twice in a single statement).
pub async fn store_module_pricings(
    pool: &PgPool,
    user_id: i64,
    entries: &[PricingEntry],
) -> sqlx::Result<()> {
    let mut upserts: Vec<PricingEntry> = Vec::new();
    let mut deletions: Vec<i64> = Vec::new();
    for entry in entries {
        if no_price_set(entry.price) {
            deletions.push(entry.module_id);
        } else {
            upserts.retain(|kept| kept.module_id != entry.module_id);
            upserts.push(*entry);
        }
    }

    let mut tx = pool.begin().await?;
    for entry in upserts {
        sqlx::query(
            "insert into module_pricing (module_id, user_id, price)
             values ($1, $2, $3)
             on conflict (module_id, user_id)
             do update set price = excluded.price, updated_at = now()",
        )
        .bind(entry.module_id)
        .bind(user_id)
        .bind(entry.price)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("delete from module_pricing where user_id = $1 and module_id = any($2)")
        .bind(user_id)
        .bind(&deletions)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::no_price_set;

    #[test]
    fn no_price_set_matches_legacy() {
        assert!(no_price_set(0.0));
        assert!(no_price_set(-1.0));
        assert!(!no_price_set(0.01));
        assert!(!no_price_set(1_000_000.0));
    }
}
