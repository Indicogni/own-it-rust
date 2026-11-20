//! LESSON 08 — Capstone: a chat server on your own runtime
//! Read first: CONCEPTS.md, "Lesson 08". Start after: lessons 02 to 07, all
//! of them. This lesson has no hidden parts: it runs on the code you wrote.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l08_
//!
//! BEFORE CODING, write in NOTES.md: which tasks exist while three clients are
//! connected; what each task owns; every place where a task can wait, and
//! what wakes it there; what `Drop` must clean up when a task is cancelled.
//!
//! IMPLEMENT the contract below. Only the public names are fixed. Use your
//! executor (lesson 02), timers (03), reactor and sockets (04), and whatever
//! you like from lessons 05 to 07. Do not start threads of your own: the
//! server runs on the thread that calls `run`, plus the two driver threads
//! that your timer driver and your reactor start.
//! EXPLAIN: Pick one client that stops reading while others keep talking.
//! Follow one message to that client through every queue and buffer between
//! the sender's socket and the slow client's socket. Say where it waits,
//! what bounds each queue, and what your server does when a bound is hit.
//!
//! # The protocol
//!
//! Text lines over TCP. Each line ends with `\n`; a `\r` before it is
//! ignored. The server finishes one line before it starts the next. Only a
//! client that is cut off (rules 6 and 9) can see half a line.
//!
//! 1. **Name.** The first line of a client is its name: 1 to 16 characters
//!    out of ASCII letters, digits, `_` and `-`. For another first line the
//!    server sends `! bad name` and closes the connection. If a connected
//!    client has the name already, it sends `! name taken` and closes.
//! 2. **Joining.** The new client gets `* welcome NAME`. Every other client
//!    gets `* NAME joined`.
//! 3. **Talking.** Each later line `TEXT` goes to every *other* client as
//!    `NAME: TEXT`. An empty line is ignored. The line `/who` is answered,
//!    to the sender only, with `* online: A, B, C`: all names, sorted.
//! 4. **Leaving.** When a client goes away for any reason, its name is free
//!    again, and every other client gets `* NAME left`, once. (While the
//!    server stops, rule 9 says what a client still gets.)
//! 5. **Limits.** A line longer than `max_line` bytes: `! line too long`,
//!    then the connection is closed. No complete line for `idle_timeout`
//!    (counted from the previous line, or from connecting): `! idle`, then
//!    the connection is closed.
//! 6. **Slow clients.** At most `outbox` lines wait for one client. A
//!    client whose queue is full when a new line is due is removed from the
//!    room (rule 4 applies) and its connection is closed, at the latest
//!    after one more `idle_timeout`.
//! 7. **Fairness.** A client that sends many lines at once must not keep the
//!    others waiting: after each line the server lets other tasks run.
//! 8. **Order.** Each client receives its lines in the order in which the
//!    server produced them. Lines from one sender keep their order.
//! 9. **Stopping.** After `Stopper::stop`, the server accepts no new
//!    connection, sends `* server closing` to every client, and closes each
//!    connection. For a client this line is the last one: lines that still
//!    wait in its queue are dropped, so it gets no `left` line for a client
//!    that was closed a moment before it. `run` returns when all connections
//!    are closed or `grace` has passed, whichever comes first. Nothing is
//!    left running afterwards.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use crate::l02_tasks::{Executor, JoinHandle, Spawner};
use crate::l03_timers::{TimerDriver, Timers};
use crate::l04_reactor::{AsyncListener, AsyncStream, Reactor};
use crate::l05_projection::{Elapsed, Timeout};
use crate::l06_streams::{Receiver, Sender, Stream, StreamExt, channel};
use crate::l07_select::{CancelToken, Either, LineReader, TooLong, select};
use crate::support::yield_now;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::net::TcpListener;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, ready};
use std::time::Instant;

/// The limits of a server (rules 5, 6 and 9). `outbox` must be at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatConfig {
    pub idle_timeout: Duration,
    pub outbox: usize,
    pub max_line: usize,
    pub grace: Duration,
}

impl Default for ChatConfig {
    fn default() -> Self {
        ChatConfig {
            idle_timeout: Duration::from_secs(300),
            outbox: 64,
            max_line: 1024,
            grace: Duration::from_secs(1),
        }
    }
}

