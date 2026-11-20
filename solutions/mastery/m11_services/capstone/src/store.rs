//! Persistence: one table, one write path, one read path.

use beacon_wire::Sample;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Opens the database, creating and migrating it as needed.
pub async fn open(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        // FULL: a committed transaction survives a crash of the machine, not
        // only of the process. An Ack is a promise, so pay for the fsync.
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Stores a batch in one transaction and returns how many samples were new.
/// Samples that are already present are skipped, which makes redelivery harmless.
pub async fn insert_batch(
    pool: &SqlitePool,
    agent: &str,
    metric: &str,
    samples: &[Sample],
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut inserted = 0;
    for sample in samples {
        // SQLite integers are signed 64-bit; a time beyond that is not a time.
        let ts_ms = i64::try_from(sample.ts_ms)
            .map_err(|_| sqlx::Error::Protocol("timestamp out of range".to_owned()))?;
        // `ON CONFLICT ... DO NOTHING` skips a row only when this key exists.
        // `INSERT OR IGNORE` would also skip a row that breaks a NOT NULL or
        // CHECK constraint, and the batch would be acknowledged without it.
        let done = sqlx::query(
            "INSERT INTO sample (agent, metric, ts_ms, value) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (agent, metric, ts_ms) DO NOTHING",
        )
        .bind(agent)
        .bind(metric)
        .bind(ts_ms)
        .bind(sample.value)
        .execute(&mut *tx)
        .await?;
        inserted += done.rows_affected();
    }
    tx.commit().await?;
    Ok(inserted)
}

/// Up to `limit` samples with `ts_ms > after`, oldest first.
pub async fn samples_after(
    pool: &SqlitePool,
    agent: &str,
    metric: &str,
    after: i64,
    limit: u32,
) -> Result<Vec<(i64, i64)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ts_ms, value FROM sample \
         WHERE agent = ?1 AND metric = ?2 AND ts_ms > ?3 ORDER BY ts_ms LIMIT ?4",
    )
    .bind(agent)
    .bind(metric)
    .bind(after)
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn redelivered_samples_are_stored_once() {
        let pool = open("sqlite::memory:").await.unwrap();
        let samples = [
            Sample {
                ts_ms: 10,
                value: 1,
            },
            Sample {
                ts_ms: 20,
                value: 2,
            },
        ];
        assert_eq!(insert_batch(&pool, "a", "m", &samples).await.unwrap(), 2);
        assert_eq!(insert_batch(&pool, "a", "m", &samples).await.unwrap(), 0);
        assert_eq!(insert_batch(&pool, "b", "m", &samples).await.unwrap(), 2);
        let page = samples_after(&pool, "a", "m", 10, 100).await.unwrap();
        assert_eq!(page, [(20, 2)]);
    }
}
