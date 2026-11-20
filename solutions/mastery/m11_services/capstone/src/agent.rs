//! The agent: a sampler and a shipper that share a bounded buffer.
//!
//! The sampler never waits for the network: it appends to the buffer, and when
//! the buffer is full it discards the oldest sample. The shipper sends batches
//! from the front of the buffer and removes samples only when the server has
//! acknowledged them, so a lost connection loses nothing that is still buffered.

use crate::config::AgentConfig;
use crate::link::{FrameReader, LinkError, write_message};
use beacon_wire::{Message, Sample};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::Notify;

const FIRST_RETRY: Duration = Duration::from_millis(50);
const MAX_RETRY: Duration = Duration::from_secs(1);
/// Longest wait for a connection to open and take the `Hello`.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Longest wait for one batch to be written and acknowledged.
const ACK_TIMEOUT: Duration = Duration::from_secs(5);

/// What the agent did, for the final line of output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    pub sampled: u64,
    pub shipped: u64,
    pub dropped: u64,
}

#[derive(Default)]
struct Buffer {
    /// Samples the server has not acknowledged, oldest first. This includes
    /// samples of a batch that is in flight.
    pending: VecDeque<Sample>,
    totals: Totals,
    sampling_done: bool,
}

#[derive(Default)]
struct Shared {
    buffer: Mutex<Buffer>,
    /// Signalled when a sample arrives or sampling ends.
    changed: Notify,
}

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Buffer> {
        self.buffer.lock().expect("buffer lock")
    }
}

/// Runs one step of network work with a time limit. A server that accepts a
/// connection and then stops reading or answering must not hold the shipper
/// forever: after the limit the caller reconnects.
async fn within<T>(
    limit: Duration,
    step: impl Future<Output = Result<T, LinkError>>,
) -> Result<T, LinkError> {
    tokio::time::timeout(limit, step)
        .await
        .unwrap_or(Err(LinkError::Timeout))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

async fn sample_loop(config: AgentConfig, shared: Arc<Shared>) {
    let mut ticker = tokio::time::interval(config.interval);
    // After a stall, continue at the normal rate instead of sampling in a burst.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_ts = 0;
    let mut number = 0u64;
    while config.count.is_none_or(|count| number < count) {
        ticker.tick().await;
        // Strictly increasing times, even if the clock stands still or steps back:
        // the server uses (agent, metric, time) as the identity of a sample.
        last_ts = now_ms().max(last_ts + 1);
        let sample = Sample {
            ts_ms: last_ts,
            value: number as i64,
        };
        number += 1;
        {
            let mut buffer = shared.lock();
            if buffer.pending.len() >= config.buffer {
                buffer.pending.pop_front();
                buffer.totals.dropped += 1;
            }
            buffer.pending.push_back(sample);
            buffer.totals.sampled += 1;
        }
        shared.changed.notify_one();
    }
    shared.lock().sampling_done = true;
    shared.changed.notify_one();
    println!("sampled={number}");
    let _ = std::io::stdout().flush();
}

/// One connection: Hello, then batches until the buffer is empty and sampling
/// has ended (`Ok`), or until something fails (`Err`, the caller reconnects).
async fn ship(config: &AgentConfig, shared: &Shared) -> Result<(), LinkError> {
    {
        // Nothing left to deliver: do not ask for a server that may be gone.
        let buffer = shared.lock();
        if buffer.pending.is_empty() && buffer.sampling_done {
            return Ok(());
        }
    }
    let (mut reader, mut write_half) = within(CONNECT_TIMEOUT, async {
        let stream = TcpStream::connect(&config.server).await?;
        let (read_half, mut write_half) = stream.into_split();
        let hello = Message::Hello {
            agent: config.agent_id.clone(),
        };
        write_message(&mut write_half, &hello).await?;
        Ok((FrameReader::new(read_half), write_half))
    })
    .await?;
    let mut seq = 0;
    loop {
        // Register for the wake-up before looking, so no signal is missed.
        let changed = shared.changed.notified();
        let batch: Vec<Sample> = {
            let buffer = shared.lock();
            if buffer.pending.is_empty() && buffer.sampling_done {
                return Ok(());
            }
            buffer.pending.iter().take(config.batch).copied().collect()
        };
        let Some(newest) = batch.last().map(|sample| sample.ts_ms) else {
            changed.await;
            continue;
        };
        seq += 1;
        let message = Message::Batch {
            seq,
            metric: config.metric.clone(),
            samples: batch,
        };
        // One limit for the write and the answer: the write waits too when the
        // server has stopped reading and the socket buffers are full.
        let exchange = async {
            write_message(&mut write_half, &message).await?;
            reader.next().await
        };
        match within(ACK_TIMEOUT, exchange).await? {
            Some(Message::Ack { seq: acked }) if acked == seq => {}
            Some(_) => return Err(LinkError::Unexpected("expected the Ack for the batch")),
            None => return Err(LinkError::UnexpectedEof),
        }
        // Acknowledged: now, and only now, the samples may leave the buffer.
        // Some may have been discarded meanwhile to make room; those were
        // already counted as dropped.
        let mut buffer = shared.lock();
        while buffer.pending.front().is_some_and(|s| s.ts_ms <= newest) {
            buffer.pending.pop_front();
            buffer.totals.shipped += 1;
        }
    }
}

/// Runs the agent to completion (only returns if `config.count` is set).
pub async fn run(config: AgentConfig) -> Totals {
    let shared = Arc::new(Shared::default());
    let sampler = tokio::spawn(sample_loop(config.clone(), Arc::clone(&shared)));
    let mut retry = FIRST_RETRY;
    loop {
        let shipped_before = shared.lock().totals.shipped;
        match ship(&config, &shared).await {
            Ok(()) => break,
            Err(error) => {
                // A connection that delivered something was a good one: the
                // next retry is a first retry again.
                if shared.lock().totals.shipped > shipped_before {
                    retry = FIRST_RETRY;
                }
                tracing::warn!(%error, ?retry, "connection lost, will retry");
                tokio::time::sleep(retry).await;
                // Back off, so a server that is down is not hammered.
                retry = (retry * 2).min(MAX_RETRY);
            }
        }
    }
    let _ = sampler.await;
    // Edition 2024 drops the lock guard, a temporary of this tail expression,
    // before the local `shared`.
    shared.lock().totals
}
