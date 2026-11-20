//! The round-robin scheduling policy, with no machine code in it.
//!
//! The scheduler only tracks *which* task should run. Saving and restoring
//! registers is the kernel's job: it calls a method here, and when the
//! method names a different task the kernel performs the context switch.

use crate::ring::RingBuffer;

/// Names one task. Ids are handed out in order (0, 1, 2, ...) and are never
/// reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    /// Waiting in the ready queue for its turn.
    Ready,
    /// The task that owns the CPU. Exactly one task is in this state.
    Running,
    /// Waiting for an event; not in the ready queue.
    Blocked,
    /// Finished. It will never run again.
    Exited,
}

/// A round-robin scheduler for at most `N` tasks, counted over its whole
/// life.
///
/// Invariants: exactly one task is `Running` and `current` names it; the
/// ready queue holds exactly the `Ready` tasks, each once, in the order in
/// which they became ready.
pub struct Scheduler<const N: usize> {
    states: [Option<TaskState>; N],
    ready: RingBuffer<TaskId, N>,
    current: TaskId,
    spawned: usize,
}

impl<const N: usize> Scheduler<N> {
    /// Creates a scheduler in which task 0 already exists and is running.
    /// Task 0 stands for the code that called `new`: the boot code.
    pub const fn new() -> Self {
        assert!(N > 0, "a scheduler needs room for the boot task");
        let mut states = [None; N];
        states[0] = Some(TaskState::Running);
        Self {
            states,
            ready: RingBuffer::new(),
            current: TaskId(0),
            spawned: 1,
        }
    }

    /// Creates a task in state `Ready` at the back of the ready queue.
    /// Returns `None` once `N` ids have been handed out.
    pub fn spawn(&mut self) -> Option<TaskId> {
        if self.spawned == N {
            return None;
        }
        let id = TaskId(self.spawned);
        self.spawned += 1;
        self.states[id.0] = Some(TaskState::Ready);
        self.enqueue(id);
        Some(id)
    }

    /// The running task.
    pub fn current(&self) -> TaskId {
        self.current
    }

    /// The state of `id`, or `None` when no such task was ever created.
    pub fn state(&self, id: TaskId) -> Option<TaskState> {
        self.states.get(id.0).copied().flatten()
    }

    /// The running task gives up the CPU but stays runnable.
    ///
    /// Returns the task that runs next. `None` means that no other task is
    /// ready and the caller simply keeps running.
    pub fn yield_current(&mut self) -> Option<TaskId> {
        let next = self.ready.pop()?;
        let prev = self.current;
        self.states[prev.0] = Some(TaskState::Ready);
        self.enqueue(prev);
        Some(self.run(next))
    }

    /// The running task starts to wait for an event.
    ///
    /// Returns the task that runs next. `None` means that no task is ready;
    /// in that case nothing changes, and the kernel has to treat the
    /// situation as a deadlock.
    pub fn block_current(&mut self) -> Option<TaskId> {
        let next = self.ready.pop()?;
        self.states[self.current.0] = Some(TaskState::Blocked);
        Some(self.run(next))
    }

    /// The running task ends. Same return value as [`Self::block_current`].
    pub fn exit_current(&mut self) -> Option<TaskId> {
        let next = self.ready.pop()?;
        self.states[self.current.0] = Some(TaskState::Exited);
        Some(self.run(next))
    }

    /// Makes a blocked task ready again, at the back of the queue.
    /// Returns `false`, and changes nothing, when `id` is not blocked.
    pub fn wake(&mut self, id: TaskId) -> bool {
        if self.state(id) != Some(TaskState::Blocked) {
            return false;
        }
        self.states[id.0] = Some(TaskState::Ready);
        self.enqueue(id);
        true
    }

    fn run(&mut self, next: TaskId) -> TaskId {
        self.states[next.0] = Some(TaskState::Running);
        self.current = next;
        next
    }

    fn enqueue(&mut self, id: TaskId) {
        // At most N - 1 tasks can be ready at once (one is running), so the
        // queue of capacity N cannot be full here.
        let pushed = self.ready.push(id);
        debug_assert!(pushed.is_ok(), "ready queue overflow");
    }
}

impl<const N: usize> Default for Scheduler<N> {
    fn default() -> Self {
        Self::new()
    }
}
