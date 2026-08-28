//! The module search query, ported from the legacy `QueryService` and the
//! `ModuleBuilder` filter scopes: filter segments chained as URL path parts
//! (`type/{id-or-slug}/sort/{field}/{direction}/goldbar/...`).
//!
//! Asset-dependent options (`without-assets`, `with-personal-modules`,
//! ...) are recognized as delimiters but inert until their milestones
//! land; contract options are live.

use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::mutation::reference::ReferenceData;

/// `attributes.metaLevel`, used by the meta-level filter like the legacy
/// `Attribute::MetaLevel` constant.
const META_LEVEL_ATTRIBUTE: i64 = 633;

/// Estimator sample size under which a type still "needs training", the
/// legacy `whereNeedsTraining` default minimum data count.
const NEEDS_TRAINING_DEFAULT_MINIMUM: i64 = 50;

/// Every legacy query option keyword; unknown segments are ignored, these
/// delimit option arguments.
const OPTION_KEYWORDS: [&str; 24] = [
    "page",
    "type",
    "meta-group",
    "meta-level",
    "auction",
    "item-exchange",
    "contracts-only",
    "no-multi-item-contracts",
    "goldbar",
    "brownbar",
    "diamondbar",
    "attributes",
    "contract-price",
    "estimated-value",
    "with-personal-modules",
    "sort",
    "without-contracts",
    "without-fitted",
    "without-other-items",
    "without-assets",
    "created",
    "search",
    "needs-training",
    "in-jita",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    pub page: i64,
    pub type_filter: Option<TypeFilter>,
    pub meta_group_id: Option<i64>,
    pub meta_level: Option<f64>,
    pub attributes: Vec<AttributeFilter>,
    pub sort: Option<Sort>,
    pub value: Option<Bounds>,
    /// Contract price bounds: a single number is a maximum, like the
    /// legacy wherePrice scope.
    pub price: Option<Bounds>,
    /// `auction` or `item_exchange` when filtered.
    pub contract_type: Option<&'static str>,
    pub only_contracts: bool,
    pub no_multi_item_contracts: bool,
    pub without_other_items: bool,
    pub with_goldbar: bool,
    pub with_brownbar: bool,
    pub with_diamondbar: bool,
    /// Character pages: modules the character created instead of their
    /// public listings (the legacy `created` option).
    pub created: bool,
    /// Personal page: exclude modules currently fitted to a ship.
    pub without_fitted: bool,
    /// Personal page: exclude modules present in the user's assets.
    pub without_assets: bool,
    /// Moderator review: only types whose estimator holds fewer than
    /// this many training samples (the legacy `needs-training` option).
    pub needs_training: Option<i64>,
}

/// Which modules a listing shows: the for-sale set of the legacy module
/// browser (modules with a live contract), everything (the all-modules
/// page), or the recorded sales (the premium historic-sales page:
/// modules with a training row, sorted and priced by the historic
/// contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    ForSale,
    All,
    Historic,
}

