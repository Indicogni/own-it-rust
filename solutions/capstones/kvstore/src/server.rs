//! The TCP side: one thread per connection, the connection limit, and
//! graceful shutdown (SPEC 8, 11, 12).

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::protocol::{self, Command, Fed, LineBuffer};
use crate::store::{Incremented, Store, StoreError};

/// The open connections. Each socket is shared by the thread that serves
/// it and this table, so that the accepting thread can close every socket
/// at shutdown. An `Arc` and not `try_clone`: a clone would cost a second
/// file descriptor per connection.
#[derive(Debug, Default)]
struct Clients {
    next_id: u64,
    open: HashMap<u64, Arc<TcpStream>>,
}

/// What every connection thread shares. The threads are scoped, so this is
/// borrowed, not reference-counted.
#[derive(Debug)]
struct Shared<'a> {
    store: &'a Store,
    /// Where the listener is; a connection to it wakes the accepting thread.
    addr: SocketAddr,
    max_clients: usize,
    stopping: AtomicBool,
    /// A second handle on the listening socket. Nothing is ever done with
    /// it: it keeps one file descriptor in reserve for [`Shared::stop`].
    reserve: Mutex<Option<TcpListener>>,
    clients: Mutex<Clients>,
    /// The first store failure, if the server stops because of one.
    failure: Mutex<Option<StoreError>>,
}

impl Shared<'_> {
    /// Starts the shutdown; safe to call more than once.
    fn stop(&self) {
        if self.stopping.swap(true, Ordering::SeqCst) {
            return;
        }
        // The accepting thread is blocked in `accept`. It checks the flag
        // after every connection, so give it one.
        //
        // That connection needs a file descriptor. A server that has
        // reached its limit of open files has none left; the connection
        // would fail, and the server would never stop. Closing the
        // reserved descriptor first makes room for it.
        drop(
            self.reserve
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take(),
        );
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_secs(1));
    }

    fn fail(&self, error: StoreError) {
        self.failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert(error);
        self.stop();
    }

    /// Registers a new connection, or gives the socket back if the server
    /// is full.
    fn admit(&self, stream: TcpStream) -> Result<(u64, Arc<TcpStream>), TcpStream> {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        if clients.open.len() >= self.max_clients {
            return Err(stream);
        }
        let stream = Arc::new(stream);
        let id = clients.next_id;
        clients.next_id += 1;
        clients.open.insert(id, Arc::clone(&stream));
        Ok((id, stream))
    }

    fn release(&self, id: u64) {
        self.clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .open
            .remove(&id);
    }
}

/// Serves clients on `listener` until a `SHUTDOWN` request or a store
/// failure. Returns after every connection thread has ended and the log
/// has been synced.
///
/// `addr` is the address `listener` is bound to.
pub fn serve(
    listener: &TcpListener,
    addr: SocketAddr,
    store: &Store,
    max_clients: usize,
) -> Result<(), StoreError> {
    let shared = Shared {
        store,
        addr,
        max_clients,
        stopping: AtomicBool::new(false),
        // If the system refuses the second handle, the server runs without
        // a reserve.
        reserve: Mutex::new(listener.try_clone().ok()),
        clients: Mutex::default(),
        failure: Mutex::default(),
    };
    let shared = &shared;

    // A scope joins all its threads before it returns. That is the whole
    // "wait for the connections" part of the shutdown.
    thread::scope(|scope| {
        // Whether the last `accept` failed, so that a failure that lasts
        // is reported once and not fifty times per second.
        let mut failing = false;
        for incoming in listener.incoming() {
            if shared.stopping.load(Ordering::SeqCst) {
                break;
            }
            let stream = match incoming {
                Ok(stream) => {
                    failing = false;
                    stream
                }
                Err(e) => {
                    // Usually "too many open files". Wait a moment instead
                    // of spinning; connections that close will free some.
                    if !failing {
                        eprintln!("kvstore: accept failed: {e}");
                        failing = true;
                    }
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }
            };
            let (id, stream) = match shared.admit(stream) {
                Ok(admitted) => admitted,
                Err(mut refused) => {
                    // Nothing has been read from this client, so closing
                    // the socket after the line cannot reset the connection.
                    let _ = refused.write_all(b"ERR too many connections\n");
                    continue;
                }
            };
            let spawned = thread::Builder::new()
                .name(format!("client-{id}"))
                .spawn_scoped(scope, move || {
                    let result = handle(shared, &stream);
                    shared.release(id);
                    if let Err(ClientError::Store(e)) = result {
                        shared.fail(e);
                    }
                    // A socket error only ends this connection.
                });
            if let Err(e) = spawned {
                eprintln!("kvstore: cannot start a thread for a connection: {e}");
                shared.release(id);
            }
        }

        // Only this thread adds connections, so the set is final. Closing
        // both directions makes a blocked read return 0 and a blocked
        // write fail; each thread then leaves its loop.
        let clients = shared.clients.lock().unwrap_or_else(|e| e.into_inner());
        for stream in clients.open.values() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    });

    // Every response that was sent came from a thread that has ended, so
    // every acknowledged write is in the log now. Make it durable.
    let synced = store.sync();
    match shared
        .failure
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        Some(failure) => Err(failure),
        None => synced,
    }
}

