//! LESSON 09 — The same service on tokio: backpressure, limits, graceful shutdown
//! Read first: CONCEPTS.md, "Lesson 09". Start after: lesson 08, and lesson
//! E06 of the ecosystem track if you have not used tokio before.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l09_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. In lesson 08 a full queue removed the client. Here a full queue makes
//!    the *sender* wait. Whose socket stops being read while it waits, and
//!    what does the operating system then do to the program at the other end?
//! 2. Two clients talk at the same moment and each one's queue is full. Why
//!    does this deadlock if one task per client both reads and writes in a
//!    single loop, and what removes the deadlock?
//!
//! IMPLEMENT `serve` from the contract below. You design everything inside.
//! Most tests use in-memory connections and tokio's paused clock, so "wait
//! 30 seconds" costs nothing and is exact; one test uses real loopback TCP.
//! EXPLAIN: Describe graceful shutdown as a sequence: what stops first, what
//! each connection still does, what bounds the wait, and what happens to a
//! connection that does not finish in time. Then say which of your `.await`
//! points a `select!` may cancel. For each one: is it safe to cancel, and if
//! it is not, what is lost and why is that right at this point?
//! JUDGMENT: tokio gives you the parts you built in lessons 02 to 07, made
//! for production: a work-stealing executor, a timer wheel, a reactor on
//! epoll, kqueue or IOCP, channels with a waiting `send`, `select!`, and
//! cooperative budgets. Use it (or another maintained runtime) for real
//! services. Your own runtime was the way to understand what it does.
//!
//! # The contract
//!
//! The protocol is the one of lesson 08, rules 1 to 5 and rule 8, with the
//! field names of [`Limits`] (`idle` is the idle time). These rules differ:
//!
//! - **Backpressure (replaces rule 6).** At most `outbox` lines wait for one
//!   client. When that queue is full, whoever wants to add a line *waits*
//!   for room, and reads no further line from its own client meanwhile. No
//!   line is dropped for a client that is merely slow.
//! - **Stuck clients.** A client that does not take a line for `patience`
//!   is removed from the room (the others get `* NAME left`) and its
//!   connection is closed. Senders that wait for room in its queue continue
//!   at once.
//! - **Connection limit.** At most `max_connections` connections are served
//!   at the same time. Further connections are not accepted until one
//!   ends: they wait in the listener.
//! - **Stopping (replaces rule 9).** When `shutdown` is cancelled, the
//!   server drops its listener at once, so nothing more is accepted. Every
//!   client gets the lines that are already queued for it, then
//!   `* server closing`, and its connection is closed. Only queued lines
//!   are promised: a line that still waits for room in a full queue at that
//!   moment is given up, so that the client that sent it is not held back
//!   by a client that takes no data. No `left` lines are sent for clients
//!   that leave because of the shutdown. `serve` waits up to `grace` for
//!   the connections to finish, aborts those that have not, and returns
//!   the [`Report`].
//! - **Fairness (rule 7)** needs no code of yours here: tokio's channels and
//!   sockets make a task yield after a fixed number of operations.

use std::future::Future;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::io::AsyncWriteExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use tokio::time::{Instant, sleep, timeout, timeout_at};

/// The limits of a server. `max_connections` and `outbox` must be at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_connections: usize,
    pub outbox: usize,
    pub max_line: usize,
    pub idle: Duration,
    pub patience: Duration,
    pub grace: Duration,
}

/// What a server did, returned by [`serve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Clients that were welcomed.
    pub joined: u64,
    /// Chat lines that were passed on, counted once per line.
    pub messages: u64,
    /// Connections that had not finished when `grace` was over.
    pub aborted: usize,
}

/// Where connections come from. `serve` is written against this trait, so
/// the same code serves TCP sockets and the in-memory pipes of the tests.
pub trait Listener: Send + 'static {
    type Io: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    /// Waits for the next connection. Implementations must be cancellation
    /// safe: a connection is handed out only by the poll that returns it.
    fn accept(&mut self) -> impl Future<Output = io::Result<Self::Io>> + Send;
}

impl Listener for TcpListener {
    type Io = TcpStream;

    async fn accept(&mut self) -> io::Result<TcpStream> {
        let (stream, _peer) = TcpListener::accept(self).await?;
        Ok(stream)
    }
}

/// A channel of ready-made connections is a listener too. When every sender
/// is gone, no connection can arrive any more, and `accept` waits for ever.
impl<S> Listener for mpsc::Receiver<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Io = S;

    async fn accept(&mut self) -> io::Result<S> {
        match self.recv().await {
            Some(io) => Ok(io),
            None => std::future::pending().await,
        }
    }
}

