//! Host acceptance tests for `kcore::Endpoint` (SPEC 5.4).
//!
//! ```text
//! cargo test --test host_ipc
//! ```
//!
//! In these tests `SERVER`, `ALICE`, `BOB` and `CAROL` are just task ids.
//! The endpoint never runs code; it only records who waits for what.

use kcore::{Endpoint, IpcError, Message, TaskId};

const SERVER: TaskId = TaskId(1);
const ALICE: TaskId = TaskId(2);
const BOB: TaskId = TaskId(3);
const CAROL: TaskId = TaskId(4);

fn msg(label: u64) -> Message {
    Message {
        label,
        words: [label + 1, label + 2, label + 3, label + 4],
    }
}

/// SPEC 5.4: `new` is a `const fn`; the kernel keeps endpoints in a `static`.
/// This function compiles only if `new` is a `const fn`.
const fn made_in_a_const_fn() -> Endpoint<2> {
    Endpoint::new()
}

#[test]
fn server_waits_first_then_client_sends() {
    let mut endpoint: Endpoint<4> = Endpoint::new();

    assert_eq!(
        endpoint.receive(SERVER),
        Ok(None),
        "nothing is queued: the server has to block"
    );
    assert_eq!(
        endpoint.send(ALICE, msg(10)),
        Ok(Some(SERVER)),
        "a server is blocked in receive: send must name it so the kernel can wake it"
    );
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((ALICE, msg(10)))),
        "the woken server calls receive again and gets the sender and an exact copy of the message"
    );
    assert_eq!(
        endpoint.take_reply(ALICE),
        None,
        "no reply yet: the client must stay blocked"
    );
    assert_eq!(endpoint.reply(ALICE, msg(20)), Ok(()));
    assert_eq!(
        endpoint.take_reply(ALICE),
        Some(msg(20)),
        "the client collects exactly the reply"
    );
    assert_eq!(
        endpoint.take_reply(ALICE),
        None,
        "a reply can be collected only once"
    );
}

#[test]
fn client_sends_first_then_server_receives() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(
        endpoint.send(ALICE, msg(1)),
        Ok(None),
        "no server is waiting: nobody to wake"
    );
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((ALICE, msg(1)))),
        "the request was queued, so receive returns it at once"
    );
}

#[test]
fn requests_are_received_in_the_order_they_were_sent() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(endpoint.send(BOB, msg(1)), Ok(None));
    assert_eq!(endpoint.send(ALICE, msg(2)), Ok(None));
    assert_eq!(endpoint.send(CAROL, msg(3)), Ok(None));
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((BOB, msg(1)))),
        "oldest request first"
    );
    // A slot freed in the middle must not disturb the order.
    assert_eq!(endpoint.reply(BOB, msg(0)), Ok(()));
    assert_eq!(endpoint.take_reply(BOB), Some(msg(0)));
    assert_eq!(endpoint.send(BOB, msg(4)), Ok(None));
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(2)))));
    assert_eq!(endpoint.receive(SERVER), Ok(Some((CAROL, msg(3)))));
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((BOB, msg(4)))),
        "BOB's second request was sent last, so it is received last"
    );
    assert_eq!(endpoint.receive(SERVER), Ok(None), "the queue is empty now");
}

#[test]
fn send_wakes_a_waiting_server_only_once() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(endpoint.receive(SERVER), Ok(None));
    assert_eq!(
        endpoint.send(ALICE, msg(1)),
        Ok(Some(SERVER)),
        "first send wakes the server"
    );
    assert_eq!(
        endpoint.send(BOB, msg(2)),
        Ok(None),
        "the server is already awake (the scheduler would refuse a second wake)"
    );
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(endpoint.receive(SERVER), Ok(Some((BOB, msg(2)))));
}

#[test]
fn replies_reach_the_right_client_in_any_order() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(endpoint.send(ALICE, msg(1)), Ok(None));
    assert_eq!(endpoint.send(BOB, msg(2)), Ok(None));
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(endpoint.receive(SERVER), Ok(Some((BOB, msg(2)))));

    // The server answers the second client first.
    assert_eq!(endpoint.reply(BOB, msg(200)), Ok(()));
    assert_eq!(
        endpoint.take_reply(ALICE),
        None,
        "ALICE has no reply yet; BOB's is not hers"
    );
    assert_eq!(endpoint.reply(ALICE, msg(100)), Ok(()));
    assert_eq!(endpoint.take_reply(ALICE), Some(msg(100)));
    assert_eq!(endpoint.take_reply(BOB), Some(msg(200)));
}

