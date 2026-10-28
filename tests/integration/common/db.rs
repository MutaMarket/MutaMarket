//! A fresh database for every test, cloned from a template. The template
//! holds the migrations and the reference fixtures, so a clone is ready to
//! use and costs milliseconds; tests no longer share rows and run in
//! parallel.
//!
//! The template is named after a hash of the migrations and the fixtures,
//! so changing either builds a new one. Whichever process gets there first
//! builds it under an advisory lock. A passing test's database is dropped;
//! a failed one is kept to look at and swept by the next run once its
//! process is gone.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use tokio::sync::{OnceCell, Semaphore, SemaphorePermit};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// The checked-in reference dumps every template is seeded with.
const REFERENCE_FIXTURES: &str = "tests/fixtures/reference";

/// Prefix of every database this module creates; nothing else is swept.
const PREFIX: &str = "mmtest_";

/// Test databases alive at once. Each pool holds up to
/// [`POOL_CONNECTIONS`], and Postgres allows a hundred by default.
const LIVE_DATABASES: usize = 24;
const POOL_CONNECTIONS: u32 = 3;

/// Any constant works, as long as every test process agrees on it.
const TEMPLATE_LOCK: i64 = 4_206_100_051;

static TEMPLATE: OnceCell<String> = OnceCell::const_new();
static LIVE: Semaphore = Semaphore::const_new(LIVE_DATABASES);
static NEXT: AtomicU32 = AtomicU32::new(0);

/// The reference fixtures, parsed once per process.
static REFERENCE: LazyLock<ReferenceTables> = LazyLock::new(|| {
    ReferenceTables::load_from_dir(Path::new(REFERENCE_FIXTURES)).expect("reference dumps parse")
});

/// The reference tables every test database already holds.
pub fn reference_tables() -> ReferenceTables {
    REFERENCE.clone()
}

/// [`reference_tables`] in the form the router and the ingest take.
pub fn reference_data() -> ReferenceData {
    ReferenceData::from_tables(reference_tables())
}

/// The server behind `TEST_DATABASE_URL`, reached through its maintenance
/// database.
fn server_options() -> PgConnectOptions {
    PgConnectOptions::from_str(&mutamarket::db::test_database_url())
        .expect("TEST_DATABASE_URL parses")
        .database("postgres")
}

async fn server() -> PgConnection {
    PgConnection::connect_with(&server_options())
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`")
}

/// The template's name: the migrations by version and checksum, plus the
/// reference fixtures' contents.
fn template_name() -> String {
    let mut hasher = DefaultHasher::new();
    for migration in MIGRATOR.iter() {
        migration.version.hash(&mut hasher);
        migration.checksum.hash(&mut hasher);
    }
    let mut fixtures: Vec<_> = std::fs::read_dir(REFERENCE_FIXTURES)
        .expect("reference fixtures")
        .map(|entry| entry.expect("fixture entry").path())
        .collect();
    fixtures.sort();
    for path in fixtures {
        path.file_name().hash(&mut hasher);
        std::fs::read(&path)
            .expect("read fixture")
            .hash(&mut hasher);
    }
    // DefaultHasher may change between toolchains; that only costs one
    // rebuild of the template.
    format!("{PREFIX}template_{:016x}", hasher.finish())
}

/// Builds the template if it is missing and sweeps what earlier runs left.
async fn ensure_template() -> String {
    let name = template_name();
    let mut conn = server().await;
    sqlx::query("select pg_advisory_lock($1)")
        .bind(TEMPLATE_LOCK)
        .execute(&mut conn)
        .await
        .expect("take the template lock");

    let exists: bool =
        sqlx::query_scalar("select exists(select 1 from pg_database where datname = $1)")
            .bind(&name)
            .fetch_one(&mut conn)
            .await
            .expect("look for the template");
    if !exists {
        // Built under another name and renamed when done, so a run that
        // dies half way leaves nothing that looks finished.
        let building = format!("{name}_building");
        drop_database(&mut conn, &building).await;
        conn.execute(AssertSqlSafe(format!("create database \"{building}\"")))
            .await
            .expect("create the template");

        let pool = PgPoolOptions::new()
            .max_connections(POOL_CONNECTIONS)
            .connect_with(server_options().database(&building))
            .await
            .expect("connect to the template");
        MIGRATOR.run(&pool).await.expect("migrate the template");
        seed_reference(&pool, &REFERENCE)
            .await
            .expect("seed the template's reference tables");
        pool.close().await;

        conn.execute(AssertSqlSafe(format!(
            "alter database \"{building}\" rename to \"{name}\""
        )))
        .await
        .expect("finish the template");
    }
    sweep(&mut conn, &name).await;

    sqlx::query("select pg_advisory_unlock($1)")
        .bind(TEMPLATE_LOCK)
        .execute(&mut conn)
        .await
        .expect("release the template lock");
    conn.close().await.ok();
    name
}

/// Drops older templates and the databases of test processes that are no
/// longer running (where a process can be looked up by pid).
async fn sweep(conn: &mut PgConnection, template: &str) {
    if !Path::new("/proc/self").exists() {
        return;
    }
    let names: Vec<String> =
        sqlx::query_scalar("select datname from pg_database where starts_with(datname, $1)")
            .bind(PREFIX)
            .fetch_all(&mut *conn)
            .await
            .expect("list test databases");
    for name in names {
        let stale = match name
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.split_once('_'))
        {
            Some(("template", _)) => name != template,
            Some((pid, _)) => pid
                .parse::<u32>()
                .is_ok_and(|pid| !Path::new(&format!("/proc/{pid}")).exists()),
            None => false,
        };
        if stale {
            drop_database(conn, &name).await;
        }
    }
}

async fn drop_database(conn: &mut PgConnection, name: &str) {
    conn.execute(AssertSqlSafe(format!(
        "drop database if exists \"{name}\" with (force)"
    )))
    .await
    .expect("drop a test database");
}

/// Keeps a test's database for as long as the test runs and drops it when
/// the test passes. Bind it to a name (`_db`, not `_`), or it goes at once.
pub struct TestDb {
    name: String,
    _live: SemaphorePermit<'static>,
}

/// A test's own database, migrated and seeded with the reference tables,
/// and the guard that keeps it.
pub async fn fresh() -> (PgPool, TestDb) {
    let template = TEMPLATE.get_or_init(ensure_template).await;
    let live = LIVE.acquire().await.expect("the semaphore is never closed");

    let name = format!(
        "{PREFIX}{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut conn = server().await;
    conn.execute(AssertSqlSafe(format!(
        "create database \"{name}\" template \"{template}\""
    )))
    .await
    .expect("clone the template");
    conn.close().await.ok();

    let pool = PgPoolOptions::new()
        .max_connections(POOL_CONNECTIONS)
        .idle_timeout(Some(Duration::from_secs(1)))
        .connect_lazy_with(server_options().database(&name));
    (pool, TestDb { name, _live: live })
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // A failed test's database stays to be looked at; the next run
        // sweeps it.
        if std::thread::panicking() {
            return;
        }
        let name = std::mem::take(&mut self.name);
        // Drop runs inside the test's runtime, which cannot block, so the
        // drop happens on a thread of its own. Joining it keeps the permit
        // until the database is really gone.
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime to drop the database on")
                .block_on(async {
                    let mut conn = server().await;
                    drop_database(&mut conn, &name).await;
                    conn.close().await.ok();
                });
        })
        .join()
        .ok();
    }
}
