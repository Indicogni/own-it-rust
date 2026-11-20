//! Synchronous message passing between tasks.
//!
//! `kcore::Endpoint` knows who waits for what. This file connects that
//! state machine to the scheduler: it blocks the caller when the state
//! machine says "wait" and wakes the task the state machine names.

use kcore::{Endpoint, IpcError, Message, TaskId};

use crate::cpu::IrqGuard;
use crate::sync::IrqLock;
use crate::task;

/// Endpoints in the system. They are created at boot and never destroyed.
const ENDPOINTS: usize = 4;
/// Calls that can be in progress on one endpoint.
const CALLS: usize = 8;

/// Names one endpoint.
#[derive(Clone, Copy)]
pub struct EndpointId(pub usize);

static TABLE: IrqLock<[Endpoint<CALLS>; ENDPOINTS]> =
    IrqLock::new([const { Endpoint::new() }; ENDPOINTS]);

/// Sends `request` and waits for the reply.
pub fn send(endpoint: EndpointId, request: Message) -> Result<Message, IpcError> {
    // IRQs stay masked from "queue the request" to "block". Otherwise the
    // timer could switch to the server in between, the server could reply
    // and wake this task while it is still running, and the wake-up would
    // be lost.
    let _irq = IrqGuard::new();
    let me = task::current();
    let waiting_server = TABLE.lock(|table| table[endpoint.0].send(me, request))?;
    if let Some(server) = waiting_server {
        task::wake(server);
    }
    loop {
        if let Some(reply) = TABLE.lock(|table| table[endpoint.0].take_reply(me)) {
            return Ok(reply);
        }
        task::block_current();
    }
}

/// Waits for the next request on `endpoint`. Returns the client to reply
/// to and its message.
pub fn receive(endpoint: EndpointId) -> Result<(TaskId, Message), IpcError> {
    let _irq = IrqGuard::new();
    let me = task::current();
    loop {
        if let Some(request) = TABLE.lock(|table| table[endpoint.0].receive(me))? {
            return Ok(request);
        }
        task::block_current();
    }
}

/// Answers the request of `client` and lets it continue.
pub fn reply(endpoint: EndpointId, client: TaskId, reply: Message) -> Result<(), IpcError> {
    let _irq = IrqGuard::new();
    TABLE.lock(|table| table[endpoint.0].reply(client, reply))?;
    task::wake(client);
    Ok(())
}
