//! LESSON 08 — Framed messages over TCP
//! Run: cargo test --test chapter18 l08_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. One side calls write_all three times. How many reads does the other side
//!    need to see all the bytes?
//! 2. How can a reader tell "the peer has finished" from "the peer vanished in
//!    the middle of a message"?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Why does a TCP protocol need framing? Why is the declared length
//! checked before the body buffer is created? Why does the client shut down only
//! its writing half instead of closing the connection?
//! COMPILER LAB (examples/ch18_scratch.rs): Bind a listener to 127.0.0.1:0, print
//! its port, and connect to it from a second thread. Give the client a read
//! timeout of one second and let the server stay silent: which error kind does
//! the client's read return on your platform?

use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};

// The protocol of this lesson:
//
//   frame    a length (u32, big endian), then that many payload bytes
//   client   sends one request frame, then waits for exactly one reply frame;
//            repeats for each request; at the end it shuts down its writing
//            half and keeps reading until the server closes
//   server   answers each request with the same payload in ASCII upper case;
//            when the client's stream ends cleanly it sends one last frame,
//            the number of requests it served as decimal digits (b"3"),
//            and closes the connection
//
// Every function takes `max`, the largest payload length it accepts. The
// tests never use a `max` below 32, so the last frame always fits.

/// A. Write one frame: the payload length as a u32 in big-endian byte order,
/// then the payload. A payload longer than `max` is an InvalidInput error, and
/// then nothing may be written. Do not flush here.
fn send_frame<W: Write>(mut out: W, payload: &[u8], max: u32) -> io::Result<()> {
    // Check first: after a rejected payload the writer must be untouched.
    let len = u32::try_from(payload.len())
        .ok()
        .filter(|&len| len <= max)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "payload above the limit"))?;
    out.write_all(&len.to_be_bytes())?;
    out.write_all(payload)
}

