//! LESSON 07, part 2 — The protocol on a connection: partial reads, acks, backpressure
//! Read first: CONCEPTS.md, "Lesson 07". Do part 1 first: `wire/src/lib.rs`.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l07_
//!
//! Part 1 turned messages into bytes. This part moves them: a reader that copes
//! with whatever chunks the network delivers, a receiver that acknowledges a
//! batch only when the next stage has accepted it, and a sender that waits for
//! that acknowledgement. The two supplied programs `src/bin/l07_recv.rs` and
//! `src/bin/l07_send.rs` call your functions, and `tests/l07_two_programs.rs`
//! runs them as two real processes.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: the receiver's consumer is slow and
//! the queue between them is full. Name every buffer between the sender's
//! `write_all` and the consumer, and say what happens to the sender when all of
//! them are full.
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! Most tests use an in-memory pipe (`tokio::io::duplex`) and paused time, so
//! "nothing more arrives" can be checked exactly.
//! EXPLAIN afterward: What is backpressure, and which line of your `receive`
//! creates it? What could a sender conclude from an `Ack` if the receiver sent it
//! before handing the batch on? Why is an unbounded queue a bug that only shows
//! under load?
//! JUDGMENT: `tokio_util::codec` (`Framed`, `Decoder`, `Encoder`) packages the
//! reader and writer you build here; use it in real code, and know what it does.
//! Stop-and-wait (one batch in flight) is the simplest correct flow control. A
//! window of several unacknowledged batches is faster on slow links and is what
//! TCP, HTTP/2 and most message brokers do; it needs the same two rules: a bound,
//! and an acknowledgement that means something.

use beacon_wire::{Message, Sample, WireError, decode_frame, encode_frame};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;

/// What can go wrong on a connection.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Wire(#[from] WireError),
    #[error("the connection closed in the middle of a frame")]
    UnexpectedEof,
    #[error("unexpected message: {0}")]
    Unexpected(&'static str),
    #[error("the consumer of received batches is gone")]
    SinkClosed,
}

/// One batch as the receiver hands it to the next stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub agent: String,
    pub seq: u64,
    pub metric: String,
    pub samples: Vec<Sample>,
}

/// A. Reads whole messages from a byte stream. You may change the private fields.
#[derive(Debug)]
pub struct FrameReader<R> {
    io: R,
    buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(io: R) -> FrameReader<R> {
        FrameReader {
            io,
            buf: Vec::new(),
        }
    }

    /// Bytes received but not yet returned as part of a message.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// The next message.
    ///
    /// - A read may deliver part of a frame, exactly one frame, or several. Keep
    ///   what is left over for the next call, and do not read from the stream
    ///   while a complete frame is already buffered.
    /// - `Ok(None)` when the peer closed the connection between two frames.
    /// - `Err(UnexpectedEof)` when it closed inside a frame.
    /// - `Err(Wire(_))` as soon as `decode_frame` reports an error. In particular,
    ///   a header that announces an oversized payload fails at once, even if the
    ///   peer then sends nothing more and keeps the connection open.
    /// - Bytes of a returned message are removed from the buffer, so it does not
    ///   grow with the length of the connection.
    pub async fn next(&mut self) -> Result<Option<Message>, LinkError> {
        loop {
            if let Some((message, used)) = decode_frame(&self.buf)? {
                self.buf.drain(..used);
                return Ok(Some(message));
            }
            let mut chunk = [0u8; 4096];
            let count = self.io.read(&mut chunk).await?;
            if count == 0 {
                // End of stream. Fine between frames, an error inside one.
                return if self.buf.is_empty() {
                    Ok(None)
                } else {
                    Err(LinkError::UnexpectedEof)
                };
            }
            self.buf.extend_from_slice(&chunk[..count]);
        }
    }
}

/// B. Write one message as one frame and flush. Build the whole frame in memory
/// first and hand it over with a single `write_all`: if this future is dropped
/// between two small writes, the peer is left with half a frame.
pub async fn write_message<W: AsyncWrite + Unpin>(
    io: &mut W,
    message: &Message,
) -> Result<(), LinkError> {
    let mut frame = Vec::new();
    encode_frame(message, &mut frame)?;
    io.write_all(&frame).await?;
    io.flush().await?;
    Ok(())
}