/// Given code: the line reader of lesson 07, on tokio's `AsyncRead`.
pub struct Lines<R> {
    reader: R,
    /// Bytes that were received and not yet returned as a line.
    buffer: Vec<u8>,
    /// `buffer[..scanned]` is known to contain no `\n`.
    scanned: usize,
    max_len: usize,
    ended: bool,
}

impl<R: AsyncRead + Unpin> Lines<R> {
    /// A reader for lines of at most `max_len` bytes, not counting the `\n`.
    pub fn new(reader: R, max_len: usize) -> Lines<R> {
        Lines {
            reader,
            buffer: Vec::new(),
            scanned: 0,
            max_len,
            ended: false,
        }
    }

    /// The next line without its `\n` (and without a `\r` before it);
    /// `Ok(None)` at the end of the input. A line that is too long gives an
    /// error for which [`is_too_long`] is `true`.
    ///
    /// Cancellation safe: every received byte is in `self.buffer` before
    /// this function waits again, and tokio's `read_buf` promises that a
    /// cancelled read has read nothing.
    pub async fn next_line(&mut self) -> io::Result<Option<String>> {
        loop {
            let newline = self.buffer[self.scanned..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|offset| self.scanned + offset);
            if newline.unwrap_or(self.buffer.len()) > self.max_len {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"));
            }
            if let Some(end) = newline {
                let mut line: Vec<u8> = self.buffer.drain(..=end).collect();
                self.scanned = 0;
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
            self.scanned = self.buffer.len();
            if self.ended {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                self.scanned = 0;
                let line = std::mem::take(&mut self.buffer);
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
            self.buffer.reserve(1024);
            if self.reader.read_buf(&mut self.buffer).await? == 0 {
                self.ended = true;
            }
        }
    }
}

/// `true` for the error that [`Lines::next_line`] returns for a long line.
pub fn is_too_long(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::InvalidData
}

/// Serves connections from `listener` until `shutdown` is cancelled, then
/// shuts down as the contract says and returns the report.
pub async fn serve<L: Listener>(
    mut listener: L,
    limits: Limits,
    shutdown: CancellationToken,
) -> Report {
    let shared = Arc::new(Shared {
        limits,
        shutdown: shutdown.clone(),
        room: Mutex::new(Room::default()),
    });
    let permits = Arc::new(Semaphore::new(limits.max_connections));
    let mut connections = JoinSet::new();
    loop {
        // The permit comes first: a connection is accepted only when there
        // is room to serve it. The others stay in the listener's queue,
        // where the operating system holds them back.
        let permit = tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            permit = Arc::clone(&permits).acquire_owned() => {
                permit.expect("the semaphore is never closed")
            }
        };
        let accepted = tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            accepted = listener.accept() => accepted,
        };
        match accepted {
            // The task owns the permit, so the slot is free again when the
            // task ends, however it ends.
            Ok(io) => {
                connections.spawn(connection(io, Arc::clone(&shared), permit));
            }
            Err(_) => {
                drop(permit);
                tokio::select! {
                    biased;
                    () = shutdown.cancelled() => break,
                    () = sleep(ACCEPT_BACKOFF) => {}
                }
            }
        }
        // Forget tasks that have finished, so the set does not grow.
        while connections.try_join_next().is_some() {}
    }
    // Stop accepting before the old connections say goodbye.
    drop(listener);
    let drain = async { while connections.join_next().await.is_some() {} };
    let _ = timeout(limits.grace, drain).await;
    let aborted = connections.len();
    // Aborts what is left and waits until those tasks are gone.
    connections.shutdown().await;
    let room = lock(&shared.room);
    Report {
        joined: room.joined,
        messages: room.messages,
        aborted,
    }
}

/// How long the accept loop pauses after a failed `accept`.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// Locks a mutex and ignores poisoning. The room is locked only for short
/// sections without an `.await`, so a `std` mutex is the right one.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One line on its way to one or more clients.
type Line = Arc<str>;

/// The queues of the clients that a line goes to.
type Queues = Vec<mpsc::Sender<Line>>;

struct Shared {
    limits: Limits,
    shutdown: CancellationToken,
    room: Mutex<Room>,
}

#[derive(Default)]
struct Room {
    members: BTreeMap<String, Member>,
    next_id: u64,
    joined: u64,
    messages: u64,
}

struct Member {
    /// Tells this membership apart from a later one with the same name.
    id: u64,
    outbox: mpsc::Sender<Line>,
}

impl Room {
    /// The queue of every member except `skip`.
    fn others(&self, skip: Option<&str>) -> Queues {
        self.members
            .iter()
            .filter(|(name, _)| Some(name.as_str()) != skip)
            .map(|(_, member)| member.outbox.clone())
            .collect()
    }
}

