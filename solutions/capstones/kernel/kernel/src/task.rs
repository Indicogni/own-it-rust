//! Kernel tasks: stacks, saved contexts and the context switch.
//!
//! The scheduling *policy* (who runs next) is `kcore::Scheduler`, tested on
//! the host. This file is the *mechanism*: it keeps one saved [`Context`]
//! and one stack per task and moves the CPU from one to the other.

use alloc::boxed::Box;
use core::arch::naked_asm;
use core::mem::{MaybeUninit, offset_of};

use kcore::{RingBuffer, Scheduler, TaskId, TaskState};

use crate::cpu::{self, IrqGuard};
use crate::sync::IrqLock;

/// Tasks that can ever be created, the boot task included.
pub const MAX_TASKS: usize = 16;
const STACK_SIZE: usize = 64 * 1024;
/// How many pre-emptions the trace remembers.
const TRACE_LEN: usize = 16;

/// The registers a task needs back when it continues.
///
/// A switch always happens inside a function call (`context_switch`), and
/// the procedure call standard already makes the *caller* of any function
/// expect x0..x18 and most floating-point registers to be overwritten. So
/// only the callee-saved registers have to be kept: x19..x28, the frame
/// pointer x29, the stack pointer, and the low halves of v8..v15. The link
/// register x30 is kept too: it is the address at which the task continues.
#[repr(C)]
struct Context {
    x19_x28: [u64; 10],
    fp: u64,
    lr: u64,
    sp: u64,
    d8_d15: [u64; 8],
}

const _: () = {
    assert!(offset_of!(Context, fp) == 80);
    assert!(offset_of!(Context, sp) == 96);
    assert!(offset_of!(Context, d8_d15) == 104);
};

impl Context {
    const ZERO: Self = Self {
        x19_x28: [0; 10],
        fp: 0,
        lr: 0,
        sp: 0,
        d8_d15: [0; 8],
    };
}

struct Task {
    name: &'static str,
    context: Context,
    /// The task's stack. `None` for the boot task (its stack comes from the
    /// linker script) and after the stack has been freed by `join`.
    stack: Option<Box<[MaybeUninit<u8>]>>,
    /// The task blocked in `join` on this task, if any.
    joiner: Option<TaskId>,
    /// How often the timer took the CPU away from this task.
    preemptions: u32,
}

impl Task {
    const UNUSED: Self = Self {
        name: "",
        context: Context::ZERO,
        stack: None,
        joiner: None,
        preemptions: 0,
    };
}

struct Tasks {
    scheduler: Scheduler<MAX_TASKS>,
    table: [Task; MAX_TASKS],
    preemption: bool,
    /// The first pre-emptions since `set_preemption(true)`: (from, to).
    trace: RingBuffer<(TaskId, TaskId), TRACE_LEN>,
}

static TASKS: IrqLock<Tasks> = IrqLock::new(Tasks {
    scheduler: Scheduler::new(),
    table: {
        let mut table = [const { Task::UNUSED }; MAX_TASKS];
        table[0].name = "boot";
        table
    },
    preemption: false,
    trace: RingBuffer::new(),
});

