//! `beacon_wire` frames on a tokio stream (the reader and writer from lesson 07).

use beacon_wire::{Message, WireError, decode_frame, encode_frame};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Wire(#[from] WireError),
    #[error("the connection closed in the middle of a frame")]
    UnexpectedEof,
    #[error("unexpected message: {0}")]
    Unexpected(&'static str),
    #[error("the peer was silent for too long")]
    Timeout,
}

pub struct FrameReader<R> {
    io: R,
    buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(io: R) -> FrameReader<R> {
        FrameReader {
            io,
            buf: Vec::new(),
        }
    }

    /// The next message, or `None` when the peer closed between frames.
    pub async fn next(&mut self) -> Result<Option<Message>, LinkError> {
        loop {
            if let Some((message, used)) = decode_frame(&self.buf)? {
                self.buf.drain(..used);
                return Ok(Some(message));
            }
            let mut chunk = [0u8; 4096];
            let count = self.io.read(&mut chunk).await?;
            if count == 0 {
                return if self.buf.is_empty() {
                    Ok(None)
                } else {
                    Err(LinkError::UnexpectedEof)
                };
            }
            self.buf.extend_from_slice(&chunk[..count]);
        }
    }
}

pub async fn write_message<W: AsyncWrite + Unpin>(
    io: &mut W,
    message: &Message,
) -> Result<(), LinkError> {
    let mut frame = Vec::new();
    encode_frame(message, &mut frame)?;
    io.write_all(&frame).await?;
    io.flush().await?;
    Ok(())
}