/// C. Serve one connection as the receiver. Returns the number of batches that
/// were handed to `sink`, when the peer closes the connection between frames.
///
/// - The first message must be `Hello`; anything else is `Unexpected`. A peer
///   that closes the connection before it has sent anything is not an error: it
///   delivered zero batches, so the result is `Ok(0)`.
/// - For each `Batch`: first hand a [`Received`] to `sink` with `send(..).await`,
///   then write `Ack { seq }`. When `sink` is full, `send` waits. While it waits,
///   this function reads nothing, the socket buffers fill, and the sender is held
///   back. That is backpressure, and it is the point of this task: do not spawn a
///   task per batch, do not collect batches in a `Vec`, do not drop batches.
/// - An `Ack` therefore means "accepted by the next stage", never just "seen".
/// - A second `Hello`, or an `Ack` from the peer, is `Unexpected`.
/// - If `sink` is closed, return `SinkClosed` without acknowledging the batch.
pub async fn receive<S>(stream: S, sink: mpsc::Sender<Received>) -> Result<u64, LinkError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = FrameReader::new(read_half);
    let agent = match reader.next().await? {
        Some(Message::Hello { agent }) => agent,
        Some(_) => return Err(LinkError::Unexpected("the first message must be Hello")),
        None => return Ok(0),
    };
    let mut accepted = 0;
    while let Some(message) = reader.next().await? {
        let Message::Batch {
            seq,
            metric,
            samples,
        } = message
        else {
            return Err(LinkError::Unexpected("only Batch may follow Hello"));
        };
        let received = Received {
            agent: agent.clone(),
            seq,
            metric,
            samples,
        };
        // Waits here while the queue is full; nothing is read meanwhile.
        sink.send(received)
            .await
            .map_err(|_| LinkError::SinkClosed)?;
        write_message(&mut write_half, &Message::Ack { seq }).await?;
        accepted += 1;
    }
    Ok(accepted)
}

