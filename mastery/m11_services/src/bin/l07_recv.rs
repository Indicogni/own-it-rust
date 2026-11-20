//! Lesson 07: the receiving program. Supplied; it only calls your lesson code.
//!
//! Listens on a loopback port chosen by the operating system, prints
//! `listening on ADDR`, serves exactly one connection with
//! `l07_protocol::receive`, prints a summary line and exits.
//!
//! Run by hand, in two terminals:
//!   cargo run --manifest-path mastery/m11_services/Cargo.toml --bin l07_recv
//!   cargo run --manifest-path mastery/m11_services/Cargo.toml --bin l07_send -- ADDR 5 100

use m11_services::l07_protocol::{Received, receive};
use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    // A stuck program must not outlive the test that started it.
    match tokio::time::timeout(Duration::from_secs(30), run()).await {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(message)) => {
            eprintln!("l07_recv: {message}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!("l07_recv: gave up after 30 s");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|err| format!("bind: {err}"))?;
    let addr = listener.local_addr().map_err(|err| err.to_string())?;
    println!("listening on {addr}");
    std::io::stdout().flush().map_err(|err| err.to_string())?;

    let (stream, _) = listener
        .accept()
        .await
        .map_err(|err| format!("accept: {err}"))?;
    // A small queue: the consumer below is the only thing that drains it.
    let (sink, mut queue) = mpsc::channel::<Received>(4);
    let consumer = tokio::spawn(async move {
        let (mut samples, mut sum) = (0u64, 0i64);
        while let Some(batch) = queue.recv().await {
            samples += batch.samples.len() as u64;
            sum += batch.samples.iter().map(|sample| sample.value).sum::<i64>();
        }
        (samples, sum)
    });
    let batches = receive(stream, sink)
        .await
        .map_err(|err| format!("receive: {err}"))?;
    let (samples, sum) = consumer.await.map_err(|err| err.to_string())?;
    println!("received batches={batches} samples={samples} sum={sum}");
    Ok(())
}