/// Queues `line` for every target and waits for room where a queue is
/// full. That wait is the backpressure. It ends when the client takes a
/// line, or when the client's writer gives up and closes the queue; a
/// closed queue is not an error for the sender.
async fn deliver(targets: Queues, line: Line) {
    for outbox in targets {
        let _ = outbox.send(Arc::clone(&line)).await;
    }
}

/// Runs `delivery` to its end, unless the shutdown begins while it waits
/// for room. Returns `false` in that case.
///
/// This is the only place where a `send` can lose a `select!`, and a `send`
/// that loses drops its line. That is allowed here and nowhere else: once
/// the shutdown has begun, the contract promises only the lines that are
/// already queued. Without this exit, one client that takes no data would
/// keep every connection that waits for room in its queue from saying
/// goodbye. `biased` polls the delivery first, so a delivery that can
/// finish at once always does.
async fn unless_shutdown(shared: &Shared, delivery: impl Future<Output = ()>) -> bool {
    tokio::select! {
        biased;
        () = delivery => true,
        () = shared.shutdown.cancelled() => false,
    }
}

/// A client's place in the room.
struct Membership {
    shared: Arc<Shared>,
    name: String,
    id: u64,
    /// This client's own queue, for the answers that only it gets.
    outbox: mpsc::Sender<Line>,
}

impl Membership {
    /// Enters the room unless the name is taken. Also returns the receiving
    /// end of the new queue and the queues that must hear of the arrival.
    fn enter(
        shared: &Arc<Shared>,
        name: &str,
    ) -> Option<(Membership, mpsc::Receiver<Line>, Queues)> {
        let (outbox, inbox) = mpsc::channel(shared.limits.outbox);
        // The welcome goes into the queue before the room, and so any other
        // task, can see the queue. It is therefore the client's first line
        // on every runtime, also on one with several worker threads. The
        // queue is new and has room for at least one line, so this cannot
        // fail.
        let _ = outbox.try_send(format!("* welcome {name}\n").into());
        let mut room = lock(&shared.room);
        if room.members.contains_key(name) {
            return None;
        }
        let id = room.next_id;
        room.next_id += 1;
        room.joined += 1;
        let others = room.others(None);
        let member = Member {
            id,
            outbox: outbox.clone(),
        };
        room.members.insert(name.to_owned(), member);
        drop(room);
        let membership = Membership {
            shared: Arc::clone(shared),
            name: name.to_owned(),
            id,
            outbox,
        };
        Some((membership, inbox, others))
    }

    /// A line for this client only.
    async fn tell(&self, line: String) {
        let _ = self.outbox.send(line.into()).await;
    }

    /// A chat line for everyone else.
    async fn say(&self, text: &str) {
        let others = {
            let mut room = lock(&self.shared.room);
            room.messages += 1;
            room.others(Some(&self.name))
        };
        deliver(others, format!("{}: {text}\n", self.name).into()).await;
    }

    /// The answer to `/who`.
    fn who(&self) -> String {
        let room = lock(&self.shared.room);
        let names: Vec<&str> = room.members.keys().map(String::as_str).collect();
        format!("* online: {}\n", names.join(", "))
    }

    /// Takes this membership out of the room, once. Returns the queues that
    /// should hear of it, or `None` if it was removed before.
    fn remove(&self) -> Option<Queues> {
        let mut room = lock(&self.shared.room);
        if room.members.get(&self.name)?.id != self.id {
            return None;
        }
        let member = room.members.remove(&self.name);
        let others = room.others(None);
        drop(room);
        drop(member);
        Some(others)
    }

    /// The orderly way out: leave the room and tell the others, unless the
    /// whole server is closing.
    async fn leave(self) {
        if let Some(others) = self.remove()
            && !self.shared.shutdown.is_cancelled()
        {
            let left = format!("* {} left\n", self.name).into();
            unless_shutdown(&self.shared, deliver(others, left)).await;
        }
    }
}

/// The way out when the task is aborted or unwinds: `Drop` cannot wait, so
/// it only frees the name. An aborted connection is not announced.
impl Drop for Membership {
    fn drop(&mut self) {
        self.remove();
    }
}

fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Writes one line. `false` if it failed or took longer than `patience`.
async fn write_line<W: AsyncWrite + Unpin>(writer: &mut W, line: &str, patience: Duration) -> bool {
    matches!(
        timeout(patience, writer.write_all(line.as_bytes())).await,
        Ok(Ok(()))
    )
}