/// B. Read one frame and return its payload.
/// - The stream ends before the first header byte: `Ok(None)`. The peer has
///   finished, and that is not an error.
/// - The stream ends anywhere later, inside the header or the payload:
///   UnexpectedEof.
/// - The declared length is above `max`: InvalidData, decided before the body
///   buffer is allocated and before any body byte is read.
///
/// One `read` may return any part of the header, even a single byte. Retry a
/// read that fails with Interrupted; propagate every other error.
fn recv_frame<R: Read>(mut input: R, max: u32) -> io::Result<Option<Vec<u8>>> {
    // read_exact cannot tell "no bytes at all" from "some bytes, then the
    // end", so the header is filled by hand.
    let mut header = [0u8; 4];
    let mut filled = 0;
    while filled < header.len() {
        match input.read(&mut header[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    let len = u32::from_be_bytes(header);
    // The peer chose this number. Decide before the buffer exists.
    if len > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "declared frame length is above the limit",
        ));
    }
    // A u32 fits in usize on every platform the standard library's sockets support.
    let mut body = vec![0u8; len as usize];
    // From here on, an early end is a cut frame: read_exact reports it.
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

/// C. Accept one connection from `listener` and play the server's side of the
/// protocol on it. Return the number of requests served. Any error ends the
/// service at once: return it, and send no further frame.
fn serve_one(listener: &TcpListener, max: u32) -> io::Result<usize> {
    let (stream, _peer) = listener.accept()?;
    // `&TcpStream` implements Read and Write, so one socket can feed a
    // buffered reader and a buffered writer at the same time.
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    let mut served = 0;
    while let Some(mut payload) = recv_frame(&mut reader, max)? {
        payload.make_ascii_uppercase();
        send_frame(&mut writer, &payload, max)?;
        // The client is waiting for this reply, so it must leave now. Header
        // and payload leave together, in one write to the socket.
        writer.flush()?;
        served += 1;
    }
    send_frame(&mut writer, served.to_string().as_bytes(), max)?;
    writer.flush()?;
    Ok(served)
}

/// D. Connect to `addr` and play the client's side of the protocol for
/// `requests`. Return every payload received, in order: one reply per request,
/// then every frame the server sends after your half-close. If the stream ends
/// cleanly where a reply was due, that is an UnexpectedEof error.
fn exchange(addr: SocketAddr, requests: &[&[u8]], max: u32) -> io::Result<Vec<Vec<u8>>> {
    let stream = TcpStream::connect(addr)?;
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    let mut received = Vec::with_capacity(requests.len() + 1);
    for request in requests {
        send_frame(&mut writer, request, max)?;
        writer.flush()?;
        // A clean end here means the server left without answering.
        let reply = recv_frame(&mut reader, max)?.ok_or(io::ErrorKind::UnexpectedEof)?;
        received.push(reply);
    }
    // Half-close: the server now reads a clean end, and this side can still
    // receive what the server sends last.
    stream.shutdown(Shutdown::Write)?;
    while let Some(frame) = recv_frame(&mut reader, max)? {
        received.push(frame);
    }
    Ok(received)
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::thread;
    use std::time::Duration;

    // Every test that touches a socket runs through `within`, and every
    // socket the tests own has a timeout. A protocol mistake therefore ends
    // in a failed test with a message, never in a test run that hangs.
    const IN_MEMORY: &str = "recv_frame (does the read loop stop at end of input?)";
    const SOCKETS: &str = "the loopback conversation (a socket read that never returns \
                           usually means one side is waiting for bytes the other side \
                           has not sent or not flushed, or for an end of stream that \
                           was never signalled)";
    const WAIT: Duration = Duration::from_secs(5);

    fn with_timeouts(stream: TcpStream) -> io::Result<TcpStream> {
        stream.set_read_timeout(Some(WAIT))?;
        stream.set_write_timeout(Some(WAIT))?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    /// Reads exactly `expected.len()` bytes and compares them.
    fn expect_bytes(stream: &mut TcpStream, expected: &[u8], what: &str) -> io::Result<()> {
        let mut got = vec![0u8; expected.len()];
        stream.read_exact(&mut got).map_err(|e| {
            io::Error::new(e.kind(), format!("{what}: expected {expected:?}, but: {e}"))
        })?;
        if got != expected {
            return Err(io::Error::other(format!(
                "{what}: expected the bytes {expected:?}, got {got:?}"
            )));
        }
        Ok(())
    }

    /// Succeeds only if the peer has closed its sending side and sent nothing more.
    fn expect_end(stream: &mut TcpStream, what: &str) -> io::Result<()> {
        let mut extra = [0u8; 1];
        match stream.read(&mut extra) {
            Ok(0) => Ok(()),
            Ok(_) => Err(io::Error::other(format!(
                "{what}: unexpected extra byte {extra:?}"
            ))),
            Err(e) => Err(io::Error::new(
                e.kind(),
                format!("{what}: no end of stream arrived: {e}"),
            )),
        }
    }

    /// Runs your `serve_one` on its own thread and `client` (raw bytes written
    /// by the test) on this one. Returns the server's result, the largest
    /// single allocation the server made, and the client's result.
    fn against_your_server<T: Send + 'static>(
        max: u32,
        client: impl FnOnce(TcpStream) -> io::Result<T> + Send + 'static,
    ) -> (io::Result<usize>, usize, io::Result<T>) {
        within(30, SOCKETS, move || {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
            let addr = listener.local_addr().expect("the listener has an address");
            let server =
                thread::spawn(move || crate::largest_allocation(|| serve_one(&listener, max)));
            // The stream is dropped when `client` returns, which lets a
            // server that is still reading see the end of the stream.
            let from_client = TcpStream::connect(addr)
                .and_then(with_timeouts)
                .and_then(client);
            let (served, largest) = server
                .join()
                .expect("serve_one panicked; its message is printed above");
            (served, largest, from_client)
        })
    }

    /// Runs `server` (raw bytes written by the test) on its own thread and
    /// your `exchange` on this one. Returns both results.
    fn against_your_client(
        requests: &'static [&'static [u8]],
        max: u32,
        server: impl FnOnce(TcpStream) -> io::Result<()> + Send + 'static,
    ) -> (io::Result<Vec<Vec<u8>>>, io::Result<()>) {
        within(30, SOCKETS, move || {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
            let addr = listener.local_addr().expect("the listener has an address");
            let double = thread::spawn(move || {
                let (stream, _) = listener.accept()?;
                server(with_timeouts(stream)?)
            });
            let received = exchange(addr, requests, max);
            // If `exchange` never connected, this wakes the test's server up.
            drop(TcpStream::connect(addr));
            let from_server = double.join().expect("the test's own server panicked");
            (received, from_server)
        })
    }

    /// Fails if the client has already sent something that was not asked for.
    fn expect_silence(stream: &TcpStream, what: &str) -> io::Result<()> {
        stream.set_nonblocking(true)?;
        let early = stream.peek(&mut [0u8; 1]);
        stream.set_nonblocking(false)?;
        match early {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
            _ => Err(io::Error::other(format!(
                "{what}: send one request, then read its reply, before you send the next"
            ))),
        }
    }

    #[test]
    fn a_frame_is_a_big_endian_length_then_the_payload() {
        let mut out = vec![];
        send_frame(&mut out, b"hey", 32).unwrap();
        assert_eq!(
            out, b"\0\0\0\x03hey",
            "length 3 as u32 big endian, then the bytes"
        );
        out.clear();
        send_frame(&mut out, &[7; 258], 1024).unwrap();
        assert_eq!(
            &out[..5],
            [0, 0, 1, 2, 7],
            "258 is 0x0102: the most significant byte is sent first"
        );
        assert_eq!(out.len(), 4 + 258);
        out.clear();
        send_frame(&mut out, b"", 32).unwrap();
        assert_eq!(out, [0, 0, 0, 0], "an empty payload is a header alone");
    }
    #[test]
    fn a_frame_respects_the_limit_before_writing() {
        let mut out = vec![];
        send_frame(&mut out, b"12345", 5).unwrap();
        assert_eq!(out.len(), 9, "max is inclusive");
        out.clear();
        assert_eq!(
            send_frame(&mut out, b"123456", 5).unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
            "6 bytes are above max = 5"
        );
        assert!(
            out.is_empty(),
            "nothing may be written for a rejected payload"
        );
    }
    #[test]
    fn a_frame_survives_short_writes_and_does_not_flush() {
        struct Narrow {
            bytes: Vec<u8>,
            flushed: usize,
        }
        impl Write for Narrow {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                let n = b.len().min(1);
                self.bytes.extend_from_slice(&b[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                self.flushed += 1;
                Ok(())
            }
        }
        let mut w = Narrow {
            bytes: vec![],
            flushed: 0,
        };
        send_frame(&mut w, b"\xffok", 32).unwrap();
        assert_eq!(
            w.bytes, b"\0\0\0\x03\xffok",
            "this writer takes one byte per call; every byte must still arrive"
        );
        assert_eq!(w.flushed, 0, "the caller decides when to flush");
    }

    #[test]
    fn b_frames_come_out_one_at_a_time_until_a_clean_end() {
        within(5, IN_MEMORY, || {
            let mut input = &b"\0\0\0\x02hi\0\0\0\0\0\0\x01\x02"[..];
            assert_eq!(
                recv_frame(&mut input, 32).unwrap().as_deref(),
                Some(&b"hi"[..])
            );
            assert_eq!(
                input.len(),
                8,
                "the first call must leave the next two frames unread"
            );
            assert_eq!(
                recv_frame(&mut input, 32).unwrap().as_deref(),
                Some(&b""[..]),
                "a zero length is a valid, empty frame, not the end"
            );
            assert_eq!(
                recv_frame(&mut input, 1024).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "header 00 00 01 02 declares 258 bytes (big endian) and none follow"
            );
            assert_eq!(
                recv_frame(&b""[..], 32).unwrap(),
                None,
                "no bytes at all is a clean end: Ok(None)"
            );
        });
    }
    #[test]
    fn b_a_stream_cut_inside_a_frame_is_an_error() {
        within(5, IN_MEMORY, || {
            for cut in [
                &b"\0"[..],
                b"\0\0",
                b"\0\0\0",
                b"\0\0\0\x05",
                b"\0\0\0\x05abcd",
            ] {
                assert_eq!(
                    recv_frame(cut, 32).unwrap_err().kind(),
                    io::ErrorKind::UnexpectedEof,
                    "{cut:?} ends inside a frame; only an end before the first \
                     header byte is clean"
                );
            }
        });
    }
    #[test]
    fn b_the_header_may_arrive_in_pieces() {
        /// One byte per call; the very first call is interrupted.
        struct Drip {
            interrupt: bool,
            rest: &'static [u8],
        }
        impl Read for Drip {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                if self.interrupt {
                    self.interrupt = false;
                    return Err(io::ErrorKind::Interrupted.into());
                }
                if self.rest.is_empty() || b.is_empty() {
                    return Ok(0);
                }
                b[0] = self.rest[0];
                self.rest = &self.rest[1..];
                Ok(1)
            }
        }
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::ConnectionReset.into())
            }
        }
        within(5, IN_MEMORY, || {
            let mut drip = Drip {
                interrupt: true,
                rest: b"\0\0\0\x03abc\0",
            };
            assert_eq!(
                recv_frame(&mut drip, 32).unwrap().as_deref(),
                Some(&b"abc"[..]),
                "retry Interrupted, and keep reading until the header is complete"
            );
            assert_eq!(drip.rest, b"\0", "exactly one frame may be consumed");
            assert_eq!(
                recv_frame(&mut drip, 32).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "one header byte and then the end is a cut frame"
            );
            assert_eq!(
                recv_frame(Broken, 32).unwrap_err().kind(),
                io::ErrorKind::ConnectionReset,
                "a real read error is returned as it is"
            );
        });
    }
    #[test]
    fn b_an_oversized_length_is_rejected_before_the_body() {
        within(5, IN_MEMORY, || {
            let mut input = &b"\0\0\0\x06abcdef"[..];
            assert_eq!(
                recv_frame(&mut input, 5).unwrap_err().kind(),
                io::ErrorKind::InvalidData,
                "a declared length of 6 is above max = 5"
            );
            assert_eq!(input, b"abcdef", "only the header may be consumed");
            assert_eq!(
                recv_frame(&b"\0\0\0\x05abcde"[..], 5).unwrap().as_deref(),
                Some(&b"abcde"[..]),
                "max is inclusive"
            );
            // This header declares 16 MiB, and the limit is 1024 bytes.
            let (result, largest) =
                crate::largest_allocation(|| recv_frame(&b"\x01\0\0\0"[..], 1024));
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert!(
                largest < 4096,
                "rejecting an oversized frame allocated {largest} bytes in one piece: \
                 compare the declared length with max before you create the body buffer"
            );
        });
    }

    #[test]
    fn c_server_answers_each_request_and_then_sends_the_count() {
        let (served, _, client) = against_your_server(1024, |mut stream| {
            // The header of the first request leaves in two pieces.
            stream.write_all(b"\0\0")?;
            stream.write_all(b"\0\x05")?;
            stream.write_all(b"hello")?;
            expect_bytes(&mut stream, b"\0\0\0\x05HELLO", "reply 1")?;
            // Two requests leave in a single write: an empty one, and one
            // that is not UTF-8.
            stream.write_all(b"\0\0\0\0\0\0\0\x03a\xffz")?;
            expect_bytes(&mut stream, b"\0\0\0\0", "reply 2 (empty)")?;
            expect_bytes(&mut stream, b"\0\0\0\x03A\xffZ", "reply 3")?;
            stream.shutdown(Shutdown::Write)?;
            expect_bytes(&mut stream, b"\0\0\0\x013", "the final count frame")?;
            expect_end(&mut stream, "after the count frame")
        });
        client.expect("the client side of the conversation failed");
        assert_eq!(
            served.expect("serve_one returned an error"),
            3,
            "the result counts the requests served"
        );
    }
    #[test]
    fn c_server_with_no_requests_sends_a_zero_count() {
        let (served, _, client) = against_your_server(32, |mut stream| {
            stream.shutdown(Shutdown::Write)?;
            expect_bytes(&mut stream, b"\0\0\0\x010", "the final count frame")?;
            expect_end(&mut stream, "after the count frame")
        });
        client.expect("the client side of the conversation failed");
        assert_eq!(served.expect("serve_one returned an error"), 0);
    }
    #[test]
    fn c_server_rejects_an_oversized_request_without_a_reply() {
        // The header declares 16 MiB; the limit is 1024 bytes.
        let (served, largest, client) = against_your_server(1024, |mut stream| {
            stream.write_all(b"\x01\0\0\0")?;
            let mut reply = Vec::new();
            // The server closes at once; depending on timing the client sees
            // a clean end or a reset. Either way no reply bytes may arrive.
            let _ = stream.read_to_end(&mut reply);
            Ok(reply)
        });
        assert_eq!(
            served.unwrap_err().kind(),
            io::ErrorKind::InvalidData,
            "the server must report the oversized frame as its error"
        );
        assert_eq!(
            client.expect("the client could not connect or write"),
            b"",
            "no frame may be sent after an error"
        );
        assert!(
            largest < (1 << 20),
            "the server allocated {largest} bytes in one piece for a frame it rejected"
        );
    }
    #[test]
    fn c_server_reports_a_client_that_vanishes_inside_a_frame() {
        let (served, _, client) = against_your_server(1024, |mut stream| {
            stream.write_all(b"\0\0\0\x05he")
            // The stream is dropped here: 2 of the 5 promised bytes were sent.
        });
        client.expect("the client could not connect or write");
        assert_eq!(
            served.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof,
            "a frame that is cut short is an error, not a clean end"
        );
    }

    #[test]
    fn d_client_takes_turns_and_then_half_closes() {
        let (received, server) =
            against_your_client(&[b"ab", b"", b"\xff\0"], 1024, |mut stream| {
                expect_bytes(&mut stream, b"\0\0\0\x02ab", "request 1")?;
                expect_silence(&stream, "request 2 arrived before reply 1 was sent")?;
                stream.write_all(b"\0\0\0\x02AB")?;
                expect_bytes(&mut stream, b"\0\0\0\0", "request 2 (empty)")?;
                expect_silence(&stream, "request 3 arrived before reply 2 was sent")?;
                // This reply leaves in two pieces.
                stream.write_all(b"\0\0")?;
                stream.write_all(b"\0\0")?;
                expect_bytes(&mut stream, b"\0\0\0\x02\xff\0", "request 3")?;
                stream.write_all(b"\0\0\0\x02\xff\0")?;
                // The count is sent only after the client's end of stream.
                expect_end(
                    &mut stream,
                    "waiting for the client to shut down its writing half",
                )?;
                stream.write_all(b"\0\0\0\x013")
            });
        let received = received.expect("exchange returned an error");
        server.expect("the test's server saw a protocol mistake");
        assert_eq!(
            received,
            [&b"AB"[..], b"", b"\xff\0", b"3"],
            "three replies, then the count frame that arrives after the half-close"
        );
    }
    #[test]
    fn d_client_reports_a_missing_or_cut_reply() {
        let (received, _) = against_your_client(&[b"ping"], 1024, |mut stream| {
            expect_bytes(&mut stream, b"\0\0\0\x04ping", "request 1")
            // The stream is dropped here: the server closes without a reply.
        });
        assert_eq!(
            received.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof,
            "a clean end where a reply was due is an error for the client"
        );
        let (received, _) = against_your_client(&[b"ping"], 1024, |mut stream| {
            expect_bytes(&mut stream, b"\0\0\0\x04ping", "request 1")?;
            stream.write_all(b"\0\0\0\x04PI")
        });
        assert_eq!(
            received.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof,
            "a reply that is cut short is an error"
        );
    }
    #[test]
    fn d_client_rejects_an_oversized_reply() {
        let (received, _) = against_your_client(&[b"ping"], 8, |mut stream| {
            expect_bytes(&mut stream, b"\0\0\0\x04ping", "request 1")?;
            stream.write_all(b"\0\0\0\x09")
        });
        assert_eq!(
            received.unwrap_err().kind(),
            io::ErrorKind::InvalidData,
            "the reply declares 9 bytes and max is 8"
        );
    }

    #[test]
    fn cd_your_client_and_your_server_talk_to_each_other() {
        // 300 000 bytes do not fit one packet, so this payload reaches the
        // other side in many pieces.
        let (received, served) = within(30, SOCKETS, || {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
            let addr = listener.local_addr().expect("the listener has an address");
            let server = thread::spawn(move || serve_one(&listener, 1 << 20));
            let big = vec![b'q'; 300_000];
            let received = exchange(addr, &[b"one", &big, b""], 1 << 20);
            // If `exchange` never connected, this lets the server finish.
            drop(TcpStream::connect(addr));
            let served = server
                .join()
                .expect("serve_one panicked; its message is printed above");
            (received, served)
        });
        let received = received.expect("exchange returned an error");
        assert_eq!(received.len(), 4, "three replies and the count frame");
        assert_eq!(received[0], b"ONE");
        assert_eq!(
            received[1].len(),
            300_000,
            "the large reply must be complete"
        );
        assert!(received[1].iter().all(|&b| b == b'Q'));
        assert_eq!(received[2], b"");
        assert_eq!(received[3], b"3", "the count frame");
        assert_eq!(served.expect("serve_one returned an error"), 3);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        let (port_is_zero, is_loopback, whole_chunks, after, at_end, late) =
            within(30, "the loopback sockets of p_predictions", || {
                // P1 looks at this listener's address.
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let addr = listener.local_addr().unwrap();

                // P2. The sender makes three 2-byte writes and closes. The
                // receiver asks for 3 bytes at a time.
                let sender = thread::spawn(move || {
                    let mut stream = TcpStream::connect(addr).unwrap();
                    for part in [b"ab", b"cd", b"ef"] {
                        stream.write_all(part).unwrap();
                    }
                });
                let mut receiver = with_timeouts(listener.accept().unwrap().0).unwrap();
                let mut chunk = [0u8; 3];
                let mut whole_chunks = 0;
                while receiver.read_exact(&mut chunk).is_ok() {
                    whole_chunks += 1;
                }
                let after = receiver.read(&mut chunk).unwrap();
                sender.join().unwrap();

                // P3. The client shuts down its writing half only.
                let mut client = with_timeouts(TcpStream::connect(addr).unwrap()).unwrap();
                let mut server = with_timeouts(listener.accept().unwrap().0).unwrap();
                client.shutdown(Shutdown::Write).unwrap();
                let at_end = server.read(&mut chunk).unwrap();
                server.write_all(b"late").unwrap();
                drop(server);
                let mut late = String::new();
                client.read_to_string(&mut late).unwrap();

                (
                    addr.port() == 0,
                    addr.ip().is_loopback(),
                    whole_chunks,
                    after,
                    at_end,
                    late,
                )
            });

        // P1. After binding to "127.0.0.1:0": is the port that local_addr
        // reports 0? Is the address a loopback address?
        let p1: (bool, bool) = (false, true);
        assert_eq!(p1, (port_is_zero, is_loopback), "P1: binding to port 0");

        // P2. How many 3-byte read_exact calls succeed, and what does the
        // next plain read return?
        let p2: (usize, usize) = (2, 0);
        assert_eq!(
            p2,
            (whole_chunks, after),
            "P2: writes and reads do not line up"
        );

        // P3. After the client's shutdown(Write): what does the server's read
        // return, and which text does the client then read?
        let p3: (usize, &str) = (0, "late");
        assert_eq!(p3, (at_end, late.as_str()), "P3: half-close");

        // P4. 258 as big-endian bytes, and the u32 that the big-endian bytes
        // 00 00 02 01 stand for.
        let p4: ([u8; 4], u32) = ([0, 0, 1, 2], 513);
        assert_eq!(
            p4,
            (258u32.to_be_bytes(), u32::from_be_bytes([0, 0, 2, 1])),
            "P4: network byte order"
        );
    }
}
