//! LESSON 03 — Persistence: migrations, queries, row mapping, transactions, pools, N+1
//! Read first: CONCEPTS.md, "Lesson 03". Start after: lesson 01 and ecosystem E01.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a function starts a transaction, runs
//! one `UPDATE`, and returns early with `?` before `commit`. What does the database
//! contain afterwards, and which line of code caused that?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! Everything runs on SQLite, in memory or in a temporary file, so no database
//! server is needed. The SQL is checked when it runs, not when it compiles.
//! EXPLAIN afterward: Why does `move_units` need a transaction even though each
//! `UPDATE` is atomic on its own? What happens to a request that needs a connection
//! when the pool is empty, and what should the client see? Why is the listing with
//! one query per shelf a problem only at scale?
//! JUDGMENT: sqlx gives you plain SQL, async I/O and a pool, and nothing else. That
//! is enough for most services and keeps the SQL visible in code review. Its
//! `query!` macros check SQL against a live database at compile time; this lesson
//! uses the runtime-checked functions so that it builds offline. diesel is the
//! mature choice when you want queries built and checked in Rust types; sea-orm
//! adds an ORM on top of sqlx. rusqlite is simpler if SQLite is your only target
//! and blocking calls are acceptable (call it from `spawn_blocking`). SQLite itself
//! suits single-node services with one writer at a time; move to PostgreSQL when
//! you need concurrent writers or more than one application server.
//!
//! The schema is in `migrations/l03/`. Read both files before you start.

use crate::support::CountingDb;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqliteExecutor, SqlitePool};
use std::str::FromStr;
use std::time::Duration;

/// The migrations of this lesson, read from `migrations/l03/` at compile time and
/// embedded in the binary. Supplied.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/l03");

/// How long a caller waits for a free connection before giving up.
pub const ACQUIRE_TIMEOUT: Duration = Duration::from_millis(250);

/// One row of `item`. `sqlx::FromRow` maps columns to fields by name.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Item {
    pub id: i64,
    pub shelf_id: i64,
    pub label: String,
    pub qty: i64,
}

/// One shelf with the number of item rows on it and the sum of their `qty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfTotal {
    pub name: String,
    pub items: i64,
    pub units: i64,
}

/// One shelf with all of its items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfListing {
    pub name: String,
    pub items: Vec<Item>,
}

/// What callers of this module can react to. `Db` is everything else; its
/// details are for logs, never for clients.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("a row with this unique value exists already")]
    Duplicate,
    #[error("the row that this one refers to does not exist")]
    MissingReference,
    #[error("no such row")]
    NotFound,
    #[error("not enough units")]
    Insufficient,
    #[error("the database is busy, try again")]
    Busy,
    #[error("database error")]
    Db(#[source] sqlx::Error),
}

/// A. Open a pool for `url` and bring the schema up to date.
///
/// - Create the database file if it is missing.
/// - Enforce foreign keys, and use the write-ahead log (`journal_mode = WAL`).
/// - At most `max_connections` connections; a caller waits at most
///   [`ACQUIRE_TIMEOUT`] for one.
/// - Run [`MIGRATOR`] before returning, so no caller ever sees an old schema.
///   Running it against an up-to-date database changes nothing.
pub async fn open_pool(url: &str, max_connections: u32) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// B. Classify database errors once, so that `?` does the right thing everywhere:
///
/// - the pool had no free connection in time → `Busy`
/// - `RowNotFound` → `NotFound`
/// - a UNIQUE constraint failed → `Duplicate`
/// - a FOREIGN KEY constraint failed → `MissingReference`
/// - a CHECK constraint failed → `Insufficient` (the only CHECK in this schema is
///   `qty >= 0`)
/// - anything else → `Db`, keeping the original error as the source
///
/// Do not compare message texts or numeric codes; sqlx classifies constraint
/// errors for you.
impl From<sqlx::Error> for StoreError {
    fn from(error: sqlx::Error) -> StoreError {
        match &error {
            sqlx::Error::PoolTimedOut => StoreError::Busy,
            sqlx::Error::RowNotFound => StoreError::NotFound,
            sqlx::Error::Database(db) if db.is_unique_violation() => StoreError::Duplicate,
            sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
                StoreError::MissingReference
            }
            sqlx::Error::Database(db) if db.is_check_violation() => StoreError::Insufficient,
            _ => StoreError::Db(error),
        }
    }
}