/// What a server did, returned by [`ChatServer::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Clients that were welcomed (rule 2).
    pub joined: u64,
    /// Chat lines that were passed on (rule 3), counted once per line, not
    /// once per receiver. `/who` and empty lines do not count.
    pub messages: u64,
}

/// A chat server that is bound to a port and not yet running.
pub struct ChatServer {
    listener: TcpListener,
    addr: SocketAddr,
    config: ChatConfig,
    stop: CancelToken,
}

impl ChatServer {
    /// Binds a listener to `127.0.0.1` on a port that the system chooses.
    /// Clients can connect from now on; they are served once `run` is called.
    pub fn bind(config: ChatConfig) -> io::Result<ChatServer> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        Ok(ChatServer {
            listener,
            addr,
            config,
            stop: CancelToken::new(),
        })
    }

    /// The address that clients connect to.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// A handle that stops the server from any thread.
    pub fn stopper(&self) -> Stopper {
        Stopper {
            stop: self.stop.clone(),
        }
    }

    /// Serves clients on the calling thread until the server is stopped
    /// (rule 9), then returns the summary.
    pub fn run(self) -> io::Result<Summary> {
        let reactor = Reactor::start();
        let timers = TimerDriver::start();
        let mut executor = Executor::new();
        let listener = AsyncListener::new(self.listener, &reactor.handle())?;
        let shared = Arc::new(Shared {
            config: self.config,
            timers: timers.handle(),
            stop: self.stop,
            room: Mutex::new(Room::default()),
        });
        let spawner = executor.spawner();
        executor.block_on(serve(listener, spawner, Arc::clone(&shared)));
        // The order matters: dropping the executor cancels every task that
        // is left, and with it every socket and every pending timer. Only
        // then may the drivers stop.
        drop(executor);
        drop(timers);
        drop(reactor);
        let room = lock(&shared.room);
        Ok(Summary {
            joined: room.joined,
            messages: room.messages,
        })
    }
}

/// Stops a running [`ChatServer`]. It can be cloned and sent to any thread.
#[derive(Clone)]
pub struct Stopper {
    stop: CancelToken,
}

impl Stopper {
    /// Starts rule 9. Calling it again does nothing.
    pub fn stop(&self) {
        self.stop.cancel();
    }
}

/// How long the accept loop pauses after a failed `accept`. Errors such as
/// "too many open files" do not go away at once; without a pause the loop
/// would spin.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// Locks a mutex and ignores poisoning (see lesson 02).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One line on its way to one or more clients. Shared, so that a line for
/// fifty clients is one allocation.
type Line = Arc<str>;

/// Where a line goes: name, membership id and queue of each receiver.
type Targets = Vec<(String, u64, Sender<Line>)>;

/// What every task of one server shares.
struct Shared {
    config: ChatConfig,
    timers: Timers,
    stop: CancelToken,
    /// Locked only for short, synchronous sections: never across an await,
    /// and never while a line is sent or a waker is woken.
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
    outbox: Sender<Line>,
}

impl Room {
    /// The outbox of every member except `skip`.
    fn others(&self, skip: Option<&str>) -> Targets {
        self.members
            .iter()
            .filter(|(name, _)| Some(name.as_str()) != skip)
            .map(|(name, member)| (name.clone(), member.id, member.outbox.clone()))
            .collect()
    }
}

/// Queues `line` for each target. A target whose outbox is full or closed
/// is removed from the room (rule 6), and its `left` line goes to the rest
/// in the same way. The room lock is not held while lines are queued,
/// because queueing wakes the receiving task.
fn deliver(shared: &Shared, targets: Targets, line: Line) {
    let mut work = VecDeque::from([(targets, line)]);
    while let Some((targets, line)) = work.pop_front() {
        for (name, id, outbox) in targets {
            if outbox.try_send(Arc::clone(&line)).is_err() {
                work.extend(remove(shared, &name, id));
            }
        }
    }
}

/// Takes one membership out of the room. Returns the `left` line and its
/// receivers, or `None` if that membership was removed before.
fn remove(shared: &Shared, name: &str, id: u64) -> Option<(Targets, Line)> {
    let mut room = lock(&shared.room);
    if room.members.get(name)?.id != id {
        return None;
    }
    let member = room.members.remove(name);
    let others = room.others(None);
    drop(room);
    // Dropping the last sender wakes the member's task; do it unlocked.
    drop(member);
    Some((others, format!("* {name} left\n").into()))
}

