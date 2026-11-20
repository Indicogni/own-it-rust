//! Lesson 07: the protocol between two real programs.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml --test l07_two_programs
//!
//! Starts `l07_recv` and `l07_send` as child processes. They share nothing but
//! the `beacon_wire` crate and a TCP connection on a loopback port that the
//! operating system chooses. Both children are killed when the test ends, also
//! when it fails. Until lesson 07 is implemented, the programs stop at `todo!()`
//! and this test reports their output.

use m11_services::support::ChildGuard;
use std::process::Command;

#[test]
fn sender_and_receiver_agree_on_the_bytes() {
    let mut receiver = ChildGuard::spawn(
        "l07_recv",
        &mut Command::new(env!("CARGO_BIN_EXE_l07_recv")),
    );
    let listening = receiver.line(20);
    let addr = listening
        .strip_prefix("listening on ")
        .unwrap_or_else(|| panic!("unexpected first line from l07_recv: {listening:?}"))
        .to_owned();

    let mut sender = ChildGuard::spawn(
        "l07_send",
        Command::new(env!("CARGO_BIN_EXE_l07_send")).args([addr.as_str(), "25", "40"]),
    );
    assert_eq!(sender.line(20), "acked 25", "stderr: {}", sender.stderr());
    assert_eq!(sender.wait(20), Some(0), "stderr: {}", sender.stderr());

    // 25 batches of 40 samples with the values 0..1000.
    let sum: i64 = (0..1000).sum();
    assert_eq!(
        receiver.line(20),
        format!("received batches=25 samples=1000 sum={sum}"),
        "stderr: {}",
        receiver.stderr()
    );
    assert_eq!(receiver.wait(20), Some(0), "stderr: {}", receiver.stderr());
}

#[test]
fn the_receiver_rejects_a_program_that_speaks_something_else() {
    use std::io::Write;
    let mut receiver = ChildGuard::spawn(
        "l07_recv",
        &mut Command::new(env!("CARGO_BIN_EXE_l07_recv")),
    );
    let listening = receiver.line(20);
    let addr = listening
        .strip_prefix("listening on ")
        .unwrap_or_else(|| panic!("unexpected first line from l07_recv: {listening:?}"));

    let mut stream = std::net::TcpStream::connect(addr).expect("connect to l07_recv");
    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: example\r\n\r\n")
        .expect("write");
    assert_eq!(
        receiver.wait(20),
        Some(1),
        "a peer that does not speak the protocol is an error, not a hang. stderr: {}",
        receiver.stderr()
    );
    assert!(
        receiver.stderr().contains("bad magic"),
        "the error should name the problem, got: {}",
        receiver.stderr()
    );
}