/// Why a connection ended early.
enum ClientError {
    /// The socket failed: the client is gone. Nobody is left to tell, so
    /// the cause is not kept.
    Socket,
    /// The store failed; the whole server has to stop.
    Store(StoreError),
}

impl From<io::Error> for ClientError {
    fn from(_: io::Error) -> ClientError {
        ClientError::Socket
    }
}

impl From<StoreError> for ClientError {
    fn from(e: StoreError) -> ClientError {
        ClientError::Store(e)
    }
}

/// What to do after a request.
enum Next {
    Continue,
    Close,
}

/// Reads requests from one connection and answers them until the client
/// leaves, says `QUIT` or `SHUTDOWN`, or the server closes the socket.
fn handle(shared: &Shared<'_>, stream: &TcpStream) -> Result<(), ClientError> {
    // Responses are written in one piece each; do not let the system hold
    // them back to merge them with later ones.
    stream.set_nodelay(true)?;
    // `&TcpStream` implements `Read` and `Write`, so one socket can feed a
    // reader and a writer without being cloned.
    let mut input = BufReader::new(stream);
    let mut output = BufWriter::new(stream);
    let mut line = LineBuffer::new();
    loop {
        // Flush only when the next read would have to wait. A client that
        // pipelines a thousand requests gets its answers in a few large
        // writes; a client that sends one and waits gets its answer now.
        if input.buffer().is_empty() {
            output.flush()?;
        }
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            // End of input. A last line without a line feed is dropped.
            break;
        }
        let (used, fed) = line.feed(bytes);
        input.consume(used);
        match fed {
            Fed::Partial => {}
            Fed::TooLong => output.write_all(b"ERR line too long\n")?,
            Fed::Line => {
                if let Next::Close = respond(shared, line.line(), &mut output)? {
                    break;
                }
            }
        }
    }
    output.flush()?;
    Ok(())
}

/// Executes one request line and writes its response to `out`.
fn respond(shared: &Shared<'_>, line: &[u8], out: &mut impl Write) -> Result<Next, ClientError> {
    let command = match protocol::parse(line) {
        Ok(command) => command,
        Err(e) => {
            writeln!(out, "{}", e.reply())?;
            return Ok(Next::Continue);
        }
    };
    let store = shared.store;
    match command {
        Command::Ping => out.write_all(b"PONG\n")?,
        Command::Get(key) => match store.get(key)? {
            Some(value) => {
                out.write_all(b"VALUE ")?;
                out.write_all(&value)?;
                out.write_all(b"\n")?;
            }
            None => out.write_all(b"NOT_FOUND\n")?,
        },
        Command::Set(key, value) => {
            store.set(key, value)?;
            out.write_all(b"OK\n")?;
        }
        Command::Del(key) => {
            let reply: &[u8] = if store.delete(key)? {
                b"OK\n"
            } else {
                b"NOT_FOUND\n"
            };
            out.write_all(reply)?;
        }
        Command::Incr(key) => match store.incr(key)? {
            Incremented::To(number) => writeln!(out, "VALUE {number}")?,
            Incremented::NotAnInteger => out.write_all(b"ERR not an integer\n")?,
            Incremented::Overflow => out.write_all(b"ERR overflow\n")?,
        },
        Command::Keys(prefix) => {
            // The store lock is released before anything is written, so a
            // client that reads slowly cannot hold up the writers.
            let keys = store.keys(prefix)?;
            writeln!(out, "KEYS {}", keys.len())?;
            for key in keys {
                out.write_all(&key)?;
                out.write_all(b"\n")?;
            }
        }
        Command::Stats => {
            let stats = store.stats()?;
            writeln!(
                out,
                "STATS keys={} log_bytes={} live_bytes={} compactions={}",
                stats.keys, stats.log_bytes, stats.live_bytes, stats.compactions
            )?;
        }
        Command::Compact => {
            store.compact()?;
            out.write_all(b"OK\n")?;
        }
        Command::Quit => {
            out.write_all(b"BYE\n")?;
            return Ok(Next::Close);
        }
        Command::Shutdown => {
            // Answer first: once `stop` has run, the accepting thread may
            // close this socket at any moment.
            out.write_all(b"OK\n")?;
            out.flush()?;
            shared.stop();
            return Ok(Next::Close);
        }
    }
    Ok(Next::Continue)
}