#[test]
fn reply_needs_a_client_whose_request_was_received() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(
        endpoint.reply(ALICE, msg(0)),
        Err(IpcError::NotAwaitingReply),
        "ALICE never sent anything"
    );
    assert_eq!(endpoint.send(ALICE, msg(1)), Ok(None));
    assert_eq!(
        endpoint.reply(ALICE, msg(0)),
        Err(IpcError::NotAwaitingReply),
        "ALICE's request is still queued: the server has not received it"
    );
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(endpoint.reply(ALICE, msg(2)), Ok(()));
    assert_eq!(
        endpoint.reply(ALICE, msg(3)),
        Err(IpcError::NotAwaitingReply),
        "one request gets one reply"
    );
    assert_eq!(
        endpoint.take_reply(ALICE),
        Some(msg(2)),
        "the refused second reply changed nothing"
    );
}

#[test]
fn a_client_has_one_call_at_a_time() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(endpoint.send(ALICE, msg(1)), Ok(None));
    assert_eq!(
        endpoint.send(ALICE, msg(2)),
        Err(IpcError::CallInProgress),
        "ALICE's first request is still queued"
    );
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(
        endpoint.send(ALICE, msg(2)),
        Err(IpcError::CallInProgress),
        "ALICE is waiting for the reply"
    );
    assert_eq!(endpoint.reply(ALICE, msg(9)), Ok(()));
    assert_eq!(
        endpoint.send(ALICE, msg(2)),
        Err(IpcError::CallInProgress),
        "ALICE has not collected the reply yet"
    );
    assert_eq!(endpoint.take_reply(ALICE), Some(msg(9)));
    assert_eq!(
        endpoint.send(ALICE, msg(2)),
        Ok(None),
        "the call is over; ALICE may send again"
    );
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((ALICE, msg(2)))),
        "the refused sends left nothing behind in the queue"
    );
}

#[test]
fn at_most_n_calls_are_in_progress() {
    let mut endpoint: Endpoint<2> = Endpoint::new();
    assert_eq!(endpoint.send(ALICE, msg(1)), Ok(None));
    assert_eq!(endpoint.send(BOB, msg(2)), Ok(None));
    assert_eq!(
        endpoint.send(CAROL, msg(3)),
        Err(IpcError::Full),
        "N = 2 and two calls are queued"
    );

    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(
        endpoint.send(CAROL, msg(3)),
        Err(IpcError::Full),
        "a received call still counts until its reply is collected"
    );
    assert_eq!(endpoint.reply(ALICE, msg(0)), Ok(()));
    assert_eq!(
        endpoint.send(CAROL, msg(3)),
        Err(IpcError::Full),
        "the reply is stored but not collected"
    );
    assert_eq!(endpoint.take_reply(ALICE), Some(msg(0)));
    assert_eq!(
        endpoint.send(CAROL, msg(3)),
        Ok(None),
        "ALICE's call is over: there is room again"
    );
}

#[test]
fn only_one_task_waits_to_receive() {
    let mut endpoint: Endpoint<4> = Endpoint::new();
    assert_eq!(endpoint.receive(SERVER), Ok(None));
    assert_eq!(
        endpoint.receive(BOB),
        Err(IpcError::ReceiverBusy),
        "SERVER is already blocked in receive on this endpoint"
    );
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(None),
        "the same task may ask again (it does so after every wake-up)"
    );
    assert_eq!(
        endpoint.send(ALICE, msg(1)),
        Ok(Some(SERVER)),
        "the refused receive did not replace SERVER"
    );
    assert_eq!(endpoint.receive(SERVER), Ok(Some((ALICE, msg(1)))));
    assert_eq!(
        endpoint.receive(BOB),
        Ok(None),
        "nobody is waiting now, so BOB may wait"
    );
}

#[test]
fn new_is_a_const_fn() {
    let mut endpoint = made_in_a_const_fn();
    assert_eq!(endpoint.send(ALICE, Message::default()), Ok(None));
    assert_eq!(
        endpoint.receive(SERVER),
        Ok(Some((
            ALICE,
            Message {
                label: 0,
                words: [0; 4]
            }
        ))),
        "an endpoint made in a const fn works like any other; the default message is all zeros"
    );
}
