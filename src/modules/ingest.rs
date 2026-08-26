//! Persisting a mutated module from its raw ESI dogma data, ported from the
//! legacy `App\Actions\Modules\ProcessModule`: build the mutation context,
//! compute the per-attribute results, and upsert the module together with
//! its `mutated_attributes` rows and `average_fraction`.

use std::fmt;

use sqlx::PgPool;

use crate::esi::{EsiClient, EsiError};
use crate::estimator::Estimator;
use crate::mutation::calculator::{AttributeMutationResult, DogmaAttribute, average_fraction, calculate};
use crate::mutation::reference::ReferenceData;

/// A mutated item as returned by ESI's dogma endpoint.
#[derive(Debug, Clone)]
pub struct DogmaItem {
    pub created_by: i64,
    pub source_type_id: i64,
    pub mutator_type_id: i64,
    pub dogma_attributes: Vec<DogmaAttribute>,
}

#[derive(Debug)]
pub enum ProcessModuleError {
    /// The (mutaplasmid, source type) combination has no reference data.
    UnknownCombination {
        mutaplasmid_id: i64,
        source_type_id: i64,
    },
    Database(sqlx::Error),
}

impl fmt::Display for ProcessModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessModuleError::UnknownCombination {
                mutaplasmid_id,
                source_type_id,
            } => write!(
                f,
                "no reference data for mutaplasmid {mutaplasmid_id} on source type {source_type_id}",
            ),
            ProcessModuleError::Database(error) => write!(f, "database error: {error}"),
        }
    }
}

impl std::error::Error for ProcessModuleError {}

impl From<sqlx::Error> for ProcessModuleError {
    fn from(error: sqlx::Error) -> Self {
        ProcessModuleError::Database(error)
    }
}

#[derive(Debug)]
pub enum ImportModuleError {
    Esi(EsiError),
    Process(ProcessModuleError),
}

impl fmt::Display for ImportModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportModuleError::Esi(error) => write!(f, "ESI import failed: {error}"),
            ImportModuleError::Process(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ImportModuleError {}

/// Fetches a module's rolled dogma data from ESI and persists it, the
/// equivalent of the legacy `GetModuleJob` + `ImportModule` chain. An
/// already-known module is left untouched, like the legacy job.
pub async fn import_module(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    estimator: &Estimator,
    type_id: i64,
    item_id: i64,
) -> Result<(), ImportModuleError> {
    let exists: Option<i64> = sqlx::query_scalar("select id from modules where id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| ImportModuleError::Process(ProcessModuleError::Database(error)))?;

    if exists.is_some() {
        return Ok(());
    }

    let item = esi
        .dynamic_item(type_id, item_id)
        .await
        .map_err(ImportModuleError::Esi)?;

    let dogma_item = DogmaItem {
        created_by: item.created_by,
        source_type_id: item.source_type_id,
        mutator_type_id: item.mutator_type_id,
        dogma_attributes: item
            .dogma_attributes
            .iter()
            .map(|attribute| DogmaAttribute {
                attribute_id: attribute.attribute_id,
                value: attribute.value,
            })
            .collect(),
    };

    process_module(pool, reference, estimator, type_id, item_id, &dogma_item)
        .await
        .map(|_| ())
        .map_err(ImportModuleError::Process)
}

/// Computes and persists a module: upserts the `modules` row and all its
/// `mutated_attributes`, in one transaction, then refreshes the module's
/// estimated value like the tail of the legacy `ProcessModule::handle`.
/// Returns the computed results.
pub async fn process_module(
    pool: &PgPool,
    reference: &ReferenceData,
    estimator: &Estimator,
    type_id: i64,
    item_id: i64,
    dogma_item: &DogmaItem,
) -> Result<Vec<AttributeMutationResult>, ProcessModuleError> {
    let context = reference
        .context(dogma_item.mutator_type_id, dogma_item.source_type_id)
        .ok_or(ProcessModuleError::UnknownCombination {
            mutaplasmid_id: dogma_item.mutator_type_id,
            source_type_id: dogma_item.source_type_id,
        })?;

    let results = calculate(&context, &dogma_item.dogma_attributes);
    let average = average_fraction(&results);

    let mut tx = pool.begin().await?;

    // The creator relation needs at least a stub character row, like the
    // legacy Character::insertById; the name gets filled by the character
    // name sync later.
    sqlx::query("insert into characters (id, name) values ($1, '') on conflict (id) do nothing")
        .bind(dogma_item.created_by)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "insert into modules (id, type_id, source_type_id, mutaplasmid_id, creator_id, average_fraction)
         values ($1, $2, $3, $4, $5, $6)
         on conflict (id) do update set
             type_id = excluded.type_id,
             source_type_id = excluded.source_type_id,
             mutaplasmid_id = excluded.mutaplasmid_id,
             creator_id = excluded.creator_id,
             average_fraction = excluded.average_fraction,
             updated_at = now()",
    )
    .bind(item_id)
    .bind(type_id)
    .bind(dogma_item.source_type_id)
    .bind(dogma_item.mutator_type_id)
    .bind(dogma_item.created_by)
    .bind(average)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into mutated_attributes
         (module_id, attribute_id, type_id, value, base_value, fraction, fraction_type,
          fraction_absolute, bar, is_virtual)
         select $1, * from unnest($2::bigint[], $3::bigint[], $4::float8[], $5::float8[],
                                  $6::float8[], $7::float8[], $8::float8[], $9::smallint[],
                                  $10::boolean[])
         on conflict (module_id, attribute_id) do update set
             type_id = excluded.type_id,
             value = excluded.value,
             base_value = excluded.base_value,
             fraction = excluded.fraction,
             fraction_type = excluded.fraction_type,
             fraction_absolute = excluded.fraction_absolute,
             bar = excluded.bar,
             is_virtual = excluded.is_virtual",
    )
    .bind(item_id)
    .bind(results.iter().map(|result| result.attribute_id).collect::<Vec<_>>())
    .bind(results.iter().map(|_| type_id).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.value).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.base_value).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.fraction).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.fraction_type).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.fraction_absolute).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.bar.as_int() as i16).collect::<Vec<_>>())
    .bind(results.iter().map(|result| result.is_virtual).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Legacy estimates outside the transaction, after the module is saved.
    // An unreachable AI server is swallowed inside the estimate; only
    // database errors propagate, like the legacy action.
    crate::estimator::estimate_module_value(pool, estimator, item_id).await?;

    Ok(results)
}
