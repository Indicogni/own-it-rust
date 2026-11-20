//! Capstone reference solution: the beacon telemetry system.
//!
//! One possible design; yours may differ and still pass the acceptance tests.
//!
//! - [`config`]: environment → validated settings (pure functions, unit-tested).
//! - [`link`]: reading and writing `beacon_wire` frames on a tokio stream.
//! - [`store`]: the SQLite schema and the two queries the system needs.
//! - [`auth`]: Argon2id password check and HMAC-signed tokens.
//! - [`ingest`]: the TCP listener that stores batches and acknowledges them.
//! - [`api`]: the HTTP API.
//! - [`agent`]: sampling, the bounded buffer, and the shipping loop.

pub mod agent;
pub mod api;
pub mod auth;
pub mod config;
pub mod ingest;
pub mod link;
pub mod store;

/// Exit code for a configuration error (`EX_CONFIG` from BSD `sysexits.h`).
pub const EXIT_CONFIG: u8 = 78;
