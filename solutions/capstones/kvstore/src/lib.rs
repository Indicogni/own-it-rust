//! A persistent key-value store server: the reference solution of the
//! `kvstore` capstone. `DESIGN.md` next to this crate explains the
//! decisions; `SPEC.md` in the capstone directory is the contract.
//!
//! The modules, from the bottom up:
//!
//! - [`crc32`]: the checksum.
//! - [`record`]: keys, values, and how a record looks on disk.
//! - [`log`]: the log file: recovery, appending, reading, compaction.
//! - [`store`]: the index in memory plus the log, behind one lock.
//! - [`protocol`]: request lines and commands; no I/O.
//! - [`server`]: sockets, threads, shutdown.
//! - [`config`]: the command line.

#![forbid(unsafe_code)]

pub mod config;
pub mod crc32;
pub mod log;
pub mod protocol;
pub mod record;
pub mod server;
pub mod store;

use std::fmt;
use std::io::{self, Write};
use std::net::{Ipv4Addr, TcpListener};

use config::Config;
use store::{Options, Store, StoreError};

/// Why [`run`] returned without a clean shutdown.
#[derive(Debug)]
pub enum RunError {
    /// The server could not start (SPEC 3.4: exit code 3).
    Start(String),
    /// The server stopped because the store failed (exit code 1).
    Store(StoreError),
}

impl RunError {
    pub fn exit_code(&self) -> u8 {
        match self {
            RunError::Start(_) => 3,
            RunError::Store(_) => 1,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Start(why) => write!(f, "cannot start: {why}"),
            RunError::Store(e) => write!(f, "stopping: {e}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Opens the store, announces the address and serves until `SHUTDOWN`.
pub fn run(config: &Config) -> Result<(), RunError> {
    let options = Options {
        fsync: config.fsync,
        auto_compact: config.auto_compact,
    };
    let (store, recovery) =
        Store::open(&config.dir, options).map_err(|e| RunError::Start(e.to_string()))?;
    if recovery.discarded > 0 {
        eprintln!(
            "kvstore: recovery kept {} records and cut off {} bytes that were not a valid record",
            recovery.records, recovery.discarded
        );
    }

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port))
        .map_err(|e| RunError::Start(format!("binding 127.0.0.1:{} failed: {e}", config.port)))?;
    let addr = listener
        .local_addr()
        .map_err(|e| RunError::Start(format!("reading the listening address failed: {e}")))?;

    // The one line on standard output. Whoever started the server waits for
    // it, so it must leave this process now. A closed standard output is
    // not a reason to stop serving.
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "LISTENING {addr}").and_then(|()| stdout.flush());
    drop(stdout);

    server::serve(&listener, addr, &store, config.max_clients).map_err(RunError::Store)
}
