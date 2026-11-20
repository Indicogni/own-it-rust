//! Lesson 07: the sending program. Supplied; it only calls your lesson code.
//!
//! Usage: l07_send ADDR BATCHES SAMPLES_PER_BATCH
//! Connects to ADDR, sends the batches with `l07_protocol::send_batches`, prints
//! `acked N` and exits. The sample values are 0, 1, 2, ... across all batches.

use beacon_wire::Sample;
use m11_services::l07_protocol::send_batches;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match tokio::time::timeout(Duration::from_secs(30), run(&args)).await {
        Ok(Ok(acked)) => {
            println!("acked {acked}");
            ExitCode::SUCCESS
        }
        Ok(Err(message)) => {
            eprintln!("l07_send: {message}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!("l07_send: gave up after 30 s");
            ExitCode::from(2)
        }
    }
}

async fn run(args: &[String]) -> Result<u64, String> {
    let [addr, batches, per_batch] = args else {
        return Err("usage: l07_send ADDR BATCHES SAMPLES_PER_BATCH".to_owned());
    };
    let batches: u64 = batches.parse().map_err(|_| "BATCHES must be a number")?;
    let per_batch: u64 = per_batch
        .parse()
        .map_err(|_| "SAMPLES_PER_BATCH must be a number")?;
    let work: Vec<(String, Vec<Sample>)> = (0..batches)
        .map(|batch| {
            let samples = (0..per_batch)
                .map(|index| {
                    let number = batch * per_batch + index;
                    Sample {
                        ts_ms: 1_800_000_000_000 + number * 100,
                        value: number as i64,
                    }
                })
                .collect();
            ("demo".to_owned(), samples)
        })
        .collect();
    let stream = TcpStream::connect(addr)
        .await
        .map_err(|err| format!("connect to {addr}: {err}"))?;
    send_batches(stream, "l07-send", work)
        .await
        .map_err(|err| format!("send: {err}"))
}
