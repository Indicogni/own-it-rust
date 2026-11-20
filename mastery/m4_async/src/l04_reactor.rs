//! LESSON 04 — A reactor over non-blocking sockets
//! Read first: CONCEPTS.md, "Lesson 04". Start after: lesson 03.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. A non-blocking `read` finds no data. What does it return, and what must
//!    the future do before it returns `Pending`?
//! 2. The data arrives after the failed `read` and before the waker is
//!    registered. Why is this wakeup not lost with the reactor of this lesson?
//!
//! IMPLEMENT the tasks; keep the tests. You design the fields; replace each
//! `_design: ()` placeholder.
//! EXPLAIN: Describe one round trip of `stream.read(&mut buf).await` when no
//! data is there yet: every call, every thread, and where each waker clone
//! lives. Then say what a reactor built on epoll, kqueue or IOCP does
//! differently in the step "decide whom to wake", and what that saves.

use std::future::Future;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

/// How long the reactor thread waits between two rounds of wakes.
pub const TICK: Duration = Duration::from_millis(1);

/// Locks a mutex and ignores poisoning (see lesson 02).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A. Owns the reactor thread.
///
/// The standard library cannot ask the operating system which sockets are
/// ready, so this reactor *polls*: an operation that finds its socket not
/// ready registers its waker, and the reactor thread wakes every registered
/// waker one [`TICK`] later. The woken task then tries again.
///
/// The thread:
/// 1. sleeps (on a condition variable, not in a loop) while no waker is
///    registered;
/// 2. then waits one `TICK`;
/// 3. then takes all registered wakers, releases the lock, and wakes them.
///
/// Each registration is woken once. Dropping the reactor stops and joins
/// the thread and releases the registered wakers without waking them
/// (outside the lock).
pub struct Reactor {
    _design: (),
}

impl Reactor {
    /// Starts the reactor thread.
    pub fn start() -> Reactor {
        todo!("A: shared state and one thread")
    }

    /// A handle for sockets. It can be cloned and sent to other threads.
    pub fn handle(&self) -> ReactorHandle {
        todo!("A: a handle to the shared state")
    }
}

// A. Implement `Drop for Reactor` here (see the contract above).

/// Registers interest in "try again soon" with one [`Reactor`].
#[derive(Clone)]
pub struct ReactorHandle {
    _design: (),
}

impl ReactorHandle {
    /// B. The heart of every asynchronous socket operation: try the
    /// non-blocking operation `op` now.
    ///
    /// - `Err` of kind `WouldBlock`: the socket is not ready. Register the
    ///   waker of `cx` with the reactor and return `Pending`. Do not call
    ///   `op` again in this poll, and do not wake the waker yourself.
    /// - `Err` of kind `Interrupted`: a signal arrived during the system
    ///   call. Call `op` again at once.
    /// - Anything else, `Ok` or `Err`: return it as `Ready`.
    pub fn poll_io<T>(
        &self,
        cx: &mut Context<'_>,
        mut op: impl FnMut() -> io::Result<T>,
    ) -> Poll<io::Result<T>> {
        todo!("B: try, then register or return")
    }

    /// A. The number of wakers that are registered and not yet woken.
    pub fn waiting(&self) -> usize {
        todo!("A: count the registered wakers")
    }
}

/// C. A TCP listener whose `accept` is a future.
pub struct AsyncListener {
    _design: (),
}

impl AsyncListener {
    /// Takes over a bound listener and switches it to non-blocking mode.
    pub fn new(listener: TcpListener, reactor: &ReactorHandle) -> io::Result<AsyncListener> {
        todo!("C: non-blocking mode, then store both parts")
    }

    /// The address the listener is bound to.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        todo!("C: ask the listener")
    }

    /// One attempt to accept a connection, through `poll_io`. The accepted
    /// stream must be non-blocking on every platform: some systems copy the
    /// mode of the listener to the new socket, others do not.
    pub fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<(AsyncStream, SocketAddr)>> {
        todo!("C: accept through poll_io, then wrap the stream")
    }

    /// Waits for the next connection.
    pub async fn accept(&self) -> io::Result<(AsyncStream, SocketAddr)> {
        todo!("C: a future from poll_accept")
    }
}

