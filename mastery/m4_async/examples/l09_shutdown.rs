//! The worked example of lesson 09 (see CONCEPTS.md): a service that answers
//! each line in upper case, with a graceful shutdown.
//!
//! Run: cargo run --manifest-path mastery/m4_async/Cargo.toml --example l09_shutdown

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// One connection: answer every line in upper case until the client leaves
/// or the server shuts down, then say goodbye. Returns the number of answers.
async fn shout(stream: TcpStream, shutdown: CancellationToken) -> std::io::Result<u64> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut answered = 0;
    loop {
        // tokio documents `next_line` as cancellation safe, so it may lose
        // this race in any round.
        let line = tokio::select! {
            () = shutdown.cancelled() => break,
            line = lines.next_line() => line?,
        };
        let Some(line) = line else { break };
        let answer = format!("{}\n", line.to_uppercase());
        writer.write_all(answer.as_bytes()).await?;
        answered += 1;
    }
    writer.write_all(b"BYE\n").await?;
    writer.shutdown().await?;
    Ok(answered)
}

/// Accepts connections until `shutdown` is cancelled, then gives the open
/// connections `grace` to finish. Returns the total number of answers.
async fn serve(listener: TcpListener, shutdown: CancellationToken, grace: Duration) -> u64 {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            accepted = listener.accept() => {
                if let Ok((stream, _peer)) = accepted {
                    connections.spawn(shout(stream, shutdown.clone()));
                }
            }
        }
    }
    // Step 2: nobody can connect any more.
    drop(listener);
    // Steps 3 and 4: the connections have seen the token; wait for them.
    let mut answered = 0;
    let drain = async {
        while let Some(joined) = connections.join_next().await {
            if let Ok(Ok(count)) = joined {
                answered += count;
            }
        }
    };
    if timeout(grace, drain).await.is_err() {
        // Step 5.
        connections.shutdown().await;
    }
    answered
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let shutdown = CancellationToken::new();
    let grace = Duration::from_secs(5);
    let server = tokio::spawn(serve(listener, shutdown.clone(), grace));

    let mut client = BufReader::new(TcpStream::connect(addr).await?);
    client.get_mut().write_all(b"hello\n").await?;
    let mut reply = String::new();
    client.read_line(&mut reply).await?;
    assert_eq!(reply, "HELLO\n");

    // In a real service this line runs when Ctrl-C or SIGTERM arrives.
    shutdown.cancel();
    reply.clear();
    client.read_line(&mut reply).await?;
    assert_eq!(reply, "BYE\n");
    let answered = server.await.expect("the server task panicked");
    assert_eq!(answered, 1);
    println!("answered {answered} line, said goodbye, and shut down");
    Ok(())
}