/// Saves the callee-saved registers into `*prev`, loads them from `*next`
/// and returns on the stack of the next task.
///
/// For a task that has run before, the `ret` returns from *its* earlier
/// call of this function. For a new task it "returns" into `task_start`.
///
/// # Safety
///
/// Both pointers must be valid, `*next` must hold either a context saved by
/// this function or the initial context built by `spawn`, and IRQs must be
/// masked.
// SAFETY: a naked function gets no compiler-generated code, so the body must
// follow the calling convention by itself. It does: it reads its two
// arguments from x0 and x1, uses only x9 (which every caller expects to
// lose) as scratch space, and ends in `ret`.
#[unsafe(naked)]
unsafe extern "C" fn context_switch(prev: *mut Context, next: *const Context) {
    naked_asm!(
        "stp x19, x20, [x0, #0]",
        "stp x21, x22, [x0, #16]",
        "stp x23, x24, [x0, #32]",
        "stp x25, x26, [x0, #48]",
        "stp x27, x28, [x0, #64]",
        "stp x29, x30, [x0, #80]",
        "mov x9, sp",
        "str x9, [x0, #96]",
        "stp d8, d9, [x0, #104]",
        "stp d10, d11, [x0, #120]",
        "stp d12, d13, [x0, #136]",
        "stp d14, d15, [x0, #152]",
        "ldp x19, x20, [x1, #0]",
        "ldp x21, x22, [x1, #16]",
        "ldp x23, x24, [x1, #32]",
        "ldp x25, x26, [x1, #48]",
        "ldp x27, x28, [x1, #64]",
        "ldp x29, x30, [x1, #80]",
        "ldr x9, [x1, #96]",
        "mov sp, x9",
        "ldp d8, d9, [x1, #104]",
        "ldp d10, d11, [x1, #120]",
        "ldp d12, d13, [x1, #136]",
        "ldp d14, d15, [x1, #152]",
        "ret",
    )
}

/// Where a new task starts. `spawn` puts the entry function in x19 and its
/// argument in x20, because those are registers `context_switch` restores.
///
/// # Safety
///
/// Only reachable through the initial context built by `spawn`; never call it.
// SAFETY: the body is three instructions that set up the two arguments of
// `task_main` as the calling convention requires and jump to it. It needs
// no stack frame and never returns.
#[unsafe(naked)]
unsafe extern "C" fn task_start() -> ! {
    naked_asm!(
        "mov x0, x19",
        "mov x1, x20",
        "b {main}",
        main = sym task_main,
    )
}

extern "C" fn task_main(entry: usize, arg: usize) -> ! {
    // SAFETY: `entry` is the value that `spawn` produced by casting a
    // `fn(usize)` to `usize`; `task_start` passes it on unchanged. Function
    // pointers and `usize` have the same size on this target.
    let entry = unsafe { core::mem::transmute::<usize, fn(usize)>(entry) };
    // Whoever switched to this task had IRQs masked. A new task starts
    // with them enabled.
    cpu::irq_enable();
    entry(arg);
    exit()
}

/// Runs `decide` on the scheduler state and performs the switch it asks
/// for. Returns `false` when `decide` returned `None` (no switch).
fn reschedule(decide: impl FnOnce(&mut Tasks) -> Option<TaskId>) -> bool {
    // Masked from the decision until this task runs again: nothing may
    // observe the moment where the scheduler already says "next" but the
    // CPU still runs "prev".
    let _irq = IrqGuard::new();
    let pair = TASKS.lock(|tasks| {
        let prev = tasks.scheduler.current();
        let next = decide(tasks)?;
        let prev_context = &raw mut tasks.table[prev.0].context;
        let next_context = &raw const tasks.table[next.0].context;
        Some((prev_context, next_context))
    });
    let Some((prev_context, next_context)) = pair else {
        return false;
    };
    // SAFETY: both pointers refer to elements of the static task table, so
    // they are valid, and they are distinct because the scheduler never
    // names the running task as the next one. IRQs are masked and the lock
    // closure has returned, so no other code touches the two contexts
    // during the switch. `next_context` was filled by an earlier switch
    // away from that task or by `spawn`.
    unsafe { context_switch(prev_context, next_context) };
    true
}

/// Creates a task that will run `entry(arg)` on its own stack.
pub fn spawn(name: &'static str, entry: fn(usize), arg: usize) -> TaskId {
    let stack = Box::new_uninit_slice(STACK_SIZE);
    // The stack grows down, so the task starts at the high end, rounded
    // down to the 16-byte alignment that AArch64 requires.
    let top = stack.as_ptr_range().end.addr() & !0xf;
    TASKS.lock(|tasks| {
        let id = tasks.scheduler.spawn().expect("task table is full");
        let mut context = Context::ZERO;
        context.x19_x28[0] = entry as usize as u64;
        context.x19_x28[1] = arg as u64;
        context.lr = task_start as *const () as usize as u64;
        context.sp = top as u64;
        tasks.table[id.0] = Task {
            name,
            context,
            stack: Some(stack),
            joiner: None,
            preemptions: 0,
        };
        id
    })
}

