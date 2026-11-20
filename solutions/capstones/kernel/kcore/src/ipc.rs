//! The state machine of one synchronous IPC endpoint.
//!
//! The protocol has three steps, as in classic message-passing kernels:
//!
//! 1. a client *sends* a request and stops running;
//! 2. the server *receives* the request and learns who sent it;
//! 3. the server *replies* to that client, which then continues.
//!
//! This module records who is waiting for what. It never blocks or wakes a
//! task itself: each method tells the kernel what to do with the scheduler.

use crate::sched::TaskId;

/// A small fixed-size message: a label that says what the message means and
/// four payload words. Messages are copied; they never hold pointers that
/// the receiver must trust.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub label: u64,
    pub words: [u64; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcError {
    /// `N` calls are already in progress on this endpoint.
    Full,
    /// The sender already has a call in progress on this endpoint.
    CallInProgress,
    /// Another task is already waiting to receive on this endpoint.
    ReceiverBusy,
    /// `reply` named a task whose request has not been received.
    NotAwaitingReply,
}

/// What one slot of the endpoint knows about one call.
#[derive(Clone, Copy)]
enum Call {
    Free,
    /// Sent, not yet received. `seq` orders the queue: lowest first.
    Queued {
        from: TaskId,
        msg: Message,
        seq: u64,
    },
    /// Received by the server; the client waits for the reply.
    AwaitingReply {
        from: TaskId,
    },
    /// Replied; the client has not collected the reply yet.
    Replied {
        to: TaskId,
        msg: Message,
    },
}

impl Call {
    fn client(&self) -> Option<TaskId> {
        match *self {
            Call::Free => None,
            Call::Queued { from, .. } | Call::AwaitingReply { from } => Some(from),
            Call::Replied { to, .. } => Some(to),
        }
    }
}

/// One endpoint with room for `N` calls in progress.
pub struct Endpoint<const N: usize> {
    calls: [Call; N],
    /// A server that found no request and is blocked in `receive`.
    receiver: Option<TaskId>,
    next_seq: u64,
}

impl<const N: usize> Endpoint<N> {
    pub const fn new() -> Self {
        Self {
            calls: [Call::Free; N],
            receiver: None,
            next_seq: 0,
        }
    }

    /// Task `from` sends `msg`. After a successful send the kernel must
    /// block `from` until [`Self::take_reply`] returns its reply.
    ///
    /// `Ok(Some(server))` means that `server` was blocked in `receive` and
    /// must be woken; it will find the request when it calls `receive`
    /// again.
    pub fn send(&mut self, from: TaskId, msg: Message) -> Result<Option<TaskId>, IpcError> {
        if self.calls.iter().any(|call| call.client() == Some(from)) {
            return Err(IpcError::CallInProgress);
        }
        let slot = self
            .calls
            .iter_mut()
            .find(|call| matches!(call, Call::Free))
            .ok_or(IpcError::Full)?;
        *slot = Call::Queued {
            from,
            msg,
            seq: self.next_seq,
        };
        self.next_seq += 1;
        Ok(self.receiver.take())
    }

    /// Task `me` asks for the oldest request.
    ///
    /// `Ok(Some((client, msg)))` hands over a request; `client` now waits
    /// for a reply. `Ok(None)` means that no request is queued: the
    /// endpoint remembers `me`, the kernel must block it, and after it is
    /// woken it calls `receive` again.
    pub fn receive(&mut self, me: TaskId) -> Result<Option<(TaskId, Message)>, IpcError> {
        if self.receiver.is_some_and(|waiting| waiting != me) {
            return Err(IpcError::ReceiverBusy);
        }
        let oldest = self
            .calls
            .iter_mut()
            .filter_map(|call| match *call {
                Call::Queued { seq, .. } => Some((seq, call)),
                _ => None,
            })
            .min_by_key(|&(seq, _)| seq);
        let Some((_, slot)) = oldest else {
            self.receiver = Some(me);
            return Ok(None);
        };
        let Call::Queued { from, msg, .. } = *slot else {
            unreachable!("the filter above only keeps queued calls");
        };
        *slot = Call::AwaitingReply { from };
        self.receiver = None;
        Ok(Some((from, msg)))
    }

    /// Stores the reply for client `to`. After `Ok(())` the kernel must
    /// wake `to`.
    pub fn reply(&mut self, to: TaskId, msg: Message) -> Result<(), IpcError> {
        let slot = self
            .calls
            .iter_mut()
            .find(|call| matches!(call, Call::AwaitingReply { from } if *from == to))
            .ok_or(IpcError::NotAwaitingReply)?;
        *slot = Call::Replied { to, msg };
        Ok(())
    }

    /// Collects the reply for client `me`, if it has arrived. This ends the
    /// call and frees its slot.
    pub fn take_reply(&mut self, me: TaskId) -> Option<Message> {
        let slot = self
            .calls
            .iter_mut()
            .find(|call| matches!(call, Call::Replied { to, .. } if *to == me))?;
        let Call::Replied { msg, .. } = *slot else {
            unreachable!("the search above only matches replied calls");
        };
        *slot = Call::Free;
        Some(msg)
    }
}

impl<const N: usize> Default for Endpoint<N> {
    fn default() -> Self {
        Self::new()
    }
}
