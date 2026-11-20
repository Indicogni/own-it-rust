//! `beacon-server`: the ingest listener and the HTTP API in one process.

use beacon::EXIT_CONFIG;
use beacon::api::{self, ApiState};
use beacon::auth::Authenticator;
use beacon::config::{Env, ServerConfig};
use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    let config = match ServerConfig::from_env(&Env::from_process()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("beacon-server: {error}");
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("beacon-server: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(config: ServerConfig) -> Result<(), String> {
    // Taking the configuration apart moves the plain password into the hashing
    // task below, where it is dropped as soon as the hash exists.
    let ServerConfig {
        ingest_addr,
        http_addr,
        database_url,
        admin_user,
        admin_password,
        token_key,
    } = config;
    let pool = beacon::store::open(&database_url)
        .await
        .map_err(|err| format!("database: {err}"))?;
    // Hashing is slow on purpose: not on an async worker thread.
    let auth = tokio::task::spawn_blocking(move || {
        Authenticator::new(&admin_user, &admin_password, &token_key)
    })
    .await
    .map_err(|err| err.to_string())??;

    let ingest = TcpListener::bind(ingest_addr)
        .await
        .map_err(|err| format!("bind {ingest_addr}: {err}"))?;
    let http = TcpListener::bind(http_addr)
        .await
        .map_err(|err| format!("bind {http_addr}: {err}"))?;
    let ingest_addr = ingest.local_addr().map_err(|err| err.to_string())?;
    let http_addr = http.local_addr().map_err(|err| err.to_string())?;

    let (stop_tx, stop_rx) = watch::channel(false);
    let ingest_task = tokio::spawn(beacon::ingest::run(ingest, pool.clone(), stop_rx));
    let app = api::router(ApiState::new(pool.clone(), auth));
    let mut http_stop = stop_tx.subscribe();
    let http_task = tokio::spawn(async move {
        axum::serve(http, app)
            .with_graceful_shutdown(async move {
                let _ = http_stop.changed().await;
            })
            .await
    });

    // Both listeners are open: only now announce the addresses.
    println!("ingest={ingest_addr}");
    println!("http={http_addr}");
    std::io::stdout().flush().map_err(|err| err.to_string())?;

    shutdown_signal().await;
    tracing::info!("shutdown requested, draining");
    let _ = stop_tx.send(true);
    let drain = async {
        let _ = ingest_task.await;
        let _ = http_task.await;
    };
    if tokio::time::timeout(Duration::from_secs(10), drain)
        .await
        .is_err()
    {
        tracing::warn!("drain time is over, exiting with requests in progress");
    }
    pool.close().await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