/// The id of the calling task.
pub fn current() -> TaskId {
    TASKS.lock(|tasks| tasks.scheduler.current())
}

/// Lets the next ready task run. Returns when it is this task's turn again.
pub fn yield_now() {
    reschedule(|tasks| tasks.scheduler.yield_current());
}

/// Stops the calling task until another task calls [`wake`] for it.
///
/// Callers mask IRQs around "check the condition, then block", otherwise
/// the wake-up could arrive between the two and be lost.
pub fn block_current() {
    let switched = reschedule(|tasks| tasks.scheduler.block_current());
    assert!(switched, "deadlock: a task blocks and no task is ready");
}

/// Makes a blocked task ready. Returns `false` if it was not blocked.
pub fn wake(id: TaskId) -> bool {
    TASKS.lock(|tasks| tasks.scheduler.wake(id))
}

/// Ends the calling task.
pub fn exit() -> ! {
    cpu::irq_disable();
    TASKS.lock(|tasks| {
        let me = tasks.scheduler.current();
        if let Some(joiner) = tasks.table[me.0].joiner.take() {
            tasks.scheduler.wake(joiner);
        }
    });
    reschedule(|tasks| tasks.scheduler.exit_current());
    // `reschedule` returns only if nothing was ready to run.
    panic!("deadlock: the last runnable task exited");
}

/// Waits until task `id` has exited, then frees its stack.
pub fn join(id: TaskId) {
    let _irq = IrqGuard::new();
    loop {
        let exited = TASKS.lock(|tasks| match tasks.scheduler.state(id) {
            Some(TaskState::Exited) => true,
            Some(_) => {
                tasks.table[id.0].joiner = Some(tasks.scheduler.current());
                false
            }
            None => panic!("join: no such task"),
        });
        if exited {
            break;
        }
        block_current();
    }
    // A task cannot free the stack it is running on, so the joiner does it.
    // The box is dropped outside the lock: freeing takes the heap lock.
    let stack = TASKS.lock(|tasks| tasks.table[id.0].stack.take());
    drop(stack);
}

/// How often the timer has pre-empted the calling task.
pub fn preemptions() -> u32 {
    TASKS.lock(|tasks| tasks.table[tasks.scheduler.current().0].preemptions)
}

/// Turns timer-driven task switches on or off. Turning them on clears the
/// trace.
pub fn set_preemption(enabled: bool) {
    TASKS.lock(|tasks| {
        tasks.preemption = enabled;
        if enabled {
            while tasks.trace.pop().is_some() {}
        }
    });
}

/// Removes the oldest recorded pre-emption and returns the names of the
/// two tasks: (pre-empted task, task that ran next).
pub fn next_trace_entry() -> Option<(&'static str, &'static str)> {
    TASKS.lock(|tasks| {
        let (from, to) = tasks.trace.pop()?;
        Some((tasks.table[from.0].name, tasks.table[to.0].name))
    })
}

/// Called from the timer interrupt handler, after the interrupt has been
/// completed at the controller.
///
/// The switch happens in the middle of the handler. The pre-empted task's
/// trap frame stays on its own stack; when the task is scheduled again,
/// `context_switch` returns here, the handler returns, and the entry code
/// restores the frame.
pub fn on_tick() {
    reschedule(|tasks| {
        if !tasks.preemption {
            return None;
        }
        let prev = tasks.scheduler.current();
        let next = tasks.scheduler.yield_current()?;
        tasks.table[prev.0].preemptions += 1;
        // A full trace simply stops recording.
        let _ = tasks.trace.push((prev, next));
        Some(next)
    });
}