/// Page-level narrowing on top of the query grammar: whose modules a
/// listing shows (the legacy per-page builder scopes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The character's public listings, the legacy
    /// `whereVisibleByCharacter` (their public ownerships).
    Character(i64),
    /// The user's modules inside one asset location (the legacy
    /// `inLocation` ancestorsAndSelf walk, taken downward from the
    /// location).
    InLocation { location_id: i64, user_id: i64 },
    /// Modules the character created (the character page's created
    /// radio).
    CreatedBy(i64),
    /// A collection's member modules.
    Collection(i64),
    /// Modules owned by any of the user's characters (personal page);
    /// carries the user id so the fitted/assets options can scope to
    /// the same account, like the legacy request-user scopes.
    OwnedByUser(i64),
    /// Modules the character has published (the sell page: the legacy
    /// whereRelation('publicAssets', character_id)).
    PublishedBy(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeFilter {
    pub id: i64,
    pub name: String,
}

/// A rolled-value filter with its bounds already resolved by roll
/// direction: a single number is a minimum where high is good, otherwise a
/// maximum, like the legacy getMinMax.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeFilter {
    pub attribute_id: i64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub lower: f64,
    pub upper: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKind {
    Price,
    Value,
    Fraction,
    Attribute(i64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sort {
    pub kind: SortKind,
    pub descending: bool,
}

#[derive(Debug)]
pub enum SearchError {
    /// Type missing or unresolvable; the legacy API answers 404.
    TypeNotFound,
    /// Invalid option arguments; the legacy aborts with 400.
    Invalid(String),
    Db(sqlx::Error),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::TypeNotFound => write!(f, "Please provide a valid type."),
            SearchError::Invalid(message) => write!(f, "{message}"),
            SearchError::Db(error) => write!(f, "database error: {error}"),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<sqlx::Error> for SearchError {
    fn from(error: sqlx::Error) -> Self {
        SearchError::Db(error)
    }
}

/// Parses a filter query path into a validated search, resolving types and
/// attributes against the database like the legacy `QueryService::get`.
pub async fn parse(
    pool: &PgPool,
    reference: &ReferenceData,
    query: &str,
) -> Result<Search, SearchError> {
    let segments: Vec<&str> = query.split('/').filter(|segment| !segment.is_empty()).collect();

    let mut search = Search {
        page: 1,
        type_filter: None,
        meta_group_id: None,
        meta_level: None,
        attributes: Vec::new(),
        sort: None,
        value: None,
        price: None,
        contract_type: None,
        only_contracts: false,
        no_multi_item_contracts: false,
        created: false,
        without_fitted: false,
        without_assets: false,
        needs_training: None,
        without_other_items: false,
        with_goldbar: false,
        with_brownbar: false,
        with_diamondbar: false,
    };

    // Raw option args collected first; attribute-dependent resolution below
    // needs the type to be known.
    let mut raw_attributes: Vec<String> = Vec::new();
    let mut raw_sort: Vec<String> = Vec::new();

    let mut index = 0;
    while index < segments.len() {
        let segment = segments[index];
        let args_start = index + 1;
        let args_end = (args_start..segments.len())
            .find(|&i| OPTION_KEYWORDS.contains(&segments[i]))
            .unwrap_or(segments.len());
        let args = &segments[args_start..args_end];

        match segment {
            "page" => {
                search.page = args
                    .first()
                    .and_then(|arg| arg.parse().ok())
                    .unwrap_or(1);
            }
            "type" => {
                let Some(needle) = args.first() else {
                    return Err(SearchError::TypeNotFound);
                };
                search.type_filter = Some(resolve_type(pool, needle).await?);
            }
            "meta-group" => {
                search.meta_group_id = Some(resolve_meta_group(args.first().copied())?);
            }
            "meta-level" => {
                let level = args
                    .first()
                    .and_then(|arg| arg.parse::<f64>().ok())
                    .ok_or_else(|| {
                        SearchError::Invalid("You provided an invalid meta level".to_owned())
                    })?;
                search.meta_level = Some(level);
            }
            "auction" => search.contract_type = Some("auction"),
            "item-exchange" => search.contract_type = Some("item_exchange"),
            "contracts-only" => search.only_contracts = true,
            "no-multi-item-contracts" => search.no_multi_item_contracts = true,
            "without-other-items" => search.without_other_items = true,
            "contract-price" => {
                search.price = args.first().and_then(|arg| match_numbers(arg)).map(
                    |(lower, upper)| Bounds { lower, upper },
                );
            }
            "goldbar" => search.with_goldbar = true,
            "created" => search.created = true,
            "without-fitted" => search.without_fitted = true,
            "without-assets" => search.without_assets = true,
            "brownbar" => search.with_brownbar = true,
            "diamondbar" => search.with_diamondbar = true,
            // The legacy `is_numeric($args[0]) ? (int) $args[0] : 50`:
            // a numeric argument is truncated to an integer, anything
            // else (a missing argument included) falls to the default.
            // Rust's f64 parser also accepts "nan"/"inf", which PHP's
            // is_numeric rejects, so non-finite parses fall through too.
            "needs-training" => {
                search.needs_training = Some(
                    args.first()
                        .and_then(|arg| arg.parse::<f64>().ok())
                        .filter(|minimum| minimum.is_finite())
                        .map(|minimum| minimum as i64)
                        .unwrap_or(NEEDS_TRAINING_DEFAULT_MINIMUM),
                );
            }
            "attributes" => {
                raw_attributes = args.iter().map(|s| (*s).to_owned()).collect();
            }
            "sort" => {
                raw_sort = args.iter().map(|s| (*s).to_owned()).collect();
            }
            "estimated-value" => {
                search.value = args.first().and_then(|arg| match_numbers(arg)).map(
                    |(lower, upper)| Bounds { lower, upper },
                );
            }
            // Recognized but inert until their milestones: contract and
            // asset options, personal filters, text search, training.
            _ => {}
        }

        index = if segment == "type"
            || OPTION_KEYWORDS.contains(&segment)
        {
            args_end.max(index + 1)
        } else {
            index + 1
        };
    }

    if !raw_sort.is_empty() {
        search.sort = Some(resolve_sort(pool, &raw_sort).await?);
    }

    if !raw_attributes.is_empty() {
        let Some(type_filter) = &search.type_filter else {
            return Err(SearchError::Invalid(
                "Module type must be specified when filtering by attributes.".to_owned(),
            ));
        };
        search.attributes =
            resolve_attributes(pool, reference, type_filter.id, &raw_attributes).await?;
    }

    if matches!(search.sort, Some(Sort { kind: SortKind::Attribute(_), .. }))
        && search.type_filter.is_none()
    {
        return Err(SearchError::Invalid(
            "Module type must be specified when sorting by attribute.".to_owned(),
        ));
    }

    Ok(search)
}

/// The module ids matching a search, sorted, like the legacy index
/// query. The search's one-based `page/N` option sets the offset, like
/// the legacy paginator.
pub async fn module_ids(
    pool: &PgPool,
    search: &Search,
    visibility: Visibility,
    limit: i64,
) -> sqlx::Result<Vec<i64>> {
    module_ids_page(pool, search, visibility, limit, page_offset(search, limit)).await
}

/// The offset of the search's one-based page.
fn page_offset(search: &Search, limit: i64) -> i64 {
    (search.page - 1).max(0) * limit
}

/// Like [`module_ids`] narrowed to a page scope (character, collection,
/// personal ownership).
pub async fn scoped_module_ids(
    pool: &PgPool,
    search: &Search,
    scope: Scope,
    limit: i64,
) -> sqlx::Result<Vec<i64>> {
    module_ids_scoped_page(pool, search, Visibility::All, Some(scope), limit, page_offset(search, limit))
        .await
}

/// Like [`module_ids`] with an offset, backing the API's cursor pages.
pub async fn module_ids_page(
    pool: &PgPool,
    search: &Search,
    visibility: Visibility,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<i64>> {
    module_ids_scoped_page(pool, search, visibility, None, limit, offset).await
}

/// The `withCommonSearch` module conditions shared by the listing query
/// and the moderator review's modules `whereHas`: type, meta group, meta
/// level, bar, attribute and estimated-value filters. Conditions are
/// appended as `and ...` clauses against a `m` modules alias.
pub fn push_common_filters(builder: &mut QueryBuilder<'_, Postgres>, search: &Search) {
    if let Some(type_filter) = &search.type_filter {
        builder.push(" and m.type_id = ");
        builder.push_bind(type_filter.id);
    }

    if let Some(meta_group_id) = search.meta_group_id {
        builder.push(
            " and exists (select 1 from types st where st.id = m.source_type_id and st.meta_group_id = ",
        );
        builder.push_bind(meta_group_id);
        builder.push(")");
    }

    if let Some(meta_level) = search.meta_level {
        builder.push(
            " and exists (select 1 from type_attributes ta where ta.type_id = m.source_type_id
               and ta.attribute_id = ",
        );
        builder.push_bind(META_LEVEL_ATTRIBUTE);
        builder.push(" and ta.value = ");
        builder.push_bind(meta_level);
        builder.push(")");
    }

    for (bar, wanted) in [
        (1i16, search.with_goldbar),
        (-1i16, search.with_brownbar),
        (2i16, search.with_diamondbar),
    ] {
        if wanted {
            builder.push(
                " and exists (select 1 from mutated_attributes b where b.module_id = m.id and b.bar = ",
            );
            builder.push_bind(bar);
            builder.push(")");
        }
    }

    for filter in &search.attributes {
        builder.push(
            " and exists (select 1 from mutated_attributes f where f.module_id = m.id
               and f.attribute_id = ",
        );
        builder.push_bind(filter.attribute_id);
        if let Some(min) = filter.min {
            builder.push(" and f.value >= ");
            builder.push_bind(min);
        }
        if let Some(max) = filter.max {
            builder.push(" and f.value <= ");
            builder.push_bind(max);
        }
        builder.push(")");
    }

    // PHP truthiness in the legacy scope: a zero lower bound disables the
    // value filter entirely.
    if let Some(bounds) = search.value.filter(|bounds| bounds.lower != 0.0) {
        builder.push(" and m.estimated_value >= ");
        builder.push_bind(bounds.lower);
        if let Some(upper) = bounds.upper {
            builder.push(" and m.estimated_value <= ");
            builder.push_bind(upper);
        }
    }
}

async fn module_ids_scoped_page(
    pool: &PgPool,
    search: &Search,
    visibility: Visibility,
    scope: Option<Scope>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<i64>> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new("select m.id from modules m");

    // The historic-sales page: only modules with a recorded sale, the
    // legacy whereHas('trainingModule') plus its sort joins (module_id
    // is unique, so the inner join never duplicates rows).
    if visibility == Visibility::Historic {
        builder.push(" join training_modules sort_training on sort_training.module_id = m.id");
        if matches!(search.sort, Some(Sort { kind: SortKind::Price, .. })) {
            builder.push(
                " join historic_contracts sort_historic
                   on sort_historic.id = sort_training.historic_contract_id",
            );
        }
    }

    // Attribute sorting joins the sorted attribute; an inner join, so only
    // modules carrying the attribute appear, like the legacy scope.
    if let Some(Sort { kind: SortKind::Attribute(attribute_id), .. }) = search.sort {
        builder.push(
            " join mutated_attributes sort_attributes
               on sort_attributes.module_id = m.id and sort_attributes.attribute_id = ",
        );
        builder.push_bind(attribute_id);
        if let Some(type_filter) = &search.type_filter {
            builder.push(" and sort_attributes.type_id = ");
            builder.push_bind(type_filter.id);
        }
    }

    builder.push(" where true");

    match scope {
        Some(Scope::Character(character_id)) => {
            builder.push(
                " and exists (select 1 from public_module_ownerships o
                   where o.module_id = m.id and o.character_id = ",
            );
            builder.push_bind(character_id);
            builder.push(")");
        }
        Some(Scope::InLocation { location_id, user_id }) => {
            builder.push(
                " and m.id in (
                    with recursive under_location as (
                        select a.item_id, a.is_abyssal from assets a
                        join characters ch on ch.id = a.character_id
                        where a.location_id = ",
            );
            builder.push_bind(location_id);
            builder.push(" and ch.user_id = ");
            builder.push_bind(user_id);
            builder.push(
                " union
                        select a.item_id, a.is_abyssal from assets a
                        join characters ch on ch.id = a.character_id
                        join under_location u on a.location_id = u.item_id
                        where ch.user_id = ",
            );
            builder.push_bind(user_id);
            builder.push(") select item_id from under_location where is_abyssal)");
        }
        Some(Scope::CreatedBy(character_id)) => {
            builder.push(" and m.creator_id = ");
            builder.push_bind(character_id);
        }
        Some(Scope::Collection(collection_id)) => {
            builder.push(
                " and exists (select 1 from collection_modules cm
                   where cm.module_id = m.id and cm.collection_id = ",
            );
            builder.push_bind(collection_id);
            builder.push(")");
        }
        Some(Scope::OwnedByUser(user_id)) => {
            // Ownership like the personal page: an abyssal asset row or
            // an issued contract holding the module. Shaped as IN over a
            // union so the planner materializes the (small) owned set
            // and drives the sort from it — the equivalent OR of two
            // correlated EXISTS probed every module in sort order and
            // took seconds.
            builder.push(
                " and m.id in (
                    select a.item_id from assets a
                      join characters ch on ch.id = a.character_id
                     where a.is_abyssal and ch.user_id = ",
            );
            builder.push_bind(user_id);
            builder.push(
                " union
                    select ci.item_id from contract_items ci
                      join contracts ct on ct.id = ci.contract_id
                      join characters ch on ch.id = ct.issuer_id
                     where ch.user_id = ",
            );
            builder.push_bind(user_id);
            builder.push(")");
        }
        Some(Scope::PublishedBy(character_id)) => {
            builder.push(
                " and m.id in (select pa.module_id from public_assets pa
                     where pa.module_id is not null and pa.character_id = ",
            );
            builder.push_bind(character_id);
            builder.push(")");
        }
        None => {}
    }

    // The asset options are account-relative, so they only apply inside
    // the personal scope (like the legacy request-user scopes).
    if let Some(Scope::OwnedByUser(user_id)) = scope {
        if search.without_fitted {
            builder.push(
                " and not exists (select 1 from assets a
                   join characters ch on ch.id = a.character_id
                   where a.item_id = m.id and ch.user_id = ",
            );
            builder.push_bind(user_id);
            // The legacy LocationFlag::fittingFlags set.
            builder.push(
                " and (a.location_flag in ('DroneBay', 'FighterBay')
                       or a.location_flag ~ '^(Hi|Lo|Med|Rig)Slot[0-7]$'))",
            );
        }
        if search.without_assets {
            builder.push(
                " and not exists (select 1 from assets a
                   join characters ch on ch.id = a.character_id
                   where a.item_id = m.id and ch.user_id = ",
            );
            builder.push_bind(user_id);
            builder.push(")");
        }
    }

    // The legacy `visible` scope: a live contract or a published (public)
    // asset. `contracts-only` narrows it to contracts, like the legacy
    // index's `when(! only_contracts, orWhere(whereHasPublicAssets))`.
    if visibility == Visibility::ForSale {
        if search.only_contracts {
            builder.push(" and m.latest_contract_id is not null");
        } else {
            builder.push(
                " and (m.latest_contract_id is not null
                   or exists (select 1 from public_module_ownerships o where o.module_id = m.id))",
            );
        }
    }

    push_common_filters(&mut builder, search);

    if let Some(contract_type) = search.contract_type {
        builder.push(
            " and exists (select 1 from contracts fc where fc.id = m.latest_contract_id and fc.type = ",
        );
        builder.push_bind(contract_type);
        builder.push(")");
    }

    // The legacy single-item rule: exactly one abyssal module, nothing else.
    if search.no_multi_item_contracts {
        builder.push(
            " and exists (select 1 from contracts fc where fc.id = m.latest_contract_id
               and fc.abyssal_modules_count = 1 and fc.non_abyssal_modules_count = 0)",
        );
    }

    // The legacy without-other-items rule: no unrelated items, or exactly
    // one other item that is asked-for PLEX.
    if search.without_other_items {
        builder.push(
            " and exists (select 1 from contracts fc where fc.id = m.latest_contract_id
               and (fc.non_abyssal_modules_count = 0
                    or (fc.non_abyssal_modules_count = 1 and fc.plex_count > 0)))",
        );
    }

    // Contract price bounds, with the legacy quirks: a zero lower bound
    // disables the filter (PHP truthiness), and a single bound is a
    // maximum. The historic page bounds the recorded sale price instead
    // (legacy whereHistoricPrice).
    if let Some(bounds) = search.price.filter(|bounds| bounds.lower != 0.0) {
        if visibility == Visibility::Historic {
            builder.push(
                " and exists (select 1 from training_modules ftm
                   join historic_contracts fhc on fhc.id = ftm.historic_contract_id
                   where ftm.module_id = m.id and fhc.unified_price ",
            );
        } else {
            builder.push(
                " and exists (select 1 from contracts fc where fc.id = m.latest_contract_id
                   and fc.unified_price ",
            );
        }
        if let Some(upper) = bounds.upper {
            builder.push(" between ");
            builder.push_bind(bounds.lower);
            builder.push(" and ");
            builder.push_bind(upper);
        } else {
            builder.push(" <= ");
            builder.push_bind(bounds.lower);
        }
        builder.push(")");
    }

    // MySQL sorts nulls first ascending and last descending; make Postgres
    // match so estimated-value ordering behaves like legacy.
    let order = match search.sort {
        Some(Sort { kind: SortKind::Attribute(_), descending }) => {
            // No nulls clause: the column is NOT NULL, and the MySQL
            // parity wording forced a full sort instead of walking the
            // (type, attribute, value, module) index in order.
            let direction = if descending { "desc" } else { "asc" };
            format!(
                " order by sort_attributes.value {direction}, sort_attributes.module_id {direction}"
            )
        }
        Some(Sort { kind: SortKind::Fraction, descending }) => {
            let direction = if descending { "desc nulls last" } else { "asc nulls first" };
            format!(" order by m.average_fraction {direction}, m.id {direction}")
        }
        Some(Sort { kind: SortKind::Value, descending }) => {
            let direction = if descending { "desc nulls last" } else { "asc nulls first" };
            format!(" order by m.estimated_value {direction}, m.id {direction}")
        }
        Some(Sort { kind: SortKind::Price, descending }) if visibility == Visibility::Historic => {
            // The legacy orderByHistoricPrice: the recorded sale price.
            let direction = if descending { "desc nulls last" } else { "asc nulls first" };
            format!(" order by sort_historic.unified_price {direction}, m.id {direction}")
        }
        Some(Sort { kind: SortKind::Price, descending }) => {
            // The trigger-maintained denormalized price: the legacy
            // ordered the live contracts join, which sorted the whole
            // visible set per request; the copy on modules is indexed.
            let direction = if descending { "desc nulls last" } else { "asc nulls first" };
            format!(" order by m.latest_contract_price {direction}, m.id {direction}")
        }
        _ if visibility == Visibility::Historic => {
            // The legacy orderByTrainingIssuedAt default: newest sale
            // first.
            " order by sort_training.issued_at desc nulls last, m.id desc".to_owned()
        }
        _ => " order by m.id desc".to_owned(),
    };
    builder.push(&order);

    builder.push(" limit ");
    builder.push_bind(limit);
    builder.push(" offset ");
    builder.push_bind(offset);

    let rows = builder.build().fetch_all(pool).await?;
    Ok(rows.iter().map(|row| row.get("id")).collect())
}

/// Legacy type resolution: numeric id, exact name (slug with dashes as
/// spaces), then a dash-wildcard LIKE ordered by shortest name.
pub(crate) async fn resolve_type(pool: &PgPool, needle: &str) -> Result<TypeFilter, SearchError> {
    if let Ok(id) = needle.parse::<i64>() {
        let row = sqlx::query("select id, name from types where id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        return row
            .map(|row| TypeFilter { id: row.get("id"), name: row.get("name") })
            .ok_or(SearchError::TypeNotFound);
    }

    let exact_name = needle.replace('-', " ");
    let exact = sqlx::query("select id, name from types where lower(name) = lower($1)")
        .bind(&exact_name)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = exact {
        return Ok(TypeFilter { id: row.get("id"), name: row.get("name") });
    }

    let pattern = needle.replace('-', "%");
    let row = sqlx::query(
        "select id, name from types where name ilike $1
         order by length(name) asc, id asc limit 1",
    )
    .bind(&pattern)
    .fetch_optional(pool)
    .await?;

    row.map(|row| TypeFilter { id: row.get("id"), name: row.get("name") })
        .ok_or(SearchError::TypeNotFound)
}

fn resolve_meta_group(arg: Option<&str>) -> Result<i64, SearchError> {
    let arg = arg.unwrap_or_default();

    if let Ok(id) = arg.parse::<i64>() {
        return Ok(id);
    }

    match arg {
        "t1" => Ok(1),
        "t2" => Ok(2),
        "storyline" => Ok(3),
        "faction" => Ok(4),
        "officer" => Ok(5),
        "deadspace" => Ok(6),
        other => Err(SearchError::Invalid(format!(
            "You provided an invalid meta group: {other}",
        ))),
    }
}

async fn resolve_sort(pool: &PgPool, args: &[String]) -> Result<Sort, SearchError> {
    let sort_by = args.first().map(String::as_str).unwrap_or_default();
    let descending = args.get(1).map(String::as_str) == Some("desc");

    let kind = match sort_by {
        "price" => SortKind::Price,
        "value" => SortKind::Value,
        "fraction" => SortKind::Fraction,
        needle => SortKind::Attribute(attribute_id_by_id_or_name(pool, needle).await?),
    };

    Ok(Sort { kind, descending })
}

async fn resolve_attributes(
    pool: &PgPool,
    reference: &ReferenceData,
    type_id: i64,
    args: &[String],
) -> Result<Vec<AttributeFilter>, SearchError> {
    let mut filters = Vec::new();

    for pair in args.chunks(2) {
        let needle = &pair[0];
        let Some(value) = pair.get(1) else {
            continue;
        };

        let attribute_id = attribute_id_by_id_or_name(pool, needle).await?;

        let Some((lower, upper)) = match_numbers(value) else {
            return Err(SearchError::Invalid(format!(
                "You provided an invalid value for attribute: {needle}",
            )));
        };

        let high_is_good = reference
            .output_type_high_is_good(type_id, attribute_id)
            .unwrap_or(true);

        let (min, max) = if upper.is_some() {
            (Some(lower), upper)
        } else if high_is_good {
            (Some(lower), None)
        } else {
            (None, Some(lower))
        };

        filters.push(AttributeFilter { attribute_id, min, max });
    }

    Ok(filters)
}

async fn attribute_id_by_id_or_name(pool: &PgPool, needle: &str) -> Result<i64, SearchError> {
    let row = if let Ok(id) = needle.parse::<i64>() {
        sqlx::query("select id from attributes where id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
    } else {
        // The legacy query builder lowercases attribute names in URLs, so
        // names must match case-insensitively.
        sqlx::query("select id from attributes where lower(name) = lower($1)")
            .bind(needle)
            .fetch_optional(pool)
            .await?
    };

    row.map(|row| row.get("id"))
        .ok_or_else(|| SearchError::Invalid(format!("Unknown attribute: {needle}")))
}

/// The legacy matchNumbers pattern: a number, optionally followed by a dash
/// and a second number, e.g. `20-30`, `2.1`, `-5--2`.
fn match_numbers(value: &str) -> Option<(f64, Option<f64>)> {
    let bytes = value.as_bytes();
    let start = bytes.iter().position(|&b| b.is_ascii_digit() || b == b'-')?;

    let (first, rest) = take_number(&value[start..])?;

    let second = rest
        .strip_prefix('-')
        .and_then(|rest| take_number(rest))
        .map(|(second, _)| second);

    Some((first, second))
}

/// Parses a leading (possibly negative) decimal number, returning it and
/// the remaining text.
fn take_number(text: &str) -> Option<(f64, &str)> {
    let negative = text.starts_with('-');
    let digits = &text[usize::from(negative)..];

    let mut end = 0;
    let mut seen_dot = false;
    for (offset, c) in digits.char_indices() {
        if c.is_ascii_digit() {
            end = offset + 1;
        } else if c == '.' && !seen_dot && end > 0 {
            seen_dot = true;
        } else {
            break;
        }
    }

    if end == 0 {
        return None;
    }

    let end = end + usize::from(negative);
    text[..end].parse().ok().map(|number| (number, &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::match_numbers;

    #[test]
    fn number_ranges_parse_like_the_legacy_pattern() {
        assert_eq!(match_numbers("20-30"), Some((20.0, Some(30.0))));
        assert_eq!(match_numbers("2.1"), Some((2.1, None)));
        assert_eq!(match_numbers("-5--2"), Some((-5.0, Some(-2.0))));
        assert_eq!(match_numbers("0-500000000"), Some((0.0, Some(500000000.0))));
        assert_eq!(match_numbers("garbage"), None);
    }
}