/// D. The sending side. Send `Hello { agent }`, then each batch as
/// `Batch { seq, metric, samples }` with `seq` counting 1, 2, 3, ... After each
/// batch, wait for `Ack` with the same `seq` before sending the next one
/// (stop-and-wait). Returns the number of acknowledged batches.
///
/// A message other than the expected `Ack` is `Unexpected`; a connection that
/// closes before the `Ack` is `UnexpectedEof`.
pub async fn send_batches<S>(
    stream: S,
    agent: &str,
    batches: Vec<(String, Vec<Sample>)>,
) -> Result<u64, LinkError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = FrameReader::new(read_half);
    let hello = Message::Hello {
        agent: agent.to_owned(),
    };
    write_message(&mut write_half, &hello).await?;
    let mut acked = 0;
    for (index, (metric, samples)) in batches.into_iter().enumerate() {
        let seq = index as u64 + 1;
        let batch = Message::Batch {
            seq,
            metric,
            samples,
        };
        write_message(&mut write_half, &batch).await?;
        match reader.next().await? {
            Some(Message::Ack { seq: got }) if got == seq => acked += 1,
            Some(_) => return Err(LinkError::Unexpected("expected the Ack for the last batch")),
            None => return Err(LinkError::UnexpectedEof),
        }
    }
    Ok(acked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{deadline, settle};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::io::{DuplexStream, duplex};
    use tokio::net::{TcpListener, TcpStream};

    fn samples(first: i64, count: i64) -> Vec<Sample> {
        (0..count)
            .map(|i| Sample {
                ts_ms: 1_800_000_000_000 + (first + i) as u64 * 10,
                value: first + i,
            })
            .collect()
    }

    fn batch(seq: u64) -> Message {
        Message::Batch {
            seq,
            metric: "rpm".to_owned(),
            samples: samples(seq as i64 * 10, 3),
        }
    }

    fn hello() -> Message {
        Message::Hello {
            agent: "pump-7".to_owned(),
        }
    }

    fn frames(messages: &[Message]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for message in messages {
            encode_frame(message, &mut bytes).unwrap();
        }
        bytes
    }

    /// Writes `bytes` in pieces of `piece` bytes, flushing after each, then closes.
    fn feed(bytes: Vec<u8>, piece: usize, pipe: usize) -> FrameReader<DuplexStream> {
        let (mut tx, rx) = duplex(pipe);
        tokio::spawn(async move {
            for chunk in bytes.chunks(piece) {
                tx.write_all(chunk).await.unwrap();
                tx.flush().await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        FrameReader::new(rx)
    }

    /// Waits up to one (virtual) second for a message.
    async fn next_soon<R: AsyncRead + Unpin>(reader: &mut FrameReader<R>) -> Option<Message> {
        match tokio::time::timeout(Duration::from_secs(1), reader.next()).await {
            Ok(result) => Some(
                result
                    .expect("no link error")
                    .expect("the peer did not close"),
            ),
            Err(_) => None,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_messages_survive_any_chunking() {
        crate::support::deadline(600, "the test `a_messages_survive_any_chunking`", async {
            let messages = [hello(), batch(1), batch(2), Message::Ack { seq: 2 }];
            let bytes = frames(&messages);
            for (piece, pipe) in [(1, 1), (3, 5), (7, 64), (bytes.len(), 4096)] {
                let mut reader = feed(bytes.clone(), piece, pipe);
                for expected in &messages {
                    let got = deadline(5, "FrameReader::next", reader.next())
                        .await
                        .unwrap();
                    assert_eq!(
                        got.as_ref(),
                        Some(expected),
                        "pieces of {piece} bytes through a {pipe}-byte pipe"
                    );
                }
                let end = deadline(5, "FrameReader::next at the end", reader.next()).await;
                assert!(
                    matches!(end, Ok(None)),
                    "a close between frames is a clean end, got {end:?}"
                );
                assert_eq!(
                    reader.buffered(),
                    0,
                    "bytes of returned messages must leave the buffer"
                );
            }
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn a_glued_frames_are_returned_without_another_read() {
        crate::support::deadline(
            600,
            "the test `a_glued_frames_are_returned_without_another_read`",
            async {
                let (mut tx, rx) = duplex(4096);
                let mut reader = FrameReader::new(rx);
                tx.write_all(&frames(&[batch(1), batch(2), batch(3)]))
                    .await
                    .unwrap();
                // The writer stays open and sends nothing more. All three messages are
                // already here; a reader that insists on reading first would wait forever.
                for seq in 1..=3 {
                    let got = deadline(5, "a buffered message", reader.next())
                        .await
                        .unwrap();
                    assert_eq!(got, Some(batch(seq)));
                }
                assert!(
                    next_soon(&mut reader).await.is_none(),
                    "there is no fourth message"
                );
                drop(tx);
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn a_a_close_inside_a_frame_is_an_error() {
        crate::support::deadline(
            600,
            "the test `a_a_close_inside_a_frame_is_an_error`",
            async {
                let mut bytes = frames(&[hello(), batch(1)]);
                bytes.truncate(bytes.len() - 3);
                let mut reader = feed(bytes, 16, 64);
                assert_eq!(reader.next().await.unwrap(), Some(hello()));
                let cut = deadline(5, "reading the cut frame", reader.next()).await;
                assert!(
                    matches!(cut, Err(LinkError::UnexpectedEof)),
                    "half a frame followed by a close is not a clean end, got {cut:?}"
                );
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn a_bad_bytes_fail_at_once() {
        crate::support::deadline(600, "the test `a_bad_bytes_fail_at_once`", async {
            let mut reader = feed(b"GET / HTTP/1.1\r\n\r\n".to_vec(), 64, 64);
            let wrong = deadline(5, "reading another protocol", reader.next()).await;
            assert!(
                matches!(wrong, Err(LinkError::Wire(WireError::BadMagic(_)))),
                "{wrong:?}"
            );

            // A header that announces almost 4 GiB; then the peer goes quiet but
            // keeps the connection open.
            let (mut tx, rx) = duplex(64);
            let mut reader = FrameReader::new(rx);
            tx.write_all(&[b'B', b'W', 1, 2, 0xff, 0xff, 0xff, 0xf0])
                .await
                .unwrap();
            let oversized = deadline(5, "refusing an oversized frame", reader.next()).await;
            assert!(
                matches!(oversized, Err(LinkError::Wire(WireError::TooLarge { .. }))),
                "{oversized:?}"
            );
            drop(tx);
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn b_write_message_sends_exactly_one_frame() {
        crate::support::deadline(
            600,
            "the test `b_write_message_sends_exactly_one_frame`",
            async {
                let (mut tx, mut rx) = duplex(4096);
                write_message(&mut tx, &batch(5)).await.unwrap();
                write_message(&mut tx, &Message::Ack { seq: 5 })
                    .await
                    .unwrap();
                drop(tx);
                let mut received = Vec::new();
                rx.read_to_end(&mut received).await.unwrap();
                assert_eq!(received, frames(&[batch(5), Message::Ack { seq: 5 }]));

                let (mut tx, _rx) = duplex(4096);
                let huge = Message::Hello {
                    agent: "x".repeat(beacon_wire::MAX_PAYLOAD + 1),
                };
                let refused = write_message(&mut tx, &huge).await;
                assert!(
                    matches!(refused, Err(LinkError::Wire(WireError::TooLarge { .. }))),
                    "{refused:?}"
                );
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn c_receive_forwards_batches_and_acks_each() {
        crate::support::deadline(
            600,
            "the test `c_receive_forwards_batches_and_acks_each`",
            async {
                let (client, server) = duplex(256);
                let (sink, mut queue) = mpsc::channel(8);
                let receiver = tokio::spawn(receive(server, sink));
                let (client_read, mut client_write) = tokio::io::split(client);
                let mut acks = FrameReader::new(client_read);

                write_message(&mut client_write, &hello()).await.unwrap();
                for seq in [1, 2, 7] {
                    write_message(&mut client_write, &batch(seq)).await.unwrap();
                    let ack = deadline(5, "an Ack", acks.next()).await.unwrap();
                    assert_eq!(ack, Some(Message::Ack { seq }));
                    let received = queue
                        .try_recv()
                        .expect("the batch is in the queue before its Ack");
                    assert_eq!(
                        received,
                        Received {
                            agent: "pump-7".to_owned(),
                            seq,
                            metric: "rpm".to_owned(),
                            samples: samples(seq as i64 * 10, 3),
                        }
                    );
                }
                drop(client_write);
                drop(acks);
                let accepted = deadline(5, "receive after the peer closed", receiver).await;
                assert_eq!(accepted.unwrap().unwrap(), 3);
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn c_a_batch_is_acknowledged_only_after_the_queue_took_it() {
        crate::support::deadline(
            600,
            "the test `c_a_batch_is_acknowledged_only_after_the_queue_took_it`",
            async {
                let (client, server) = duplex(4096);
                let (sink, mut queue) = mpsc::channel(2);
                let receiver = tokio::spawn(receive(server, sink));
                let (client_read, mut client_write) = tokio::io::split(client);
                let mut acks = FrameReader::new(client_read);

                // Nobody consumes the queue, which has room for two batches.
                write_message(&mut client_write, &hello()).await.unwrap();
                for seq in 1..=3 {
                    write_message(&mut client_write, &batch(seq)).await.unwrap();
                }
                assert_eq!(next_soon(&mut acks).await, Some(Message::Ack { seq: 1 }));
                assert_eq!(next_soon(&mut acks).await, Some(Message::Ack { seq: 2 }));
                assert_eq!(
                    next_soon(&mut acks).await,
                    None,
                    "batch 3 does not fit in the queue, so it must not be acknowledged yet: \
             an Ack promises that the next stage has the batch"
                );
                assert_eq!(
                    queue.len(),
                    2,
                    "batch 3 must wait; it must not be dropped or hidden"
                );
                assert!(!receiver.is_finished());

                // The consumer takes one batch: now there is room, and the Ack follows.
                assert_eq!(queue.recv().await.unwrap().seq, 1);
                assert_eq!(next_soon(&mut acks).await, Some(Message::Ack { seq: 3 }));
                assert_eq!(queue.recv().await.unwrap().seq, 2);
                assert_eq!(queue.recv().await.unwrap().seq, 3);
                receiver.abort();
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn c_a_full_queue_stops_the_sender() {
        crate::support::deadline(600, "the test `c_a_full_queue_stops_the_sender`", async {
            // A 64-byte pipe stands in for the socket buffers.
            let (client, server) = duplex(64);
            let (sink, queue) = mpsc::channel(2);
            let receiver = tokio::spawn(receive(server, sink));
            let (client_read, mut client_write) = tokio::io::split(client);
            // Acks are read and thrown away, so the only thing that can stop the
            // sender is the full queue.
            let ack_reader = tokio::spawn(async move {
                let mut acks = FrameReader::new(client_read);
                while let Ok(Some(_)) = acks.next().await {}
            });

            let written = Arc::new(AtomicUsize::new(0));
            let progress = Arc::clone(&written);
            let writer = tokio::spawn(async move {
                write_message(&mut client_write, &hello()).await.unwrap();
                for seq in 1..=200 {
                    write_message(&mut client_write, &batch(seq)).await.unwrap();
                    progress.fetch_add(1, Ordering::SeqCst);
                }
            });

            // Nobody reads the queue. Wait until no task can make progress any more.
            settle().await;
            let stuck_at = written.load(Ordering::SeqCst);
            assert!(
                stuck_at < 200,
                "all 200 batches were accepted although nothing consumes them: \
             something in the receiver buffers without limit"
            );
            assert!(
                stuck_at <= 12,
                "{stuck_at} batches were written before the sender stopped. With a \
             queue of 2 and a 64-byte pipe only a handful fit: the receiver must \
             stop reading while the queue is full"
            );
            assert_eq!(queue.len(), 2);
            writer.abort();
            receiver.abort();
            ack_reader.abort();
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn c_protocol_violations_and_a_closed_sink() {
        crate::support::deadline(
            600,
            "the test `c_protocol_violations_and_a_closed_sink`",
            async {
                // A batch before Hello.
                let (mut client, server) = duplex(4096);
                let (sink, _queue) = mpsc::channel(8);
                write_message(&mut client, &batch(1)).await.unwrap();
                let result = deadline(5, "receive", receive(server, sink)).await;
                assert!(
                    matches!(result, Err(LinkError::Unexpected(_))),
                    "{result:?}"
                );

                // An Ack sent by the client.
                let (mut client, server) = duplex(4096);
                let (sink, _queue) = mpsc::channel(8);
                client
                    .write_all(&frames(&[hello(), Message::Ack { seq: 1 }]))
                    .await
                    .unwrap();
                let result = deadline(5, "receive", receive(server, sink)).await;
                assert!(
                    matches!(result, Err(LinkError::Unexpected(_))),
                    "{result:?}"
                );

                // The consumer is gone: the batch is not acknowledged.
                let (client, server) = duplex(4096);
                let (sink, queue) = mpsc::channel(8);
                drop(queue);
                let (client_read, mut client_write) = tokio::io::split(client);
                client_write
                    .write_all(&frames(&[hello(), batch(1)]))
                    .await
                    .unwrap();
                let result = deadline(5, "receive", receive(server, sink)).await;
                assert!(matches!(result, Err(LinkError::SinkClosed)), "{result:?}");
                let mut acks = FrameReader::new(client_read);
                let end = deadline(5, "the connection closing", acks.next()).await;
                assert!(
                    matches!(end, Ok(None)),
                    "no Ack may be sent for a batch that nobody took, got {end:?}"
                );

                // A connection that closes before Hello carried nothing.
                let (client, server) = duplex(64);
                let (sink, _queue) = mpsc::channel(8);
                drop(client);
                let result = deadline(5, "receive", receive(server, sink)).await;
                assert!(matches!(result, Ok(0)), "{result:?}");
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn d_the_sender_waits_for_each_ack() {
        crate::support::deadline(600, "the test `d_the_sender_waits_for_each_ack`", async {
            let (client, server) = duplex(4096);
            let work = vec![
                ("rpm".to_owned(), samples(0, 2)),
                ("rpm".to_owned(), samples(2, 2)),
                ("temp".to_owned(), samples(4, 1)),
            ];
            let sender = tokio::spawn(send_batches(client, "pump-7", work));
            let (server_read, mut server_write) = tokio::io::split(server);
            let mut incoming = FrameReader::new(server_read);

            assert_eq!(next_soon(&mut incoming).await, Some(hello()));
            let first = Message::Batch {
                seq: 1,
                metric: "rpm".to_owned(),
                samples: samples(0, 2),
            };
            assert_eq!(next_soon(&mut incoming).await, Some(first));
            assert_eq!(
                next_soon(&mut incoming).await,
                None,
                "batch 2 was sent before batch 1 was acknowledged"
            );
            write_message(&mut server_write, &Message::Ack { seq: 1 })
                .await
                .unwrap();
            let second = next_soon(&mut incoming).await;
            assert!(
                matches!(&second, Some(Message::Batch { seq: 2, metric, .. }) if metric == "rpm"),
                "{second:?}"
            );
            write_message(&mut server_write, &Message::Ack { seq: 2 })
                .await
                .unwrap();
            let third = next_soon(&mut incoming).await;
            assert!(
                matches!(&third, Some(Message::Batch { seq: 3, metric, .. }) if metric == "temp"),
                "{third:?}"
            );
            write_message(&mut server_write, &Message::Ack { seq: 3 })
                .await
                .unwrap();
            let acked = deadline(5, "send_batches", sender).await.unwrap();
            assert_eq!(acked.unwrap(), 3);
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn d_wrong_or_missing_acks_are_errors() {
        crate::support::deadline(
            600,
            "the test `d_wrong_or_missing_acks_are_errors`",
            async {
                // The receiver acknowledges a different batch.
                let (client, server) = duplex(4096);
                let work = vec![("rpm".to_owned(), samples(0, 1))];
                let sender = tokio::spawn(send_batches(client, "pump-7", work.clone()));
                let (server_read, mut server_write) = tokio::io::split(server);
                let mut incoming = FrameReader::new(server_read);
                incoming.next().await.unwrap();
                incoming.next().await.unwrap();
                write_message(&mut server_write, &Message::Ack { seq: 99 })
                    .await
                    .unwrap();
                let result = deadline(5, "send_batches", sender).await.unwrap();
                assert!(
                    matches!(result, Err(LinkError::Unexpected(_))),
                    "{result:?}"
                );

                // The receiver closes without acknowledging.
                let (client, server) = duplex(4096);
                let sender = tokio::spawn(send_batches(client, "pump-7", work));
                let mut incoming = FrameReader::new(server);
                incoming.next().await.unwrap();
                incoming.next().await.unwrap();
                drop(incoming);
                let result = deadline(5, "send_batches", sender).await.unwrap();
                assert!(
                    matches!(
                        result,
                        Err(LinkError::UnexpectedEof) | Err(LinkError::Io(_))
                    ),
                    "a batch without an Ack must not count as delivered, got {result:?}"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn e_both_ends_over_loopback_tcp() {
        crate::support::deadline(120, "the test `e_both_ends_over_loopback_tcp`", async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (sink, mut queue) = mpsc::channel(2);
            let receiver = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                receive(stream, sink).await
            });
            let consumer = tokio::spawn(async move {
                let mut values = Vec::new();
                while let Some(received) = queue.recv().await {
                    assert_eq!(received.agent, "tcp-agent");
                    values.extend(received.samples.iter().map(|sample| sample.value));
                }
                values
            });

            let work: Vec<(String, Vec<Sample>)> = (0..50)
                .map(|batch| ("rpm".to_owned(), samples(batch * 20, 20)))
                .collect();
            let stream = TcpStream::connect(addr).await.unwrap();
            let acked = deadline(
                20,
                "send_batches over TCP",
                send_batches(stream, "tcp-agent", work),
            )
            .await
            .unwrap();
            assert_eq!(acked, 50);
            let accepted = deadline(20, "receive over TCP", receiver).await.unwrap();
            assert_eq!(accepted.unwrap(), 50);
            let values = deadline(20, "the consumer", consumer).await.unwrap();
            assert_eq!(
                values,
                (0..1000).collect::<Vec<i64>>(),
                "nothing lost, nothing reordered"
            );
        })
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test(start_paused = true)]
    async fn p_predictions() {
        crate::support::deadline(600, "the test `p_predictions`", async {
            // P1. A pipe holds at most 16 bytes. One side writes 40 bytes with
            // `write_all`; nobody reads. Does the write finish within a second?
            let (mut tx, mut rx) = duplex(16);
            let write =
                tokio::time::timeout(Duration::from_secs(1), tx.write_all(&[7u8; 40])).await;
            let p1: bool = false;
            assert_eq!(p1, write.is_ok(), "P1: a full pipe makes the writer wait");

            // P2. The writer is dropped. The reader now reads into a 64-byte buffer
            // three times. How many bytes does each read return?
            drop(tx);
            let mut chunk = [0u8; 64];
            let reads = (
                rx.read(&mut chunk).await.unwrap(),
                rx.read(&mut chunk).await.unwrap(),
                rx.read(&mut chunk).await.unwrap(),
            );
            let p2: (usize, usize, usize) = (16, 0, 0);
            assert_eq!(p2, reads, "P2: buffered bytes first, then end of stream");

            // P3. A bounded channel with room for two. Three values are offered with
            // `try_send`. How many are accepted, and what is `capacity()` after one
            // value has been received?
            let (tx, mut rx) = mpsc::channel::<u8>(2);
            let accepted = (0..3).filter(|value| tx.try_send(*value).is_ok()).count();
            rx.recv().await;
            let p3: (usize, usize) = (2, 1);
            assert_eq!(p3, (accepted, tx.capacity()), "P3: a bounded queue says no");
        })
        .await
    }
}
