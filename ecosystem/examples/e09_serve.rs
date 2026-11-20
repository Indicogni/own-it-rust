//! E09: run your stock service as a real process and talk to it with `curl`.
//! Run: cargo run --manifest-path ecosystem/Cargo.toml --example e09_serve
//! (Pass another address as the first argument if port 3000 is taken.)
//!
//! In a second terminal:
//!
//! ```sh
//! curl -i http://127.0.0.1:3000/health
//! curl -i -X PUT -H 'content-type: application/json' -d '{"quantity":5}' http://127.0.0.1:3000/stock/bolt
//! curl -i -X POST -H 'content-type: application/json' -d '{"quantity":9}' http://127.0.0.1:3000/stock/bolt/take
//! curl -i -X PUT -d '{"quantity":5}' http://127.0.0.1:3000/stock/bolt
//! ```
//!
//! Then press Ctrl-C in the first terminal and watch the process leave through
//! `serve`'s graceful shutdown instead of being killed.
//!
//! This file is complete. It only wires together what you wrote in `src/e09_http.rs`.

use ecosystem::e09_http::{AppState, app, serve, with_deadline};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

// `#[tokio::main]` builds the multi-thread runtime: one worker thread per CPU core.
// That is the usual choice for a server. `jobq` (E07) uses the current-thread runtime.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3000".to_owned());
    let listener = TcpListener::bind(&address).await?;
    println!("listening on http://{}", listener.local_addr()?);

    // One token, cancelled by the signal task and watched by the server. Any other
    // background task of a larger service would hold a clone of the same token.
    let shutdown = CancellationToken::new();
    let on_signal = shutdown.clone();
    tokio::spawn(async move {
        // Ctrl-C is SIGINT on Unix. A service in a container is stopped with SIGTERM;
        // listen for that with `tokio::signal::unix::signal(SignalKind::terminate())`.
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nCtrl-C: finishing requests in flight, then stopping");
            on_signal.cancel();
        }
    });

    let service = with_deadline(app(AppState::default()), Duration::from_secs(5));
    serve(listener, service, shutdown).await?;
    println!("stopped cleanly");
    Ok(())
}