/// Reads the client's lines until something ends the connection. Returns
/// the last line that the client should get, if any.
async fn read_lines<R: AsyncRead + Unpin>(
    lines: &mut Lines<R>,
    me: &Membership,
    kicked: &CancellationToken,
) -> Option<&'static str> {
    let idle = me.shared.limits.idle;
    // Outside the loop: only a line from the client moves the deadline.
    let mut idle_deadline = Instant::now() + idle;
    loop {
        // All three branches are cancellation safe.
        let line = tokio::select! {
            biased;
            () = me.shared.shutdown.cancelled() => return Some("* server closing\n"),
            () = kicked.cancelled() => return None,
            line = timeout_at(idle_deadline, lines.next_line()) => line,
        };
        match line {
            Err(_elapsed) => return Some("! idle\n"),
            Ok(Err(error)) if is_too_long(&error) => return Some("! line too long\n"),
            Ok(Err(_)) | Ok(Ok(None)) => return None,
            Ok(Ok(Some(line))) => {
                idle_deadline = Instant::now() + idle;
                // A line that was read is delivered to everyone it is for,
                // however long that takes. The one exception is a shutdown
                // that begins while the delivery waits for room.
                let delivery = async {
                    match line.as_str() {
                        "" => {}
                        "/who" => me.tell(me.who()).await,
                        text => me.say(text).await,
                    }
                };
                if !unless_shutdown(&me.shared, delivery).await {
                    return Some("* server closing\n");
                }
            }
        }
    }
}

