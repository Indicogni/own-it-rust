//! The ingest listener: one task per connection, store first, acknowledge second.

use crate::link::{FrameReader, LinkError, write_message};
use crate::store;
use beacon_wire::Message;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};

/// A connection that sends nothing for this long is closed.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// A peer that does not take an `Ack` for this long is closed. Without this
/// limit, a client that sends batches and never reads would hold its slot forever.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// More connections than this wait in the kernel's accept queue.
const MAX_CONNECTIONS: usize = 256;

#[derive(Debug, thiserror::Error)]
enum IngestError {
    #[error(transparent)]
    Link(#[from] LinkError),
    #[error("database error: {0}")]
    Store(#[from] sqlx::Error),
}

/// Accepts connections until `stop` changes. Then the open connections are
/// dropped at their next `.await`: a batch that is being stored is rolled back
/// and not acknowledged, so its agent sends it again later. Returns when all
/// connection tasks have ended, or after five seconds.
pub async fn run(listener: TcpListener, pool: SqlitePool, mut stop: watch::Receiver<bool>) {
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        // Taking the permit before accepting bounds the number of tasks.
        let permit = tokio::select! {
            permit = Arc::clone(&slots).acquire_owned() => permit,
            _ = stop.changed() => break,
        };
        let Ok(permit) = permit else {
            break;
        };
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            _ = stop.changed() => break,
        };
        let Ok((stream, peer)) = accepted else {
            // For example "too many open files": wait a little, do not spin.
            tokio::time::sleep(Duration::from_millis(50)).await;
            continue;
        };
        let (pool, mut stop) = (pool.clone(), stop.clone());
        tokio::spawn(async move {
            let _permit = permit;
            let outcome = tokio::select! {
                outcome = serve(stream, &pool) => outcome,
                _ = stop.changed() => Ok(()),
            };
            if let Err(error) = outcome {
                tracing::warn!(%peer, %error, "ingest connection closed with an error");
            }
        });
    }
    drop(listener);
    // Every connection task holds a permit; all permits back means all done.
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        slots.acquire_many(MAX_CONNECTIONS as u32),
    )
    .await;
}

async fn serve(stream: TcpStream, pool: &SqlitePool) -> Result<(), IngestError> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = FrameReader::new(read_half);
    let mut next = async || match tokio::time::timeout(IDLE_TIMEOUT, reader.next()).await {
        Ok(result) => result,
        Err(_) => Err(LinkError::Timeout),
    };
    let agent = match next().await? {
        Some(Message::Hello { agent }) => agent,
        Some(_) => return Err(LinkError::Unexpected("the first message must be Hello").into()),
        None => return Ok(()),
    };
    while let Some(message) = next().await? {
        let Message::Batch {
            seq,
            metric,
            samples,
        } = message
        else {
            return Err(LinkError::Unexpected("only Batch may follow Hello").into());
        };
        // Commit, then acknowledge. An Ack for data that a crash could still
        // lose would be a lie that the agent acts on by deleting its copy.
        let new = store::insert_batch(pool, &agent, &metric, &samples).await?;
        tracing::debug!(%agent, seq, new, "batch stored");
        let ack = Message::Ack { seq };
        match tokio::time::timeout(WRITE_TIMEOUT, write_message(&mut write_half, &ack)).await {
            Ok(written) => written?,
            Err(_) => return Err(LinkError::Timeout.into()),
        }
    }
    Ok(())
}