/// C. A TCP stream whose reads and writes are futures. All methods take
/// `&self`, so one task can read while another writes: `Read` and `Write`
/// are implemented for `&TcpStream`.
pub struct AsyncStream {
    _design: (),
}

impl AsyncStream {
    /// Takes over a connected stream and switches it to non-blocking mode.
    pub fn new(stream: TcpStream, reactor: &ReactorHandle) -> io::Result<AsyncStream> {
        todo!("C: non-blocking mode, then store both parts")
    }

    /// The address of the other end.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        todo!("C: ask the stream")
    }

    /// One attempt to read into `buf`. `Ready(Ok(0))` means end of stream
    /// (for a non-empty `buf`).
    pub fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        todo!("C: read through poll_io")
    }

    /// One attempt to write from `buf`. The count may be smaller than
    /// `buf.len()`.
    pub fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        todo!("C: write through poll_io")
    }

    /// Reads some bytes: at least one, unless the stream has ended.
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        todo!("C: a future from poll_read")
    }

    /// Writes all of `buf`, in as many partial writes as it takes. A write
    /// that accepts zero bytes is the error `ErrorKind::WriteZero`.
    pub async fn write_all(&self, mut buf: &[u8]) -> io::Result<()> {
        todo!("C: loop over poll_write until nothing is left")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{WakeCounter, WakeLog, block_on, guess, within};
    use std::net::Shutdown;
    use std::pin::pin;
    use std::sync::mpsc::{self, Sender};
    use std::task::Wake;

    const LIMIT: Duration = Duration::from_secs(10);

    /// An operation that gives the scripted results in order and counts its calls.
    fn scripted(
        results: Vec<io::Result<u32>>,
    ) -> (impl FnMut() -> io::Result<u32>, Arc<Mutex<usize>>) {
        let calls = Arc::new(Mutex::new(0));
        let counter = Arc::clone(&calls);
        let mut results = results.into_iter();
        let op = move || {
            *counter.lock().unwrap() += 1;
            results
                .next()
                .expect("the operation was called more often than the script allows")
        };
        (op, calls)
    }

    fn would_block() -> io::Result<u32> {
        Err(ErrorKind::WouldBlock.into())
    }

    #[test]
    fn b_poll_io_returns_a_result_at_once() {
        within(20, "poll_io with an operation that succeeds", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let (counter, waker) = WakeCounter::new();
            let (mut op, calls) = scripted(vec![Ok(5)]);
            let result = handle.poll_io(&mut Context::from_waker(&waker), &mut op);
            assert!(
                matches!(result, Poll::Ready(Ok(5))),
                "a result that is not WouldBlock is returned as Ready"
            );
            assert_eq!(*calls.lock().unwrap(), 1, "one call is enough");
            assert_eq!(
                (handle.waiting(), Arc::strong_count(&counter)),
                (0, 2),
                "(registered wakers, references to the waker): a finished operation registers nothing"
            );
        });
    }

    #[test]
    fn b_poll_io_registers_on_would_block_and_the_reactor_wakes() {
        within(
            20,
            "poll_io with an operation that is not ready twice",
            || {
                let reactor = Reactor::start();
                let handle = reactor.handle();
                let (waker, wakes) = WakeLog::pair();
                let mut cx = Context::from_waker(&waker);
                let (mut op, calls) = scripted(vec![would_block(), would_block(), Ok(7)]);
                let mut waking_threads = Vec::new();
                for round in 1..=2 {
                    assert!(
                        handle.poll_io(&mut cx, &mut op).is_pending(),
                        "WouldBlock must become Pending"
                    );
                    assert_eq!(
                        *calls.lock().unwrap(),
                        round,
                        "after WouldBlock, poll_io must return without calling the operation again"
                    );
                    let by = wakes.recv_timeout(LIMIT).expect(
                        "the waker was not woken within 10 s: poll_io must register it, \
                     and the reactor thread must wake it after one tick",
                    );
                    waking_threads.push(by);
                }
                assert!(matches!(
                    handle.poll_io(&mut cx, &mut op),
                    Poll::Ready(Ok(7))
                ));
                assert!(
                    waking_threads
                        .iter()
                        .all(|id| *id != thread::current().id()),
                    "the wake came from the thread that called poll_io: \
                 poll_io must not wake itself, the reactor thread does that"
                );
                assert_eq!(
                    waking_threads[0], waking_threads[1],
                    "both wakes must come from the one reactor thread, not from a new thread per wait"
                );
            },
        );
    }

    #[test]
    fn b_poll_io_retries_interrupted_and_passes_errors_through() {
        within(20, "poll_io with Interrupted and with a real error", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let mut cx = Context::from_waker(Waker::noop());
            let interrupted = || Err(ErrorKind::Interrupted.into());
            let (mut op, calls) = scripted(vec![interrupted(), interrupted(), Ok(1)]);
            assert!(
                matches!(handle.poll_io(&mut cx, &mut op), Poll::Ready(Ok(1))),
                "Interrupted means: call the operation again at once"
            );
            assert_eq!(*calls.lock().unwrap(), 3);

            let (mut op, _) = scripted(vec![Err(ErrorKind::ConnectionReset.into())]);
            match handle.poll_io(&mut cx, &mut op) {
                Poll::Ready(Err(error)) => assert_eq!(error.kind(), ErrorKind::ConnectionReset),
                _ => panic!("an error that is not WouldBlock or Interrupted is returned as Ready"),
            }
            assert_eq!(handle.waiting(), 0, "only WouldBlock registers a waker");
        });
    }

    #[test]
    fn a_b_each_registration_is_woken_exactly_once() {
        within(20, "one registration and a quiet period", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let (waker, wakes) = WakeLog::pair();
            let (mut op, _) = scripted(vec![would_block()]);
            assert!(
                handle
                    .poll_io(&mut Context::from_waker(&waker), &mut op)
                    .is_pending()
            );
            wakes
                .recv_timeout(LIMIT)
                .expect("the registered waker was not woken within 10 s");
            assert_eq!(
                handle.waiting(),
                0,
                "a waker that was woken must be forgotten"
            );
            // Fifty ticks pass. A reactor that keeps old registrations wakes again.
            thread::sleep(TICK * 50);
            assert!(
                wakes.try_recv().is_err(),
                "the waker was woken a second time without a second registration"
            );
        });
    }

    /// A waker that calls into the reactor when it is woken and when it is
    /// dropped. Both calls need the reactor's lock.
    struct Probe {
        handle: ReactorHandle,
        events: Mutex<Sender<&'static str>>,
    }
    impl Wake for Probe {
        fn wake(self: Arc<Self>) {
            self.handle.waiting();
            let _ = self.events.lock().unwrap().send("woken");
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.handle.waiting();
            let _ = self.events.lock().unwrap().send("dropped");
        }
    }
    fn probe(handle: &ReactorHandle) -> (Waker, mpsc::Receiver<&'static str>) {
        let (tx, rx) = mpsc::channel();
        let waker = Waker::from(Arc::new(Probe {
            handle: handle.clone(),
            events: Mutex::new(tx),
        }));
        (waker, rx)
    }

    #[test]
    fn a_the_reactor_wakes_after_releasing_its_lock() {
        within(
            20,
            "a waker that uses the reactor again when it is woken (the reactor \
             must release its lock before it calls wake)",
            || {
                let reactor = Reactor::start();
                let handle = reactor.handle();
                let (waker, events) = probe(&handle);
                let (mut op, _) = scripted(vec![would_block()]);
                assert!(
                    handle
                        .poll_io(&mut Context::from_waker(&waker), &mut op)
                        .is_pending()
                );
                drop(waker);
                assert_eq!(
                    events.recv_timeout(LIMIT),
                    Ok("woken"),
                    "the wake did not finish: the reactor called wake while it held its lock"
                );
                assert_eq!(events.recv_timeout(LIMIT), Ok("dropped"));
            },
        );
    }

    #[test]
    fn a_dropping_the_reactor_stops_the_thread_and_releases_wakers() {
        within(
            20,
            "dropping a Reactor that has registered wakers (stop and join the \
             thread; drop the wakers outside the lock)",
            || {
                let reactor = Reactor::start();
                let handle = reactor.handle();
                let (counter, waker) = WakeCounter::new();
                let (probe_waker, events) = probe(&handle);
                for waker in [&waker, &probe_waker] {
                    let (mut op, _) = scripted(vec![would_block()]);
                    assert!(
                        handle
                            .poll_io(&mut Context::from_waker(waker), &mut op)
                            .is_pending()
                    );
                }
                drop(probe_waker);
                drop(reactor);
                assert_eq!(
                    Arc::strong_count(&counter),
                    2,
                    "dropping the reactor must release every registered waker; \
                     if the thread is still running, it still owns them"
                );
                assert!(
                    events.try_iter().any(|event| event == "dropped"),
                    "the probe waker was not released"
                );
                assert_eq!(handle.waiting(), 0);
            },
        );
    }

    /// A listener on an ephemeral loopback port, and two connected ends.
    fn pair(handle: &ReactorHandle) -> (TcpStream, AsyncStream) {
        let listener = AsyncListener::new(TcpListener::bind("127.0.0.1:0").unwrap(), handle)
            .expect("AsyncListener::new failed");
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client.set_read_timeout(Some(LIMIT)).unwrap();
        client.set_write_timeout(Some(LIMIT)).unwrap();
        let (server, peer) = block_on(listener.accept()).expect("accept failed");
        assert_eq!(
            peer,
            client.local_addr().unwrap(),
            "accept must return the address of the client"
        );
        assert_eq!(server.peer_addr().unwrap(), peer);
        (client, server)
    }

    #[test]
    fn c_accept_is_pending_while_nobody_connects() {
        within(
            20,
            "poll_accept without a client (a listener that is still in blocking \
             mode waits here for ever)",
            || {
                let reactor = Reactor::start();
                let handle = reactor.handle();
                let listener =
                    AsyncListener::new(TcpListener::bind("127.0.0.1:0").unwrap(), &handle).unwrap();
                assert!(
                    listener
                        .poll_accept(&mut Context::from_waker(Waker::noop()))
                        .is_pending(),
                    "no client has connected yet"
                );
            },
        );
    }

    #[test]
    fn c_a_wrapped_stream_never_blocks() {
        within(
            20,
            "poll_read on streams without data (a stream that is still in \
             blocking mode waits here for ever; call set_nonblocking(true) in \
             AsyncStream::new)",
            || {
                let reactor = Reactor::start();
                let handle = reactor.handle();
                let (_client, server) = pair(&handle);
                let mut cx = Context::from_waker(Waker::noop());
                let mut buf = [0u8; 8];
                assert!(
                    server.poll_read(&mut cx, &mut buf).is_pending(),
                    "the accepted stream has no data yet"
                );
                // A stream from `connect` starts in blocking mode on every
                // platform, and this one has no read timeout either.
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let blocking = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
                let wrapped = AsyncStream::new(blocking, &handle).unwrap();
                assert!(
                    wrapped.poll_read(&mut cx, &mut buf).is_pending(),
                    "the wrapped stream has no data yet"
                );
            },
        );
    }

    #[test]
    fn c_read_and_write_cross_the_loopback() {
        within(20, "a request and a reply over loopback", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let (mut client, server) = pair(&handle);
            client.write_all(b"ping").unwrap();
            let request = block_on(async {
                let mut request = Vec::new();
                let mut buf = [0u8; 16];
                while request.len() < 4 {
                    let n = server.read(&mut buf).await.expect("read failed");
                    assert!(
                        n > 0,
                        "the client has not closed, so read must not report the end"
                    );
                    request.extend_from_slice(&buf[..n]);
                }
                server.write_all(b"pong").await.expect("write_all failed");
                request
            });
            assert_eq!(request, b"ping");
            let mut reply = [0u8; 4];
            client.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"pong");

            client.shutdown(Shutdown::Write).unwrap();
            let mut buf = [0u8; 16];
            let at_end = block_on(server.read(&mut buf)).expect("read failed");
            assert_eq!(
                at_end, 0,
                "after the client closes its writing side, read gives 0"
            );
            drop(server);
            let mut rest = Vec::new();
            client.read_to_end(&mut rest).unwrap();
            assert!(
                rest.is_empty(),
                "dropping the AsyncStream closes the connection"
            );
        });
    }

    #[test]
    fn c_one_thread_serves_two_connections() {
        within(20, "two connections on one thread", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let (mut quiet_client, quiet) = pair(&handle);
            let (mut busy_client, busy) = pair(&handle);
            busy_client.write_all(b"b").unwrap();
            // Wait for data on both streams at once. The quiet one must not
            // hold up the busy one.
            let mut quiet_buf = [0u8; 1];
            let mut busy_buf = [0u8; 1];
            let first = block_on(std::future::poll_fn(|cx| {
                if let Poll::Ready(result) = quiet.poll_read(cx, &mut quiet_buf) {
                    return Poll::Ready(("quiet", result.unwrap()));
                }
                busy.poll_read(cx, &mut busy_buf)
                    .map(|result| ("busy", result.unwrap()))
            }));
            assert_eq!(first, ("busy", 1), "(stream that had data, bytes read)");
            assert_eq!(busy_buf, *b"b");
            quiet_client.write_all(b"q").unwrap();
            assert_eq!(block_on(quiet.read(&mut quiet_buf)).unwrap(), 1);
            assert_eq!(quiet_buf, *b"q");
        });
    }

    #[test]
    fn c_write_all_survives_a_full_socket_buffer() {
        const SIZE: usize = 16 * 1024 * 1024;
        let received = within(60, "sending 16 MiB to a client that reads late", || {
            let reactor = Reactor::start();
            let handle = reactor.handle();
            let (mut client, server) = pair(&handle);
            let payload: Vec<u8> = (0..SIZE).map(|i| (i % 251) as u8).collect();
            // The client starts to read only when it is told to. Until then
            // the kernel's buffers fill up, a write accepts only a part of
            // the data, and the next write reports WouldBlock.
            let (go_tx, go_rx) = mpsc::channel::<()>();
            let reader = thread::spawn(move || {
                go_rx.recv().unwrap();
                let mut received = Vec::new();
                client.read_to_end(&mut received).unwrap();
                received
            });
            let (waker, wakes) = WakeLog::pair();
            let mut cx = Context::from_waker(&waker);
            let mut reader_started = false;
            {
                let mut write = pin!(server.write_all(&payload));
                let result = loop {
                    match write.as_mut().poll(&mut cx) {
                        Poll::Ready(result) => break result,
                        Poll::Pending => {
                            if !reader_started {
                                reader_started = true;
                                go_tx.send(()).unwrap();
                            }
                            wakes
                                .recv_timeout(LIMIT)
                                .expect("write_all returned Pending and was not woken within 10 s");
                        }
                    }
                };
                result.expect("write_all failed");
            }
            if !reader_started {
                go_tx.send(()).unwrap();
            }
            drop(server);
            let received = reader.join().unwrap();
            assert_eq!(
                received.len(),
                SIZE,
                "every byte must arrive exactly once: a write may accept only a part \
                 of the buffer, and write_all must continue with the rest"
            );
            received == payload
        });
        assert!(received, "the bytes must arrive in order and unchanged");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        server.set_read_timeout(Some(LIMIT)).unwrap();
        let mut buf = [0u8; 8];

        // P1. The listener is switched to non-blocking mode and no second
        // client connects. Which error kind does `accept` give?
        listener.set_nonblocking(true).unwrap();
        let p1: ErrorKind = guess();
        assert_eq!(
            p1,
            listener.accept().unwrap_err().kind(),
            "P1: accept on a non-blocking listener without a waiting client"
        );

        // P2. The client sends 3 bytes and closes. The server reads with a
        // blocking socket and an 8-byte buffer until `read` returns 0. Then
        // the server socket is switched to non-blocking mode and reads once
        // more. Is that last result Ok(0) (write `true`) or a WouldBlock
        // error (write `false`)?
        client.write_all(b"abc").unwrap();
        drop(client);
        let mut total = 0;
        loop {
            let n = server.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }
        assert_eq!(total, 3);
        server.set_nonblocking(true).unwrap();
        let p2: bool = guess();
        assert_eq!(
            p2,
            matches!(server.read(&mut buf), Ok(0)),
            "P2: the end of a stream on a non-blocking socket"
        );

        // P3. This loop is the core of `poll_io`. The operation fails with
        // Interrupted, Interrupted, WouldBlock, and would then succeed. How
        // many times is it called before the loop ends?
        let mut script = [
            ErrorKind::Interrupted,
            ErrorKind::Interrupted,
            ErrorKind::WouldBlock,
        ]
        .into_iter();
        let mut calls = 0;
        loop {
            calls += 1;
            match script.next() {
                Some(ErrorKind::Interrupted) => continue,
                _ => break,
            }
        }
        let p3: usize = guess();
        assert_eq!(p3, calls, "P3: calls of the operation in one poll");
    }
}
