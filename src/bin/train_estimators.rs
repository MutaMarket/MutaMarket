//! One-shot estimator training, the legacy `app:estimator:train` /
//! `app:estimator:train-type` commands.
//!
//! Usage: `cargo run --release --bin train_estimators [type-id-or-name]`
//! Without an argument every mutaplasmid output type is trained; with one
//! only the matching type (numeric id, or exact name like the legacy
//! command's firstOrFail lookup).

use mutamarket::db;
use mutamarket::estimator::training::{self, TrainOutcome};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let pool = db::connect().await?;
    db::migrate(&pool).await?;

    match std::env::args().nth(1) {
        Some(argument) => {
            let type_id: Option<i64> = match argument.parse::<i64>() {
                Ok(id) => sqlx::query_scalar("select id from types where id = $1")
                    .bind(id)
                    .fetch_optional(&pool)
                    .await?,
                Err(_) => sqlx::query_scalar("select id from types where name = $1")
                    .bind(&argument)
                    .fetch_optional(&pool)
                    .await?,
            };
            let Some(type_id) = type_id else {
                return Err(format!("no type matches '{argument}'").into());
            };

            match training::train_type(&pool, type_id).await? {
                TrainOutcome::Trained {
                    data_count,
                    rows,
                    metrics,
                } => println!(
                    "trained on {rows} rows ({data_count} sold modules): \
                     r2 {:.2}, mae {:.2}, nmae {:.2}",
                    metrics.r2, metrics.mae, metrics.nmae,
                ),
                TrainOutcome::NotEnoughData { data_count } => println!(
                    "not enough data: {data_count} sold modules (minimum {})",
                    training::MINIMUM_DATA_COUNT,
                ),
                TrainOutcome::NoFeatures => println!("type has no estimator attributes"),
            }
        }
        None => {
            let run = training::train_all(&pool, |line| println!("{line}")).await?;
            println!(
                "{} types trained, {} skipped, {} module estimates cleared",
                run.trained, run.skipped, run.cleared,
            );
        }
    }

    Ok(())
}