/// A client's place in the room. Dropping it is rule 4, on every path:
/// a normal return, an error, and the cancellation of the task.
struct Membership {
    shared: Arc<Shared>,
    name: String,
    id: u64,
}

impl Membership {
    /// Rule 1 and 2: enters the room unless the name is taken.
    fn join(shared: &Arc<Shared>, name: &str) -> Option<(Membership, Receiver<Line>)> {
        let (outbox, inbox) = channel(shared.config.outbox);
        let mut room = lock(&shared.room);
        if room.members.contains_key(name) {
            return None;
        }
        let id = room.next_id;
        room.next_id += 1;
        room.joined += 1;
        let others = room.others(None);
        room.members.insert(name.to_owned(), Member { id, outbox });
        drop(room);
        deliver(shared, others, format!("* {name} joined\n").into());
        let membership = Membership {
            shared: Arc::clone(shared),
            name: name.to_owned(),
            id,
        };
        Some((membership, inbox))
    }

    /// Rule 3: a chat line for everyone else.
    fn say(&self, text: &str) {
        let others = {
            let mut room = lock(&self.shared.room);
            room.messages += 1;
            room.others(Some(&self.name))
        };
        deliver(
            &self.shared,
            others,
            format!("{}: {text}\n", self.name).into(),
        );
    }

    /// Rule 3: the answer to `/who`. It goes into this client's own queue
    /// and not straight to the socket: written directly, it could overtake
    /// lines that were queued for the client earlier (rule 8). Rule 6 then
    /// covers it too: a client whose queue is full is removed.
    fn who(&self) {
        let (me, answer) = {
            let room = lock(&self.shared.room);
            let names: Vec<&str> = room.members.keys().map(String::as_str).collect();
            let answer = format!("* online: {}\n", names.join(", "));
            // No entry with our id means that rule 6 has removed us; the
            // task then ends as soon as it sees its queue closed.
            let me = room
                .members
                .get(&self.name)
                .filter(|member| member.id == self.id)
                .map(|member| (self.name.clone(), member.id, member.outbox.clone()));
            (me, answer)
        };
        deliver(&self.shared, me.into_iter().collect(), answer.into());
    }
}

impl Drop for Membership {
    fn drop(&mut self) {
        if let Some((others, line)) = remove(&self.shared, &self.name, self.id) {
            deliver(&self.shared, others, line);
        }
    }
}

/// The bytes of one connection as a stream of chunks, for `LineReader`.
struct Chunks<'a> {
    stream: &'a AsyncStream,
}

impl Stream for Chunks<'_> {
    type Item = Vec<u8>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Vec<u8>>> {
        let mut buffer = [0u8; 2048];
        match ready!(self.stream.poll_read(cx, &mut buffer)) {
            Ok(count) if count > 0 => Poll::Ready(Some(buffer[..count].to_vec())),
            // The end of the stream, or an error that ends it for us.
            _ => Poll::Ready(None),
        }
    }
}

fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Writes one line. Gives up, with `false`, on an error or when the client
/// does not take the line within the idle time.
async fn send(shared: &Shared, stream: &AsyncStream, line: &str) -> bool {
    let write = stream.write_all(line.as_bytes());
    let patience = shared.timers.sleep(shared.config.idle_timeout);
    matches!(Timeout::new(write, patience).await, Ok(Ok(())))
}

/// Runs both futures until one finishes. `second_first` says which one is
/// polled first; the caller flips it, so that neither side starves.
async fn race<T>(
    first: impl Future<Output = T>,
    second: impl Future<Output = T>,
    second_first: bool,
) -> T {
    if second_first {
        match select(second, first).await {
            Either::Left(output) | Either::Right(output) => output,
        }
    } else {
        match select(first, second).await {
            Either::Left(output) | Either::Right(output) => output,
        }
    }
}

/// What a connection task can wake up for.
enum Event {
    Stop,
    Line(Result<Option<String>, TooLong>),
    Idle,
    Deliver(Option<Line>),
}

