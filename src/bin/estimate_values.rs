//! One-shot module value estimate pass, the legacy `app:estimate-values`
//! command: refreshes the stalest estimates for modules whose type has a
//! trained model, through the AI estimation server (`AI_HOST`/`AI_PORT`).
//!
//! Usage: `cargo run --bin estimate_values [count] [type-name-fragment]`
//! (default count from `AI_COUNT`, like the legacy `config('ai.COUNT')`;
//! the optional type fragment mirrors the legacy `--type` option).

use mutamarket::db;
use mutamarket::estimator::{self, EstimatorClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let count = match std::env::args().nth(1) {
        Some(argument) => argument.parse()?,
        None => estimator::estimate_count_from_env(),
    };
    let type_filter = std::env::args().nth(2);

    let pool = db::connect().await?;
    db::migrate(&pool).await?;

    let client = EstimatorClient::from_env();
    let run = estimator::estimate_values(&pool, &client, count, type_filter.as_deref()).await?;

    println!("estimated {} of {} modules", run.updated, run.attempted);

    Ok(())
}
