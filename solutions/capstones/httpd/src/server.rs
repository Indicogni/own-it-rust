//! Start-up, the accept loop, admission control and shutdown (SPEC
//! sections 3, 10 and 11).
//!
//! Threads:
//!
//! - the main thread accepts connections and hands them to the pool;
//! - `workers` pool threads each serve one connection at a time;
//! - one control thread reads standard input and starts the shutdown.

use std::fmt;
use std::io::{self, BufRead, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::config::Config;
use crate::conn::{self, Context};
use crate::pool::Pool;
use crate::response::{Response, Status};

/// Why the server could not start. Exit status 1.
#[derive(Debug)]
pub enum StartError {
    Root(PathBuf, io::Error),
    RootNotADirectory(PathBuf),
    Bind(SocketAddr, io::Error),
    Threads(io::Error),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(root, error) => write!(f, "cannot use root `{}`: {error}", root.display()),
            Self::RootNotADirectory(root) => {
                write!(f, "root `{}` is not a directory", root.display())
            }
            Self::Bind(addr, error) => write!(f, "cannot listen on {addr}: {error}"),
            Self::Threads(error) => write!(f, "cannot start threads: {error}"),
        }
    }
}

impl std::error::Error for StartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Root(_, error) | Self::Bind(_, error) | Self::Threads(error) => Some(error),
            Self::RootNotADirectory(_) => None,
        }
    }
}

/// Runs the server until it has shut down gracefully.
pub fn run(config: Config) -> Result<(), StartError> {
    // Resolve the root once. Later requests are then not affected by a
    // change of the working directory, and on Windows the canonical form
    // switches off the rewriting of names such as `NUL` or `file.`.
    let root = config
        .root
        .canonicalize()
        .map_err(|error| StartError::Root(config.root.clone(), error))?;
    if !root.is_dir() {
        return Err(StartError::RootNotADirectory(config.root));
    }

    let listener =
        TcpListener::bind(config.addr).map_err(|error| StartError::Bind(config.addr, error))?;
    let addr = listener
        .local_addr()
        .map_err(|error| StartError::Bind(config.addr, error))?;

    let (workers, queue) = (config.workers, config.queue);
    let context = Arc::new(Context {
        config,
        root,
        shutdown: AtomicBool::new(false),
    });
    let pool = {
        let context = Arc::clone(&context);
        Pool::new(workers, queue, move |stream| conn::serve(stream, &context))
            .map_err(StartError::Threads)?
    };
    {
        let context = Arc::clone(&context);
        thread::Builder::new()
            .name("control".into())
            .spawn(move || control(&context, addr))
            .map_err(StartError::Threads)?;
    }

    // Only now is every part in place: announce the address. A parent
    // process that reads this line may connect at once.
    announce(format_args!("listening on {addr}"));

    accept_loop(&listener, &pool, &context);

    // SPEC 11.2, in this order: stop accepting, say so, drop what waits,
    // let the workers finish.
    drop(listener);
    announce(format_args!("shutting down"));
    // Dropping a queued connection closes it without a response.
    drop(pool.close());
    pool.join();
    Ok(())
}