/// The task of one client, from its first byte to its last.
async fn connection(stream: AsyncStream, shared: Arc<Shared>) {
    let idle = shared.config.idle_timeout;
    let mut lines = LineReader::new(Chunks { stream: &stream }, shared.config.max_line);

    // Rule 1. The stop signal also ends a client that has not named itself.
    let first_line = Timeout::new(lines.next_line(), shared.timers.sleep(idle));
    let name = match select(shared.stop.cancelled(), first_line).await {
        Either::Left(()) => {
            send(&shared, &stream, "* server closing\n").await;
            return;
        }
        Either::Right(Ok(Ok(Some(name)))) if valid_name(&name) => name,
        Either::Right(Ok(Ok(Some(_)))) => {
            send(&shared, &stream, "! bad name\n").await;
            return;
        }
        Either::Right(Ok(Ok(None))) => return,
        Either::Right(Ok(Err(TooLong))) => {
            send(&shared, &stream, "! line too long\n").await;
            return;
        }
        Either::Right(Err(Elapsed)) => {
            send(&shared, &stream, "! idle\n").await;
            return;
        }
    };
    let Some((membership, mut inbox)) = Membership::join(&shared, &name) else {
        send(&shared, &stream, "! name taken\n").await;
        return;
    };
    // From here on `membership` is alive, so rule 4 holds on every exit.
    if !send(&shared, &stream, &format!("* welcome {name}\n")).await {
        return;
    }

    // The deadline lives outside the loop body: a delivery must not give the
    // client more idle time, and a lost race must not restart the clock.
    let mut idle_deadline = Instant::now() + idle;
    let mut inbox_first = false;
    loop {
        let input = async {
            let line = Timeout::new(lines.next_line(), shared.timers.sleep_until(idle_deadline));
            match line.await {
                Ok(line) => Event::Line(line),
                Err(Elapsed) => Event::Idle,
            }
        };
        let delivery = async { Event::Deliver(inbox.next().await) };
        // Every branch is cancellation safe: `next_line` keeps its bytes in
        // `lines`, `next` takes an item only when it returns it, and the
        // wait for the stop signal owns nothing.
        let event = match select(shared.stop.cancelled(), race(input, delivery, inbox_first)).await
        {
            Either::Left(()) => Event::Stop,
            Either::Right(event) => event,
        };
        inbox_first = !inbox_first;
        let farewell = match event {
            Event::Stop => "* server closing\n",
            Event::Idle => "! idle\n",
            Event::Line(Err(TooLong)) => "! line too long\n",
            Event::Line(Ok(None)) | Event::Deliver(None) => return,
            Event::Line(Ok(Some(line))) => {
                idle_deadline = Instant::now() + idle;
                match line.as_str() {
                    "" => {}
                    "/who" => membership.who(),
                    text => membership.say(text),
                }
                // Rule 7: the next line may already be in the buffer, and
                // then nothing in this loop would ever return Pending.
                yield_now().await;
                continue;
            }
            Event::Deliver(Some(line)) => {
                if !send(&shared, &stream, &line).await {
                    return;
                }
                continue;
            }
        };
        send(&shared, &stream, farewell).await;
        return;
    }
}

