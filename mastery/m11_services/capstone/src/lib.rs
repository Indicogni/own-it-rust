//! CAPSTONE — beacon: a small telemetry system
//! Read first: `capstone/README.md` (the brief) and CONCEPTS.md, "Capstone".
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon
//!
//! This library is yours. The acceptance tests in `tests/acceptance.rs` start
//! the two binaries as separate processes and talk to them only through TCP,
//! HTTP, standard output and exit codes. Nothing in this crate has a fixed
//! name or signature: decide which modules exist, what the types are, and how
//! errors flow. Put the logic here and keep the two `main` functions thin, so
//! that you can write unit tests for the parts.
//!
//! A possible first cut, if you want one (delete it when you have your own):
//!
//! ```text
//! config   environment variables → validated settings
//! store    schema, "insert a batch", "read a page"
//! auth     password check, tokens
//! ingest   TCP listener: frames in, rows stored, acks out
//! api      the HTTP router
//! agent    sampler, bounded buffer, shipper
//! ```