/// C. Insert a shelf and return its id. `db` may be a pool, a connection, or a
/// transaction; that is what `impl SqliteExecutor` buys you.
/// Values always travel as bind parameters, never inside the SQL text.
pub async fn add_shelf(db: impl SqliteExecutor<'_>, name: &str) -> Result<i64, StoreError> {
    let done = sqlx::query("INSERT INTO shelf (name) VALUES (?1)")
        .bind(name)
        .execute(db)
        .await?;
    Ok(done.last_insert_rowid())
}

/// C. Insert an item and return the stored row, read back in the same statement
/// (`RETURNING`).
pub async fn add_item(
    db: impl SqliteExecutor<'_>,
    shelf_id: i64,
    label: &str,
    qty: i64,
) -> Result<Item, StoreError> {
    let item = sqlx::query_as::<_, Item>(
        "INSERT INTO item (shelf_id, label, qty) VALUES (?1, ?2, ?3) \
         RETURNING id, shelf_id, label, qty",
    )
    .bind(shelf_id)
    .bind(label)
    .bind(qty)
    .fetch_one(db)
    .await?;
    Ok(item)
}

/// C. Every item whose label is exactly `label`, ordered by id.
pub async fn items_labelled(
    db: impl SqliteExecutor<'_>,
    label: &str,
) -> Result<Vec<Item>, StoreError> {
    let items = sqlx::query_as::<_, Item>(
        "SELECT id, shelf_id, label, qty FROM item WHERE label = ?1 ORDER BY id",
    )
    .bind(label)
    .fetch_all(db)
    .await?;
    Ok(items)
}

/// C. One [`ShelfTotal`] per shelf, ordered by name, including shelves without
/// items (0 items, 0 units). Use one aggregate query, and map each row by hand
/// with `Row::try_get`: `ShelfTotal` has no `FromRow` impl on purpose.
pub async fn shelf_totals(db: impl SqliteExecutor<'_>) -> Result<Vec<ShelfTotal>, StoreError> {
    let rows = sqlx::query(
        "SELECT shelf.name AS name, count(item.id) AS items, coalesce(sum(item.qty), 0) AS units \
         FROM shelf LEFT JOIN item ON item.shelf_id = shelf.id \
         GROUP BY shelf.id ORDER BY shelf.name",
    )
    .fetch_all(db)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(ShelfTotal {
                name: row.try_get("name")?,
                items: row.try_get("items")?,
                units: row.try_get("units")?,
            })
        })
        .collect()
}

/// D. Move `units` from item `from` to item `to`, all or nothing.
///
/// - An unknown `from` or `to` gives `NotFound`.
/// - If `from` holds fewer than `units`, the result is `Insufficient`.
/// - On **any** error, including errors you did not expect, both rows are exactly
///   as they were before the call.
///
/// Use one transaction. Think about what happens to it on each early return.
pub async fn move_units(
    pool: &SqlitePool,
    from: i64,
    to: i64,
    units: u32,
) -> Result<(), StoreError> {
    let mut tx = pool.begin().await?;
    // The CHECK (qty >= 0) constraint refuses an overdraft; `From` turns that
    // error into `Insufficient`.
    let taken = sqlx::query("UPDATE item SET qty = qty - ?1 WHERE id = ?2")
        .bind(units)
        .bind(from)
        .execute(&mut *tx)
        .await?;
    if taken.rows_affected() == 0 {
        // Returning drops `tx`, and dropping an open transaction rolls it back.
        return Err(StoreError::NotFound);
    }
    let given = sqlx::query("UPDATE item SET qty = qty + ?1 WHERE id = ?2")
        .bind(units)
        .bind(to)
        .execute(&mut *tx)
        .await?;
    if given.rows_affected() == 0 {
        return Err(StoreError::NotFound);
    }
    tx.commit().await?;
    Ok(())
}