/// One client, from its first byte to its last.
async fn connection<S>(io: S, shared: Arc<Shared>, _permit: OwnedSemaphorePermit)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let limits = shared.limits;
    let (read_half, mut write_half) = tokio::io::split(io);
    let mut lines = Lines::new(read_half, limits.max_line);

    let first_line = tokio::select! {
        biased;
        () = shared.shutdown.cancelled() => Err(Some("* server closing\n")),
        line = timeout(limits.idle, lines.next_line()) => match line {
            Err(_elapsed) => Err(Some("! idle\n")),
            Ok(Err(error)) if is_too_long(&error) => Err(Some("! line too long\n")),
            Ok(Err(_)) | Ok(Ok(None)) => Err(None),
            Ok(Ok(Some(name))) if valid_name(&name) => Ok(name),
            Ok(Ok(Some(_))) => Err(Some("! bad name\n")),
        },
    };
    let entered = first_line.and_then(|name| {
        Membership::enter(&shared, &name)
            .map(|parts| (name, parts))
            .ok_or(Some("! name taken\n"))
    });
    let (name, (membership, mut inbox, others)) = match entered {
        Ok(entered) => entered,
        Err(refusal) => {
            if let Some(line) = refusal {
                write_line(&mut write_half, line, limits.patience).await;
            }
            let _ = timeout(limits.patience, write_half.shutdown()).await;
            return;
        }
    };
    // Reading and writing run side by side in this one task. While the
    // reading side waits for room in another client's queue, the writing
    // side still empties our own queue; two clients that fill each other's
    // queues therefore cannot block each other for ever.
    let kicked = CancellationToken::new();
    let writing = async {
        while let Some(line) = inbox.recv().await {
            if !write_line(&mut write_half, &line, limits.patience).await {
                // Stuck or gone. Closing the queue lets every sender that
                // waits for room continue at once.
                inbox.close();
                kicked.cancel();
                return false;
            }
        }
        // Every sender is gone: the reading side has left the room.
        true
    };
    let reading = async {
        let joined = format!("* {name} joined\n").into();
        // If the shutdown interrupts this, `read_lines` sees the token in
        // its first round and returns the closing line.
        unless_shutdown(&shared, deliver(others, joined)).await;
        let farewell = read_lines(&mut lines, &membership, &kicked).await;
        // Leaving drops the senders of our queue. The writing side then
        // sends what is still queued and ends.
        membership.leave().await;
        farewell
    };
    let (farewell, still_writable) = tokio::join!(reading, writing);
    if still_writable && let Some(line) = farewell {
        write_line(&mut write_half, line, limits.patience).await;
    }
    // Flush and close our direction, so that the client sees the end.
    let _ = timeout(limits.patience, write_half.shutdown()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::guess;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
    use tokio::task::JoinHandle;
    use tokio::time::{Instant, sleep, timeout};

    const SECOND: Duration = Duration::from_secs(1);
    /// Longer than any test needs, shorter than the `idle` time of `limits()`.
    const TEST_LIMIT: Duration = Duration::from_secs(600);

    fn limits() -> Limits {
        Limits {
            max_connections: 16,
            outbox: 8,
            max_line: 256,
            idle: SECOND * 86_400,
            patience: SECOND * 3600,
            grace: SECOND * 30,
        }
    }

    /// Fails the test if `future` takes more than ten minutes of tokio time.
    /// With the paused clock that happens at once when everything is stuck.
    async fn soon<F: Future>(what: &str, future: F) -> F::Output {
        match timeout(TEST_LIMIT, future).await {
            Ok(output) => output,
            Err(_) => panic!("{what}: nothing happened for ten minutes of tokio time"),
        }
    }

    /// A server that runs on in-memory connections.
    struct Running {
        incoming: mpsc::Sender<DuplexStream>,
        shutdown: CancellationToken,
        done: JoinHandle<Report>,
    }

    fn start(limits: Limits) -> Running {
        let (incoming, listener) = mpsc::channel(64);
        let shutdown = CancellationToken::new();
        let done = tokio::spawn(serve(listener, limits, shutdown.clone()));
        Running {
            incoming,
            shutdown,
            done,
        }
    }

    impl Running {
        /// A new connection whose pipe holds `pipe` bytes in each direction.
        async fn connect(&self, pipe: usize) -> Client {
            let (client_end, server_end) = tokio::io::duplex(pipe);
            self.incoming
                .send(server_end)
                .await
                .expect("the server dropped its listener while it should be running");
            let (read_half, write_half) = tokio::io::split(client_end);
            Client {
                lines: Lines::new(read_half, 4096),
                write_half,
            }
        }

        /// Connects, sends the name, and checks the welcome line.
        async fn join(&self, name: &str) -> Client {
            self.join_with_pipe(name, 4096).await
        }

        async fn join_with_pipe(&self, name: &str, pipe: usize) -> Client {
            let mut client = self.connect(pipe).await;
            client.send(name).await;
            assert_eq!(client.line().await, format!("* welcome {name}"));
            client
        }

        /// Cancels the token and waits for `serve` to return.
        async fn stop(self) -> Report {
            self.shutdown.cancel();
            soon("waiting for serve to return after shutdown", self.done)
                .await
                .expect("serve panicked")
        }
    }

    struct Client {
        lines: Lines<ReadHalf<DuplexStream>>,
        write_half: WriteHalf<DuplexStream>,
    }

    impl Client {
        async fn send(&mut self, line: &str) {
            let bytes = format!("{line}\n");
            soon(
                "sending a line",
                self.write_half.write_all(bytes.as_bytes()),
            )
            .await
            .expect("the client could not send a line");
        }

        /// The next line, or `None` when the server has closed the connection.
        async fn next(&mut self) -> Option<String> {
            soon("waiting for a line from the server", self.lines.next_line())
                .await
                .unwrap_or(None)
        }

        async fn line(&mut self) -> String {
            self.next()
                .await
                .expect("the server closed the connection; a line was expected")
        }

        /// Everything until the server closes the connection.
        async fn rest(&mut self) -> Vec<String> {
            let mut lines = Vec::new();
            while let Some(line) = self.next().await {
                lines.push(line);
            }
            lines
        }

        /// Asks `/who` and returns the lines that arrived before the answer.
        async fn sync(&mut self) -> Vec<String> {
            self.send("/who").await;
            let mut before = Vec::new();
            loop {
                let line = self.line().await;
                if line.starts_with("* online: ") {
                    return before;
                }
                before.push(line);
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_the_protocol_of_lesson_08_still_holds() {
        let server = start(limits());
        let mut ann = server.join("ann").await;
        let mut bob = server.join("bob").await;
        assert_eq!(ann.line().await, "* bob joined");
        bob.send("hello ann").await;
        assert_eq!(ann.line().await, "bob: hello ann");
        ann.send("").await;
        ann.send("hi\r").await;
        assert_eq!(bob.line().await, "ann: hi");
        bob.send("/who").await;
        assert_eq!(bob.line().await, "* online: ann, bob");

        let mut bad = server.connect(4096).await;
        bad.send("two words").await;
        assert_eq!(bad.rest().await, ["! bad name"]);
        let mut taken = server.connect(4096).await;
        taken.send("ann").await;
        assert_eq!(taken.rest().await, ["! name taken"]);
        let mut long = server.connect(4096).await;
        long.send(&"x".repeat(257)).await;
        assert_eq!(long.rest().await, ["! line too long"]);

        drop(bob);
        assert_eq!(ann.line().await, "* bob left");
        assert!(
            ann.sync().await.is_empty(),
            "refused clients are never announced"
        );
        assert_eq!(
            server.stop().await,
            Report {
                joined: 2,
                messages: 2,
                aborted: 0
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn b_a_full_queue_slows_the_sender_and_loses_nothing() {
        const LINES: usize = 2000;
        let server = start(Limits {
            outbox: 4,
            ..limits()
        });
        // Small pipes: 64 bytes towards `slow`, 256 bytes from `talker`.
        // The server can move very little before `slow` reads.
        let mut slow = server.join_with_pipe("slow", 64).await;
        let mut talker = server.join_with_pipe("talker", 256).await;
        assert_eq!(slow.line().await, "* talker joined");

        let sent = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&sent);
        let talking = tokio::spawn(async move {
            for n in 0..LINES {
                talker.send(&format!("line {n}")).await;
                counter.fetch_add(1, Ordering::SeqCst);
            }
            talker
        });
        // `slow` reads nothing for a minute. The clock jumps there as soon
        // as every task is waiting.
        sleep(SECOND * 60).await;
        let sent_while_blocked = sent.load(Ordering::SeqCst);
        assert!(
            sent_while_blocked < 200,
            "the talker got rid of {sent_while_blocked} lines while the only receiver took none. \
             Something between them holds an unlimited number of lines: when the receiver's \
             queue is full, the sender must wait and stop reading its own client"
        );
        for n in 0..LINES {
            assert_eq!(
                slow.next().await,
                Some(format!("talker: line {n}")),
                "a slow client must get every line, in order: a full queue makes the sender \
                 wait, it does not drop the line or the client"
            );
        }
        let mut talker = soon("the talker's last lines", talking).await.unwrap();
        assert_eq!(
            sent.load(Ordering::SeqCst),
            LINES,
            "once the receiver reads, the sender continues"
        );
        talker.send("/who").await;
        assert_eq!(
            talker.line().await,
            "* online: slow, talker",
            "a client that reads within `patience` is not removed"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn b_two_busy_clients_do_not_block_each_other() {
        const LINES: usize = 300;
        let server = start(Limits {
            outbox: 2,
            ..limits()
        });
        let mut ann = server.join("ann").await;
        let mut bob = server.join("bob").await;
        assert_eq!(ann.line().await, "* bob joined");
        // Both send a burst before either reads a line. All of it fits into
        // the pipes, so the clients are not stuck; but inside the server each
        // one's queue of two lines is full while the other one still talks.
        for n in 0..LINES {
            ann.send(&format!("{n}")).await;
        }
        for n in 0..LINES {
            bob.send(&format!("{n}")).await;
        }
        for n in 0..LINES {
            assert_eq!(
                bob.line().await,
                format!("ann: {n}"),
                "both clients talk at once and neither gets its lines: a connection that \
                 waits for room in another client's queue must keep emptying its own queue"
            );
            assert_eq!(ann.line().await, format!("bob: {n}"));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn c_a_stuck_client_is_removed_after_patience() {
        let patience = SECOND * 30;
        let server = start(Limits {
            outbox: 2,
            patience,
            ..limits()
        });
        let mut stuck = server.join_with_pipe("stuck", 64).await;
        let mut reader = server.join("reader").await;
        let mut talker = server.join("talker").await;
        assert_eq!(reader.line().await, "* talker joined");
        let start = Instant::now();
        let talking = tokio::spawn(async move {
            for n in 0..50 {
                talker.send(&format!("line {n}")).await;
            }
            talker
        });
        // Whatever the order in which the server serves its clients, the
        // healthy one must get every line once `stuck` is out of the way.
        let mut seen = Vec::new();
        while seen.len() < 51 {
            seen.push(reader.line().await);
        }
        let waited = start.elapsed();
        let left = seen
            .iter()
            .position(|line| line == "* stuck left")
            .expect("the others must be told that the stuck client left");
        seen.remove(left);
        assert_eq!(
            seen,
            (0..50)
                .map(|n| format!("talker: line {n}"))
                .collect::<Vec<_>>(),
            "the healthy client gets every line in order"
        );
        assert!(
            waited >= patience,
            "the stuck client was removed after {waited:?}: a client must get `patience` \
             ({patience:?}) to take a line before it is removed"
        );
        assert!(
            waited < patience * 3,
            "removing one stuck client took {waited:?}; it should take about one `patience`"
        );
        let mut talker = soon("the talker", talking).await.unwrap();
        assert_eq!(
            talker.sync().await,
            ["* stuck left"],
            "the talker is told too, once"
        );
        talker.send("/who").await;
        assert_eq!(talker.line().await, "* online: reader, talker");
        // The stuck client's connection was closed. What fitted into its
        // pipe is still readable, and then the pipe ends.
        let leftovers = stuck.rest().await;
        assert!(
            leftovers.len() < 50,
            "the stuck client cannot have received everything"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn c_only_max_connections_are_served_at_once() {
        let server = start(Limits {
            max_connections: 2,
            ..limits()
        });
        let mut first = server.join("first").await;
        let second = server.join("second").await;
        assert_eq!(first.line().await, "* second joined");
        let mut third = server.connect(4096).await;
        third.send("third").await;
        let early = timeout(SECOND * 5, third.lines.next_line()).await;
        assert!(
            early.is_err(),
            "the third connection was served while two others were open: take a permit \
             before you accept, and hold it until the connection has ended"
        );
        assert_eq!(
            server.incoming.max_capacity() - server.incoming.capacity(),
            1,
            "connections that still wait in the listener: the server took the third \
             connection out of the listener although it cannot serve it yet. Take the \
             permit first and accept afterwards, so that the waiting happens in the listener"
        );
        drop(second);
        assert_eq!(first.line().await, "* second left");
        assert_eq!(
            third.line().await,
            "* welcome third",
            "when a connection ends, the next waiting one must be served"
        );
        assert_eq!(first.line().await, "* third joined");
        assert_eq!(server.stop().await.joined, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn c_idle_time_is_counted_from_the_clients_last_line() {
        let idle = SECOND * 60;
        let server = start(Limits { idle, ..limits() });
        let start = Instant::now();
        let mut quiet = server.join("quiet").await;
        let mut active = server.join("active").await;
        assert_eq!(quiet.line().await, "* active joined");
        // `active` says something every 40 seconds, so it is never idle for
        // a minute. `quiet` only receives; receiving is not activity.
        let talking = tokio::spawn(async move {
            for n in 0..4 {
                sleep(SECOND * 40).await;
                active.send(&format!("still here {n}")).await;
            }
            active
        });
        assert_eq!(quiet.line().await, "active: still here 0");
        assert_eq!(
            quiet.rest().await,
            ["! idle"],
            "a client that sends nothing for `idle` is told so and disconnected, \
             even while lines are delivered to it"
        );
        assert_eq!(
            start.elapsed(),
            idle,
            "the idle time of `quiet` runs from its last line (its name), exactly"
        );
        let mut active = soon("the active client", talking).await.unwrap();
        assert_eq!(active.line().await, "* quiet left");
        active.send("/who").await;
        assert_eq!(
            active.line().await,
            "* online: active",
            "a client that sends a line every 40 seconds is never idle for 60"
        );
        assert_eq!(start.elapsed(), SECOND * 160);
    }

    #[tokio::test(start_paused = true)]
    async fn d_shutdown_delivers_what_is_queued_and_says_goodbye() {
        let server = start(limits());
        let mut nameless = server.connect(4096).await;
        let mut ann = server.join("ann").await;
        let mut bob = server.join("bob").await;
        bob.send("last words").await;
        // The `/who` answer proves that the server has handled the line.
        assert!(bob.sync().await.is_empty());
        let report = server.stop().await;
        assert_eq!(
            report,
            Report {
                joined: 2,
                messages: 1,
                aborted: 0
            }
        );
        assert_eq!(
            ann.rest().await,
            ["* bob joined", "bob: last words", "* server closing"],
            "lines that were queued before the shutdown are still delivered, \
             then the closing line, and no `left` lines"
        );
        assert_eq!(bob.rest().await, ["* server closing"]);
        assert_eq!(nameless.rest().await, ["* server closing"]);
    }

    #[tokio::test(start_paused = true)]
    async fn d_shutdown_aborts_what_does_not_finish_in_time() {
        let grace = SECOND * 20;
        let server = start(Limits {
            outbox: 64,
            grace,
            ..limits()
        });
        let mut stuck = server.join_with_pipe("stuck", 64).await;
        let mut talker = server.join("talker").await;
        // More than the 64-byte pipe of `stuck` holds, less than its queue.
        for n in 0..20 {
            talker.send(&format!("some more bytes {n}")).await;
        }
        assert!(talker.sync().await.is_empty());
        let start = Instant::now();
        server.shutdown.cancel();
        // Five seconds into the grace time the server still waits for
        // `stuck`, but it must have given up its listener at the start.
        sleep(SECOND * 5).await;
        assert!(
            server.incoming.is_closed(),
            "the server still holds its listener while it waits for the connections to \
             finish: drop the listener as soon as the shutdown begins, so that nothing \
             more can be accepted"
        );
        let report = server.stop().await;
        assert_eq!(
            report.aborted, 1,
            "one connection cannot finish: its client takes no data"
        );
        assert_eq!(
            start.elapsed(),
            grace,
            "serve must wait for the grace time, and not longer"
        );
        assert_eq!(talker.rest().await, ["* server closing"]);
        let leftovers = stuck.rest().await;
        assert!(
            !leftovers.contains(&String::from("* server closing")),
            "the aborted connection was cut off; its pipe just ends"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn d_shutdown_reaches_a_client_whose_line_waits_for_room() {
        let grace = SECOND * 20;
        let server = start(Limits {
            outbox: 2,
            grace,
            ..limits()
        });
        let mut stuck = server.join_with_pipe("stuck", 64).await;
        let mut talker = server.join("talker").await;
        // More lines than the pipe and the queue of `stuck` hold together.
        // The server passes on a few and then waits for room in that queue.
        for n in 0..40 {
            talker.send(&format!("line {n}")).await;
        }
        // The clock moves only when every task waits: now the backpressure
        // has reached the reading side of the talker's connection.
        sleep(SECOND).await;
        let start = Instant::now();
        let report = server.stop().await;
        assert_eq!(
            talker.rest().await,
            ["* server closing"],
            "the talker is a healthy client, so it must get the closing line although \
             one of its lines still waited for room in the queue of a stuck client: \
             a wait for room must end when the shutdown begins"
        );
        assert_eq!(
            report.aborted, 1,
            "only the connection whose client takes no data is aborted"
        );
        assert_eq!(
            start.elapsed(),
            grace,
            "serve waits for the stuck connection until the grace time is over"
        );
        let leftovers = stuck.rest().await;
        assert!(
            !leftovers.contains(&String::from("* server closing")),
            "the aborted connection was cut off; its pipe just ends"
        );
    }

    #[tokio::test]
    async fn e_the_same_code_serves_real_sockets() {
        use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
        async fn join(
            addr: std::net::SocketAddr,
            name: &str,
        ) -> (Lines<OwnedReadHalf>, OwnedWriteHalf) {
            let stream = TcpStream::connect(addr).await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            write_half
                .write_all(format!("{name}\n").as_bytes())
                .await
                .unwrap();
            let mut lines = Lines::new(read_half, 4096);
            assert_eq!(
                lines.next_line().await.unwrap(),
                Some(format!("* welcome {name}"))
            );
            (lines, write_half)
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let done = tokio::spawn(serve(listener, limits(), shutdown.clone()));
        let run = async {
            let (mut ann, _ann_write) = join(addr, "ann").await;
            let (mut bob, mut bob_write) = join(addr, "bob").await;
            assert_eq!(
                ann.next_line().await.unwrap().as_deref(),
                Some("* bob joined")
            );
            bob_write.write_all(b"over tcp\n").await.unwrap();
            assert_eq!(
                ann.next_line().await.unwrap().as_deref(),
                Some("bob: over tcp")
            );
            shutdown.cancel();
            for client in [&mut ann, &mut bob] {
                assert_eq!(
                    client.next_line().await.unwrap().as_deref(),
                    Some("* server closing")
                );
                assert_eq!(client.next_line().await.unwrap(), None);
            }
            done.await.expect("serve panicked")
        };
        // Real time here: a limit of ten seconds on the wall clock.
        let report = timeout(SECOND * 10, run)
            .await
            .expect("the TCP test did not finish within 10 s");
        assert_eq!(
            report,
            Report {
                joined: 2,
                messages: 1,
                aborted: 0
            }
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test(start_paused = true)]
    async fn p_predictions() {
        // P1. A channel with room for two values is offered five with
        // `try_send`, and nobody receives. How many are accepted?
        let (tx, mut rx) = mpsc::channel(2);
        let accepted = (0..5).filter(|n| tx.try_send(*n).is_ok()).count();
        let p1: usize = 2;
        assert_eq!(p1, accepted, "P1: try_send into a bounded channel");

        // P2. The channel is still full. A task now calls `send(9).await`.
        // The test takes ONE value out and then lets the task run. Which
        // values does the channel hold afterwards, in order?
        let sender = tokio::spawn(async move { tx.send(9).await.is_ok() });
        let first = rx.recv().await;
        assert!(sender.await.unwrap());
        let mut left = Vec::new();
        while let Ok(value) = rx.try_recv() {
            left.push(value);
        }
        let p2: (Option<i32>, [i32; 2]) = (Some(0), [1, 9]);
        assert_eq!(
            (p2.0, p2.1.as_slice()),
            (first, left.as_slice()),
            "P2: (the value that was taken out, what the channel held afterwards)"
        );

        // P3. The clock is paused. A future that never finishes gets a
        // timeout of 90 seconds. How many seconds of tokio time have passed
        // when the timeout returns, and is the result an error?
        let start = Instant::now();
        let result = timeout(SECOND * 90, std::future::pending::<()>()).await;
        let p3: (u64, bool) = (90, true);
        assert_eq!(
            p3,
            (start.elapsed().as_secs(), result.is_err()),
            "P3: (seconds passed, the timeout fired)"
        );

        // P4. A child token is made from a parent, and a grandchild from
        // the child. The CHILD is cancelled. Which of the three are
        // cancelled now: (parent, child, grandchild)?
        let parent = CancellationToken::new();
        let child = parent.child_token();
        let grandchild = child.child_token();
        child.cancel();
        let p4: (bool, bool, bool) = (false, true, true);
        assert_eq!(
            p4,
            (
                parent.is_cancelled(),
                child.is_cancelled(),
                grandchild.is_cancelled()
            ),
            "P4: cancellation goes down the tree, never up"
        );
    }
}