/// Writes one protocol line to standard output. A parent that has closed
/// the pipe must not bring the server down, so errors are ignored
/// (`println!` would panic).
fn announce(line: fmt::Arguments<'_>) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// Accepts connections until shutdown begins.
fn accept_loop(listener: &TcpListener, pool: &Pool<TcpStream>, context: &Context) {
    loop {
        let accepted = listener.accept();
        if context.shutting_down() {
            // Whatever was accepted (most likely the control thread's
            // wake-up connection) is dropped unanswered.
            return;
        }
        match accepted {
            Ok((stream, _peer)) => {
                if let Err(stream) = pool.try_submit(stream) {
                    reject(stream);
                }
            }
            // The client gave up between connecting and `accept`, or was
            // interrupted: nothing is wrong with the listener.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => {
                // Typically "too many open files". Sleeping gives the
                // workers time to close some; looping at once would only
                // burn a core on the same error.
                eprintln!("httpd: accept failed: {error}");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// How many 4 KiB reads [`reject`] spends on what a refused client has
/// already sent: 64 KiB, the same amount as the lingering close.
const REJECT_DRAIN_READS: usize = 16;

/// Tells a connection that there is no room for it (SPEC 10.3).
///
/// This runs on the accept thread, so it must never wait: the response is
/// far smaller than a socket's send buffer, and the write has a short
/// timeout in case it is not.
fn reject(mut stream: TcpStream) {
    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
    let response =
        Response::status_only(Status::ServiceUnavailable).with_header("Retry-After", "1");
    if response.write_to(&mut stream, false, false).is_err() {
        return;
    }
    // Take what the client has already sent out of the socket, without
    // waiting for more. A close with unread bytes can turn into a reset
    // that destroys the 503 before the client reads it. A full lingering
    // close (see `conn::linger`) would be more thorough, but it waits.
    if stream.set_nonblocking(true).is_ok() {
        let mut sink = [0u8; 4096];
        // A bounded number of reads: a client that sends as fast as this
        // loop reads must not keep the accept thread either.
        for _ in 0..REJECT_DRAIN_READS {
            if !matches!(stream.read(&mut sink), Ok(1..)) {
                break;
            }
        }
    }
}

/// The control thread: waits for the line `shutdown` or for the end of
/// standard input, then starts the shutdown (SPEC 11.1).
fn control(context: &Context, listening_on: SocketAddr) {
    wait_for_shutdown_request(io::stdin().lock());
    context.shutdown.store(true, Ordering::Relaxed);
    wake_acceptor(listening_on);
}

/// Returns when `input` has delivered the line `shutdown` or has ended.
/// Other lines are ignored, whatever they contain and however long they
/// are: only the first few bytes of a line are kept.
fn wait_for_shutdown_request(input: impl BufRead) {
    const COMMAND: &[u8] = b"shutdown";
    let mut line = Vec::with_capacity(COMMAND.len() + 2);
    // `bytes()` on a buffered reader does not cost a system call per byte.
    for byte in input.bytes() {
        match byte {
            Err(_) => return,
            Ok(b'\n') => {
                if line.strip_suffix(b"\r").unwrap_or(&line) == COMMAND {
                    return;
                }
                line.clear();
            }
            // One byte more than the longest accepted line (`shutdown\r`)
            // is enough to know that a longer line is not the command.
            Ok(byte) if line.len() <= COMMAND.len() + 1 => line.push(byte),
            Ok(_) => {}
        }
    }
}

/// The accept loop is blocked in `accept` and looks at the flag only when
/// a connection arrives, so one is made here. If this fails, the listen
/// queue is full, which means `accept` has work and will return anyway.
fn wake_acceptor(listening_on: SocketAddr) {
    // A listener on "any address" cannot be connected to as such; the
    // loopback address of the same family reaches it.
    let ip = match listening_on.ip() {
        ip if !ip.is_unspecified() => ip,
        IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
        IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
    };
    let target = SocketAddr::new(ip, listening_on.port());
    let _ = TcpStream::connect_timeout(&target, Duration::from_secs(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// True when the reader was left before its end, that is, when a
    /// shutdown line was found.
    fn stops_early(input: &[u8]) -> bool {
        let mut reader = input;
        wait_for_shutdown_request(&mut reader);
        !reader.is_empty()
    }

    #[test]
    fn only_the_exact_line_asks_for_shutdown() {
        assert!(stops_early(b"shutdown\nrest"));
        assert!(stops_early(b"shutdown\r\nrest"));
        assert!(stops_early(b"hello\n\nshutdown\nrest"));
        for other in [
            &b"shutdown now\nrest\n"[..],
            b" shutdown\nrest\n",
            b"shutdown \nrest\n",
            b"SHUTDOWN\nrest\n",
            b"shutdown\r\r\nrest\n",
            b"\xff\xfe\nrest\n",
            b"xshutdown\nrest\n",
        ] {
            assert!(!stops_early(other), "{:?}", String::from_utf8_lossy(other));
        }
        let long = [vec![b'x'; 100_000], b"shutdown\nrest\n".to_vec()].concat();
        assert!(!stops_early(&long), "the end of a long line is not a line");
    }
}