/// F. Every shelf with its items: shelves ordered by name, items ordered by id,
/// shelves without items included with an empty list.
///
/// The number of statements must not grow with the number of shelves: use at most
/// two. (Task E has no code of its own: it is the `e_` test, which passes once
/// tasks A and B are right.)
pub async fn shelf_listings(db: &CountingDb) -> Result<Vec<ShelfListing>, StoreError> {
    // One statement: a LEFT JOIN keeps empty shelves, with NULLs in the item
    // columns. The rows arrive grouped because of the ORDER BY.
    let rows = sqlx::query(
        "SELECT shelf.name AS shelf_name, item.id AS id, item.shelf_id AS shelf_id, \
                item.label AS label, item.qty AS qty \
         FROM shelf LEFT JOIN item ON item.shelf_id = shelf.id \
         ORDER BY shelf.name, item.id",
    )
    .fetch_all(db)
    .await?;
    let mut listings: Vec<ShelfListing> = Vec::new();
    for row in &rows {
        let name: String = row.try_get("shelf_name")?;
        if listings.last().is_none_or(|last| last.name != name) {
            listings.push(ShelfListing {
                name,
                items: Vec::new(),
            });
        }
        // `id` is NULL for a shelf without items.
        if let Some(id) = row.try_get::<Option<i64>, _>("id")? {
            let item = Item {
                id,
                shelf_id: row.try_get("shelf_id")?,
                label: row.try_get("label")?,
                qty: row.try_get("qty")?,
            };
            listings
                .last_mut()
                .expect("a listing was pushed above")
                .items
                .push(item);
        }
    }
    Ok(listings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{TempDb, deadline};
    use std::time::Instant;

    /// A private in-memory database with one connection.
    async fn memory() -> SqlitePool {
        open_pool("sqlite::memory:", 1)
            .await
            .expect("open_pool on an in-memory database")
    }

    async fn qty(pool: &SqlitePool, id: i64) -> i64 {
        sqlx::query_scalar("SELECT qty FROM item WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("item exists")
    }

    /// Shelves "bolts" (two items) and "glue" (one item), plus the empty "attic".
    async fn seeded() -> (SqlitePool, [Item; 3]) {
        let pool = memory().await;
        let bolts = add_shelf(&pool, "bolts").await.unwrap();
        let glue = add_shelf(&pool, "glue").await.unwrap();
        add_shelf(&pool, "attic").await.unwrap();
        let m4 = add_item(&pool, bolts, "M4x10", 40).await.unwrap();
        let m6 = add_item(&pool, bolts, "M6x20", 12).await.unwrap();
        let epoxy = add_item(&pool, glue, "epoxy", 3).await.unwrap();
        (pool, [m4, m6, epoxy])
    }

    #[tokio::test]
    async fn a_open_pool_creates_the_file_and_migrates() {
        crate::support::deadline(
            120,
            "the test `a_open_pool_creates_the_file_and_migrates`",
            async {
                let db = TempDb::new("l03-open");
                assert!(!db.path().exists());
                let pool = open_pool(&db.url(), 2).await.expect("open_pool");
                assert!(db.path().exists(), "the database file must be created");

                let tables: Vec<String> = sqlx::query_scalar(
                    "SELECT name FROM sqlite_master WHERE type IN ('table', 'index') \
             AND name IN ('shelf', 'item', 'item_by_shelf') ORDER BY name",
                )
                .fetch_all(&pool)
                .await
                .unwrap();
                assert_eq!(
                    tables,
                    ["item", "item_by_shelf", "shelf"],
                    "both migrations ran"
                );

                let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(mode, "wal", "use the write-ahead log for file databases");
                let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(foreign_keys, 1, "foreign keys must be enforced");
                assert_eq!(pool.options().get_max_connections(), 2);
                assert_eq!(pool.options().get_acquire_timeout(), ACQUIRE_TIMEOUT);
                pool.close().await;
            },
        )
        .await
    }

    #[tokio::test]
    async fn a_open_pool_twice_keeps_the_data() {
        crate::support::deadline(120, "the test `a_open_pool_twice_keeps_the_data`", async {
            let db = TempDb::new("l03-reopen");
            let pool = open_pool(&db.url(), 1).await.unwrap();
            add_shelf(&pool, "bolts").await.unwrap();
            pool.close().await;

            let pool = open_pool(&db.url(), 1)
                .await
                .expect("opening an up-to-date database must succeed");
            let names: Vec<String> = sqlx::query_scalar("SELECT name FROM shelf")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert_eq!(
                names,
                ["bolts"],
                "migrations must not run twice or wipe data"
            );
            pool.close().await;
        })
        .await
    }

    #[tokio::test]
    async fn b_errors_are_classified() {
        crate::support::deadline(120, "the test `b_errors_are_classified`", async {
            let (pool, [m4, ..]) = seeded().await;
            let duplicate = add_shelf(&pool, "bolts").await.unwrap_err();
            assert!(matches!(duplicate, StoreError::Duplicate), "{duplicate:?}");
            let orphan = add_item(&pool, 999, "lost", 1).await.unwrap_err();
            assert!(matches!(orphan, StoreError::MissingReference), "{orphan:?}");
            let negative = add_item(&pool, m4.shelf_id, "debt", -1).await.unwrap_err();
            assert!(matches!(negative, StoreError::Insufficient), "{negative:?}");

            let not_found: StoreError = sqlx::Error::RowNotFound.into();
            assert!(matches!(not_found, StoreError::NotFound));
            let busy: StoreError = sqlx::Error::PoolTimedOut.into();
            assert!(matches!(busy, StoreError::Busy));

            let syntax = sqlx::query("SELEKT 1").execute(&pool).await.unwrap_err();
            let other: StoreError = syntax.into();
            assert!(matches!(other, StoreError::Db(_)), "{other:?}");
            assert_eq!(
                other.to_string(),
                "database error",
                "the Display text is safe to show; details stay in the source"
            );
            let source = std::error::Error::source(&other).expect("Db keeps the cause");
            assert!(source.to_string().contains("syntax error"), "{source}");
        })
        .await
    }

    #[tokio::test]
    async fn c_inserts_return_ids_and_rows() {
        crate::support::deadline(120, "the test `c_inserts_return_ids_and_rows`", async {
            let (pool, [m4, m6, epoxy]) = seeded().await;
            assert_eq!(
                m4,
                Item {
                    id: 1,
                    shelf_id: 1,
                    label: "M4x10".to_owned(),
                    qty: 40
                }
            );
            assert_eq!((m6.id, epoxy.id, epoxy.shelf_id), (2, 3, 2));
            let stored: Item =
                sqlx::query_as("SELECT id, shelf_id, label, qty FROM item WHERE id = 2")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(stored, m6, "RETURNING must give back the row as stored");
        })
        .await
    }

    #[tokio::test]
    async fn c_values_are_bound_not_pasted() {
        crate::support::deadline(120, "the test `c_values_are_bound_not_pasted`", async {
            let (pool, [m4, ..]) = seeded().await;
            let attack = "x' OR '1'='1";
            assert_eq!(
                items_labelled(&pool, attack).await.unwrap(),
                [],
                "a label is data: it must never be read as SQL"
            );
            let odd = add_item(
                &pool,
                m4.shelf_id,
                "3/8\" 'fine' thread; DROP TABLE item",
                5,
            )
            .await
            .expect("quotes and semicolons are ordinary characters in a bound value");
            let found = items_labelled(&pool, &odd.label).await.unwrap();
            assert_eq!(found, [odd]);
            assert_eq!(items_labelled(&pool, "M4x10").await.unwrap(), [m4]);
        })
        .await
    }

    #[tokio::test]
    async fn c_totals_include_empty_shelves() {
        crate::support::deadline(120, "the test `c_totals_include_empty_shelves`", async {
            let (pool, _) = seeded().await;
            let total = |name: &str, items, units| ShelfTotal {
                name: name.to_owned(),
                items,
                units,
            };
            assert_eq!(
                shelf_totals(&pool).await.unwrap(),
                [
                    total("attic", 0, 0),
                    total("bolts", 2, 52),
                    total("glue", 1, 3)
                ]
            );
        })
        .await
    }

    #[tokio::test]
    async fn c_functions_accept_a_transaction() {
        crate::support::deadline(120, "the test `c_functions_accept_a_transaction`", async {
            let pool = memory().await;
            let mut tx = pool.begin().await.unwrap();
            let shelf = add_shelf(&mut *tx, "temp").await.unwrap();
            add_item(&mut *tx, shelf, "widget", 1).await.unwrap();
            assert_eq!(items_labelled(&mut *tx, "widget").await.unwrap().len(), 1);
            tx.rollback().await.unwrap();
            assert_eq!(
                shelf_totals(&pool).await.unwrap(),
                [],
                "work done through the transaction disappears on rollback"
            );
        })
        .await
    }

    #[tokio::test]
    async fn d_move_transfers_units() {
        crate::support::deadline(120, "the test `d_move_transfers_units`", async {
            let (pool, [m4, m6, _]) = seeded().await;
            move_units(&pool, m4.id, m6.id, 15).await.unwrap();
            assert_eq!((qty(&pool, m4.id).await, qty(&pool, m6.id).await), (25, 27));
            move_units(&pool, m4.id, m6.id, 25).await.unwrap();
            assert_eq!((qty(&pool, m4.id).await, qty(&pool, m6.id).await), (0, 52));
        })
        .await
    }

    #[tokio::test]
    async fn d_failed_moves_change_nothing() {
        crate::support::deadline(120, "the test `d_failed_moves_change_nothing`", async {
            let (pool, [m4, m6, _]) = seeded().await;

            let short = move_units(&pool, m6.id, m4.id, 13).await.unwrap_err();
            assert!(matches!(short, StoreError::Insufficient), "{short:?}");
            assert_eq!(
                (qty(&pool, m4.id).await, qty(&pool, m6.id).await),
                (40, 12),
                "an overdraft must leave BOTH rows unchanged"
            );

            let no_target = move_units(&pool, m4.id, 999, 5).await.unwrap_err();
            assert!(matches!(no_target, StoreError::NotFound), "{no_target:?}");
            assert_eq!(
                qty(&pool, m4.id).await,
                40,
                "units taken from `from` must come back when `to` does not exist"
            );

            let no_source = move_units(&pool, 999, m4.id, 5).await.unwrap_err();
            assert!(matches!(no_source, StoreError::NotFound), "{no_source:?}");
            assert_eq!(qty(&pool, m4.id).await, 40);
        })
        .await
    }

    #[tokio::test]
    async fn d_unexpected_errors_roll_back_too() {
        crate::support::deadline(120, "the test `d_unexpected_errors_roll_back_too`", async {
            let (pool, [m4, m6, epoxy]) = seeded().await;
            // A failure that no validation in Rust can foresee: the database refuses
            // any update of the row labelled "epoxy".
            sqlx::query(
                "CREATE TRIGGER jammed BEFORE UPDATE ON item WHEN NEW.label = 'epoxy' \
             BEGIN SELECT RAISE(ABORT, 'the drawer is jammed'); END",
            )
            .execute(&pool)
            .await
            .unwrap();

            let into_jammed = move_units(&pool, m4.id, epoxy.id, 5).await.unwrap_err();
            assert!(matches!(into_jammed, StoreError::Db(_)), "{into_jammed:?}");
            assert_eq!(
                (qty(&pool, m4.id).await, qty(&pool, epoxy.id).await),
                (40, 3),
                "the first UPDATE must be undone when the second one fails"
            );

            let from_jammed = move_units(&pool, epoxy.id, m6.id, 1).await.unwrap_err();
            assert!(matches!(from_jammed, StoreError::Db(_)), "{from_jammed:?}");
            assert_eq!(
                (qty(&pool, epoxy.id).await, qty(&pool, m6.id).await),
                (3, 12),
                "whichever UPDATE runs first must be undone when the other one fails"
            );

            // The pool has one connection. If a failed move left its transaction
            // open, this call could not get a connection.
            move_units(&pool, m4.id, m6.id, 1)
                .await
                .expect("the pool must be usable after failed moves");
        })
        .await
    }

    #[tokio::test]
    async fn e_an_exhausted_pool_reports_busy_instead_of_hanging() {
        crate::support::deadline(
            120,
            "the test `e_an_exhausted_pool_reports_busy_instead_of_hanging`",
            async {
                let db = TempDb::new("l03-busy");
                let pool = open_pool(&db.url(), 1).await.unwrap();
                // Someone else holds the only connection, for example a slow transaction.
                let held = pool.acquire().await.unwrap();

                let start = Instant::now();
                let result =
                    deadline(10, "add_shelf on an exhausted pool", add_shelf(&pool, "x")).await;
                let error = result.expect_err("there is no free connection");
                assert!(
                    matches!(error, StoreError::Busy),
                    "pool exhaustion is `Busy`, got {error:?}"
                );
                assert!(
                    start.elapsed() >= ACQUIRE_TIMEOUT,
                    "the caller should wait for ACQUIRE_TIMEOUT before giving up"
                );

                drop(held);
                add_shelf(&pool, "x")
                    .await
                    .expect("with the connection back, the same call succeeds");
                pool.close().await;
            },
        )
        .await
    }

    #[tokio::test]
    async fn f_listings_are_grouped_and_ordered() {
        crate::support::deadline(
            120,
            "the test `f_listings_are_grouped_and_ordered`",
            async {
                let (pool, [m4, m6, epoxy]) = seeded().await;
                let db = CountingDb::new(pool);
                let listings = shelf_listings(&db).await.unwrap();
                let expected = [
                    ShelfListing {
                        name: "attic".to_owned(),
                        items: vec![],
                    },
                    ShelfListing {
                        name: "bolts".to_owned(),
                        items: vec![m4, m6],
                    },
                    ShelfListing {
                        name: "glue".to_owned(),
                        items: vec![epoxy],
                    },
                ];
                assert_eq!(listings, expected);
            },
        )
        .await
    }

    #[tokio::test]
    async fn f_statement_count_does_not_grow_with_the_data() {
        crate::support::deadline(
            120,
            "the test `f_statement_count_does_not_grow_with_the_data`",
            async {
                let pool = memory().await;
                for shelf in 0..40 {
                    let id = add_shelf(&pool, &format!("shelf-{shelf:02}"))
                        .await
                        .unwrap();
                    for item in 0..(shelf % 4) {
                        add_item(&pool, id, &format!("item-{item}"), item)
                            .await
                            .unwrap();
                    }
                }
                let db = CountingDb::new(pool);
                let listings = shelf_listings(&db).await.unwrap();
                assert_eq!(listings.len(), 40);
                assert_eq!(listings[7].name, "shelf-07");
                assert_eq!(listings[7].items.len(), 3);
                assert_eq!(listings.iter().map(|l| l.items.len()).sum::<usize>(), 60);
                assert!(
                    db.statements() >= 1,
                    "run the statements through `db`, not through `db.pool()`, so they are counted"
                );
                assert!(
                    db.statements() <= 2,
                    "40 shelves took {} statements: that is one query per shelf (N+1). \
             Fetch all items in one statement and group them in memory.",
                    db.statements()
                );
            },
        )
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            // P1. A transaction inserts a shelf and is dropped without `commit`.
            // How many shelves are there afterwards?
            let pool = memory().await;
            {
                let mut tx = pool.begin().await.unwrap();
                add_shelf(&mut *tx, "ghost").await.unwrap();
            }
            let shelves: i64 = sqlx::query_scalar("SELECT count(*) FROM shelf")
                .fetch_one(&pool)
                .await
                .unwrap();
            let p1: i64 = 0;
            assert_eq!(p1, shelves, "P1: dropping a transaction");

            // P2. `shelf.id` is `INTEGER PRIMARY KEY` without AUTOINCREMENT. Shelves a
            // and b are inserted, b is deleted, c is inserted. What is c's id?
            add_shelf(&pool, "a").await.unwrap();
            let b = add_shelf(&pool, "b").await.unwrap();
            sqlx::query("DELETE FROM shelf WHERE id = ?1")
                .bind(b)
                .execute(&pool)
                .await
                .unwrap();
            let c = add_shelf(&pool, "c").await.unwrap();
            let p2: i64 = 2;
            assert_eq!(p2, c, "P2: the id after deleting the newest row");

            // P3. How many rows does sqlx's bookkeeping table hold after `open_pool`
            // has run against the same database file twice?
            let db = TempDb::new("l03-predict");
            open_pool(&db.url(), 1).await.unwrap().close().await;
            let pool = open_pool(&db.url(), 1).await.unwrap();
            let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .unwrap();
            pool.close().await;
            let p3: i64 = 2;
            assert_eq!(p3, applied, "P3: one row per migration file");

            // P4. Two pools are opened from the same text "sqlite::memory:". A shelf is
            // added through the first. How many shelves does the second pool see?
            let first = memory().await;
            let second = memory().await;
            add_shelf(&first, "only here?").await.unwrap();
            let seen: i64 = sqlx::query_scalar("SELECT count(*) FROM shelf")
                .fetch_one(&second)
                .await
                .unwrap();
            let p4: i64 = 0;
            assert_eq!(p4, seen, "P4: every in-memory pool is its own database");
        })
        .await
    }
}