/// The accept loop, and rule 9.
async fn serve(listener: AsyncListener, spawner: Spawner, shared: Arc<Shared>) {
    let mut connections: Vec<JoinHandle<()>> = Vec::new();
    loop {
        // `accept` is cancellation safe: a connection is taken from the
        // listener only in the poll that returns it.
        match select(shared.stop.cancelled(), listener.accept()).await {
            Either::Left(()) => break,
            Either::Right(Ok((stream, _peer))) => {
                connections.retain(|connection| !connection.is_finished());
                connections.push(spawner.spawn(connection(stream, Arc::clone(&shared))));
            }
            Either::Right(Err(_)) => {
                let pause = shared.timers.sleep(ACCEPT_BACKOFF);
                select(shared.stop.cancelled(), pause).await;
            }
        }
    }
    // Close the listening socket now: no new client can connect while the
    // old ones say goodbye.
    drop(listener);
    let all_closed = async {
        for connection in connections {
            let _ = connection.await;
        }
    };
    let _ = Timeout::new(all_closed, shared.timers.sleep(shared.config.grace)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    const LIMIT: Duration = Duration::from_secs(10);

    fn config() -> ChatConfig {
        ChatConfig {
            idle_timeout: Duration::from_secs(60),
            outbox: 1024,
            max_line: 256,
            grace: Duration::from_secs(2),
        }
    }

    /// A running server. Dropping it stops the server, so that a failed test
    /// does not leave one behind.
    struct Running {
        addr: SocketAddr,
        stopper: Stopper,
        done: mpsc::Receiver<io::Result<Summary>>,
    }

    fn start(config: ChatConfig) -> Running {
        let server = ChatServer::bind(config).expect("ChatServer::bind failed");
        let addr = server.local_addr();
        let stopper = server.stopper();
        let (done_tx, done) = mpsc::channel();
        thread::spawn(move || {
            let _ = done_tx.send(server.run());
        });
        Running {
            addr,
            stopper,
            done,
        }
    }

    impl Running {
        /// Stops the server and waits for `run` to return.
        fn stop(&self) -> Summary {
            self.stopper.stop();
            self.done
                .recv_timeout(LIMIT)
                .expect("`run` did not return within 10 s after `stop` (or it panicked)")
                .expect("`run` returned an error")
        }

        fn client(&self) -> Client {
            let stream = TcpStream::connect(self.addr).expect("cannot connect to the server");
            stream.set_read_timeout(Some(LIMIT)).unwrap();
            stream.set_write_timeout(Some(LIMIT)).unwrap();
            Client {
                reader: BufReader::new(stream.try_clone().unwrap()),
                stream,
            }
        }

        /// Connects, sends the name, and checks the welcome line.
        fn join(&self, name: &str) -> Client {
            let mut client = self.client();
            client.send(name);
            assert_eq!(client.line(), format!("* welcome {name}"));
            client
        }
    }

    impl Drop for Running {
        fn drop(&mut self) {
            self.stopper.stop();
        }
    }

    /// A blocking test client. Every read gives up after 10 seconds.
    struct Client {
        reader: BufReader<TcpStream>,
        stream: TcpStream,
    }

    impl Client {
        fn send(&mut self, line: &str) {
            self.stream
                .write_all(format!("{line}\n").as_bytes())
                .expect("the client could not send a line");
        }

        /// The next line, or `None` at the end of the connection.
        fn next(&mut self) -> Option<String> {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => None,
                Ok(_) => {
                    assert!(
                        line.ends_with('\n'),
                        "the server sent an unfinished line: {line:?}"
                    );
                    line.pop();
                    Some(line)
                }
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    panic!("the client waited 10 s for a line from the server")
                }
                // A connection that the server closed hard counts as closed.
                Err(_) => None,
            }
        }

        fn line(&mut self) -> String {
            self.next()
                .expect("the server closed the connection; a line was expected")
        }

        /// Everything the server sends until it closes the connection.
        fn rest(&mut self) -> Vec<String> {
            std::iter::from_fn(|| self.next()).collect()
        }

        /// Asks `/who`, and returns the lines that arrived before the answer.
        /// The answer proves that the server has handled every line that
        /// this client sent before.
        fn sync(&mut self) -> Vec<String> {
            self.send("/who");
            self.until_answer().0
        }

        /// Reads up to the answer of a `/who` that was sent already. Returns
        /// the lines that arrived before the answer, and the answer.
        fn until_answer(&mut self) -> (Vec<String>, String) {
            let mut before = Vec::new();
            loop {
                let line = self.line();
                if line.starts_with("* online: ") {
                    return (before, line);
                }
                before.push(line);
            }
        }
    }

    #[test]
    fn a_clients_join_talk_and_leave() {
        let server = start(config());
        let mut ann = server.join("ann");
        let mut bob = server.join("bob");
        assert_eq!(
            ann.line(),
            "* bob joined",
            "rule 2: the others learn of a new client"
        );
        bob.send("hello ann");
        assert_eq!(ann.line(), "bob: hello ann", "rule 3");
        ann.send("hi");
        ann.send("");
        ann.send("how are you?\r");
        assert_eq!(bob.line(), "ann: hi");
        assert_eq!(
            bob.line(),
            "ann: how are you?",
            "an empty line is ignored and a `\\r` before the newline is dropped"
        );
        assert!(
            bob.sync().is_empty(),
            "rule 3: the sender does not receive its own lines"
        );
        drop(bob);
        assert_eq!(ann.line(), "* bob left", "rule 4");
        assert_eq!(
            server.stop(),
            Summary {
                joined: 2,
                messages: 3
            },
            "two clients were welcomed and three chat lines were passed on"
        );
    }

    #[test]
    fn a_who_lists_the_names_in_order() {
        let server = start(config());
        let mut zoe = server.join("zoe");
        let mut ann = server.join("ann");
        let mut max = server.join("Max_2");
        max.send("/who");
        assert_eq!(
            max.line(),
            "* online: Max_2, ann, zoe",
            "all names, sorted as strings, separated by comma and space"
        );
        assert_eq!(
            zoe.sync(),
            ["* ann joined", "* Max_2 joined"],
            "`/who` is answered to the sender only"
        );
        drop(zoe);
        assert_eq!(ann.line(), "* Max_2 joined");
        assert_eq!(ann.line(), "* zoe left");
        ann.send("/who");
        assert_eq!(ann.line(), "* online: Max_2, ann");
        assert_eq!(server.stop().messages, 0, "`/who` is not a chat line");
    }

    #[test]
    fn a_bad_names_and_taken_names_are_refused() {
        let server = start(config());
        let mut ann = server.join("ann");
        for bad in ["", "two words", "seventeen-letters", "caf\u{e9}", "/who"] {
            let mut client = server.client();
            client.send(bad);
            assert_eq!(
                client.rest(),
                ["! bad name"],
                "the name {bad:?} must be refused, and the connection closed"
            );
        }
        let mut second = server.client();
        second.send("ann");
        assert_eq!(second.rest(), ["! name taken"]);
        let mut sixteen = server.join("sixteen-letters_");
        assert_eq!(ann.line(), "* sixteen-letters_ joined");
        // A name is free again when its owner has left.
        drop(sixteen.stream.shutdown(Shutdown::Both));
        assert_eq!(ann.line(), "* sixteen-letters_ left");
        let _again = server.join("sixteen-letters_");
        assert_eq!(
            ann.sync(),
            ["* sixteen-letters_ joined"],
            "refused clients never joined, so nobody hears of them"
        );
        assert_eq!(server.stop().joined, 3, "refused clients do not count");
    }

    #[test]
    fn b_every_client_gets_every_line_in_order() {
        const CLIENTS: usize = 6;
        const LINES: usize = 40;
        let server = start(config());
        let mut clients: Vec<Client> = (0..CLIENTS)
            .map(|i| server.join(&format!("c{i}")))
            .collect();
        // Each client first reads the `joined` lines of those after it.
        for (i, client) in clients.iter_mut().enumerate() {
            for later in i + 1..CLIENTS {
                assert_eq!(client.line(), format!("* c{later} joined"));
            }
        }
        let talkers: Vec<_> = clients
            .into_iter()
            .enumerate()
            .map(|(i, mut client)| {
                thread::spawn(move || {
                    for n in 0..LINES {
                        client.send(&format!("{n}"));
                    }
                    // Read until every line of every other client is here.
                    let mut seen = vec![Vec::new(); CLIENTS];
                    let mut missing = (CLIENTS - 1) * LINES;
                    while missing > 0 {
                        let line = client.line();
                        let (name, text) = line
                            .split_once(": ")
                            .unwrap_or_else(|| panic!("unexpected line {line:?}"));
                        let from: usize = name[1..].parse().unwrap();
                        assert_ne!(from, i, "a client must not get its own line");
                        seen[from].push(text.parse::<usize>().unwrap());
                        missing -= 1;
                    }
                    (i, seen, client)
                })
            })
            .collect();
        let mut kept = Vec::new();
        for talker in talkers {
            let (i, seen, client) = talker.join().expect("a client thread failed");
            for (from, numbers) in seen.iter().enumerate() {
                if from != i {
                    assert_eq!(
                        *numbers,
                        (0..LINES).collect::<Vec<_>>(),
                        "client c{i} must get the lines of c{from} once each and in order"
                    );
                }
            }
            kept.push(client);
        }
        assert_eq!(
            server.stop(),
            Summary {
                joined: CLIENTS as u64,
                messages: (CLIENTS * LINES) as u64
            }
        );
    }

    #[test]
    fn b_a_burst_from_one_client_does_not_push_out_another() {
        const LINES: usize = 500;
        let server = start(ChatConfig {
            outbox: 4,
            ..config()
        });
        let mut reader = server.join("reader");
        let mut talker = server.join("talker");
        assert_eq!(reader.line(), "* talker joined");
        // One write with 500 lines: the server finds them all in its buffer.
        let burst: String = (0..LINES).map(|n| format!("line {n}\n")).collect();
        talker.stream.write_all(burst.as_bytes()).unwrap();
        for n in 0..LINES {
            assert_eq!(
                reader.next(),
                Some(format!("talker: line {n}")),
                "rule 7: `reader` takes its lines at once, so its queue of 4 never fills, \
                 unless the talker's task keeps the thread for the whole burst; \
                 let other tasks run after each line"
            );
        }
    }

    #[test]
    fn b_a_silent_client_does_not_hold_up_the_others() {
        let server = start(config());
        let _silent = server.join("silent");
        let mut ann = server.join("ann");
        let mut bob = server.join("bob");
        assert_eq!(ann.line(), "* bob joined");
        for n in 0..50 {
            ann.send(&format!("ping {n}"));
            assert_eq!(bob.line(), format!("ann: ping {n}"));
            bob.send(&format!("pong {n}"));
            assert_eq!(ann.line(), format!("bob: pong {n}"));
        }
    }

    #[test]
    fn c_a_client_that_never_reads_is_removed() {
        let server = start(ChatConfig {
            outbox: 8,
            grace: Duration::from_millis(200),
            ..config()
        });
        let _stuck = server.join("stuck");
        let mut talker = server.join("talker");
        // One round is 500 lines and then `/who`, in a single write, so
        // that the server finds the question right behind the lines.
        let mut round = format!("{}\n", "x".repeat(200)).repeat(500);
        round.push_str("/who\n");
        // `stuck` never reads. First the kernel's buffers fill, then its
        // queue of 8 lines, and then rule 6 applies.
        let mut rounds = 0;
        let removed = loop {
            rounds += 1;
            talker.stream.write_all(round.as_bytes()).unwrap();
            let (before, answer) = talker.until_answer();
            if answer == "* online: talker" {
                assert!(
                    before.contains(&String::from("* stuck left")),
                    "rule 8: the answer to `/who` shows that `stuck` is gone, but `* stuck left` \
                     has not arrived. That line was produced first, while one of the talker's \
                     own lines was handled, so it must arrive first: the answer may not \
                     overtake lines that already wait in the client's queue"
                );
                break true;
            }
            if rounds == 400 {
                break false;
            }
        };
        assert!(
            removed,
            "40 MB were sent to a client that reads nothing, and it is still in the room: \
             the queue for one client must be bounded (rule 6)"
        );
        talker.send("/who");
        assert_eq!(talker.line(), "* online: talker");
    }

    #[test]
    fn c_a_line_that_is_too_long_ends_the_connection() {
        let server = start(config());
        let mut ann = server.join("ann");
        let mut bob = server.join("bob");
        assert_eq!(ann.line(), "* bob joined");
        bob.send(&"x".repeat(256));
        assert_eq!(
            ann.line(),
            format!("bob: {}", "x".repeat(256)),
            "a line of exactly max_line bytes is allowed"
        );
        bob.send(&"y".repeat(257));
        assert_eq!(
            bob.rest(),
            ["! line too long"],
            "rule 5: one byte more than max_line ends the connection"
        );
        assert_eq!(ann.line(), "* bob left");

        // The limit also holds for the name, and without any newline.
        let mut flood = server.client();
        flood.stream.write_all(&[b'z'; 1000]).unwrap();
        assert_eq!(flood.rest(), ["! line too long"]);
    }

    #[test]
    fn c_an_idle_client_is_disconnected() {
        let idle = Duration::from_millis(300);
        let server = start(ChatConfig {
            idle_timeout: idle,
            ..config()
        });
        let start = Instant::now();
        let mut quiet = server.join("quiet");
        assert_eq!(
            quiet.rest(),
            ["! idle"],
            "rule 5: a client that sends nothing gets `! idle` and is disconnected"
        );
        assert!(
            start.elapsed() >= idle,
            "the client was disconnected after {:?}, before the idle time was over",
            start.elapsed()
        );

        // The same clock runs for a client that has not sent its name.
        let start = Instant::now();
        let mut nameless = server.client();
        assert_eq!(nameless.rest(), ["! idle"]);
        assert!(start.elapsed() >= idle);
        assert_eq!(server.stop().joined, 1);
    }

    #[test]
    fn c_lines_from_others_do_not_reset_the_idle_clock() {
        const LINES: usize = 100;
        let server = start(ChatConfig {
            idle_timeout: Duration::from_millis(300),
            ..config()
        });
        let mut quiet = server.join("quiet");
        let mut talker = server.join("talker");
        assert_eq!(quiet.line(), "* talker joined");
        // The talker says something every 20 ms, for two seconds at most:
        // it is never idle itself, and `quiet` gets a line every 20 ms.
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let talking = thread::spawn(move || {
            for n in 0..LINES {
                talker.send(&format!("{n}"));
                if done_rx.recv_timeout(Duration::from_millis(20)).is_ok() {
                    break;
                }
            }
            talker
        });
        let received = quiet.rest();
        let _ = done_tx.send(());
        let _talker = talking.join().expect("the talking thread failed");
        assert_eq!(
            received.last().map(String::as_str),
            Some("! idle"),
            "rule 5: a client that sends nothing is told `! idle` and disconnected"
        );
        assert!(
            received.len() <= LINES,
            "`quiet` sent nothing, and it was served until the talker had said all of its \
             {LINES} lines, two seconds later: a line from somebody else is not activity. \
             Keep the idle deadline in a variable outside the loop, so that only the \
             client's own lines move it"
        );
    }

    #[test]
    fn d_stop_says_goodbye_to_everyone_and_closes() {
        let server = start(config());
        // Connections are accepted in the order in which they were made, so
        // this one has its task before `ann` is welcomed.
        let mut nameless = server.client();
        let mut ann = server.join("ann");
        let mut bob = server.join("bob");
        assert_eq!(ann.line(), "* bob joined");
        bob.send("last words");
        assert_eq!(ann.line(), "bob: last words");
        let summary = server.stop();
        assert_eq!(
            summary,
            Summary {
                joined: 2,
                messages: 1
            }
        );
        for (who, client) in [
            ("ann", &mut ann),
            ("bob", &mut bob),
            ("nameless", &mut nameless),
        ] {
            assert_eq!(
                client.rest(),
                ["* server closing"],
                "rule 9: after `stop`, {who} must get `* server closing` and nothing else \
                 (lines that still wait in a queue are dropped, also the `left` lines of \
                 clients that were closed a moment earlier), and then the connection ends"
            );
        }
    }

    #[test]
    fn d_stop_before_run_and_stop_twice_are_fine() {
        let server = ChatServer::bind(config()).expect("ChatServer::bind failed");
        let stopper = server.stopper();
        stopper.stop();
        stopper.clone().stop();
        let (done_tx, done) = mpsc::channel();
        thread::spawn(move || {
            let _ = done_tx.send(server.run());
        });
        let summary = done
            .recv_timeout(LIMIT)
            .expect("`run` must return at once when the server was stopped before")
            .expect("`run` returned an error");
        assert_eq!(
            summary,
            Summary {
                joined: 0,
                messages: 0
            }
        );
    }

    #[test]
    fn d_stop_does_not_wait_longer_than_the_grace_time() {
        let grace = Duration::from_millis(300);
        let server = start(ChatConfig {
            outbox: 100_000,
            max_line: 1024,
            grace,
            ..config()
        });
        let mut stuck = server.join("stuck");
        let mut talker = server.join("talker");
        // 32 MB for a client that reads nothing. That is more than the
        // kernel's buffers hold, so the task that serves `stuck` ends up in
        // the middle of a write that cannot finish. Its queue is large
        // enough to keep it in the room.
        let line = "x".repeat(1000);
        for _ in 0..32 {
            for _ in 0..1000 {
                talker.send(&line);
            }
            assert!(
                talker.sync().is_empty(),
                "with a queue of 100 000 lines nobody is removed here"
            );
        }
        let start = Instant::now();
        server.stop();
        let waited = start.elapsed();
        assert!(
            waited < grace + Duration::from_secs(2),
            "`run` returned {waited:?} after `stop`, with a grace time of {grace:?}: \
             it must give up on a client that takes no data once the grace time is over"
        );
        assert_eq!(talker.rest(), ["* server closing"]);
        // Whatever was in flight to `stuck` may arrive, but the connection
        // must end: `run` has returned, so its task and its socket are gone.
        let mut sink = Vec::new();
        let _ = stuck.stream.read_to_end(&mut sink);
    }

    #[test]
    fn e_the_server_runs_on_your_runtime() {
        let source = include_str!("l08_capstone_chat.rs");
        let server_code = source
            .split("#[cfg(test)]")
            .next()
            .expect("the file has a part before its tests");
        assert!(
            !server_code.contains(concat!("todo", "!(")),
            "the server still contains a placeholder that panics; replace every one"
        );
        let code_lines = server_code
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"));
        for line in code_lines {
            assert!(
                !line.contains("thread::"),
                "this line uses `std::thread`: {line:?}\n\
                 The capstone must run on your executor: one thread for all connections, \
                 plus the driver threads that lessons 03 and 04 start."
            );
        }
    }
}
