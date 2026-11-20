//! `beacon-agent`: samples a counter and ships it to `beacon-server`.

use beacon::EXIT_CONFIG;
use beacon::config::{AgentConfig, Env};
use std::process::ExitCode;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let config = match AgentConfig::from_env(&Env::from_process()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("beacon-agent: {error}");
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let totals = beacon::agent::run(config).await;
    println!("shipped={} dropped={}", totals.shipped, totals.dropped);
    ExitCode::SUCCESS
}
