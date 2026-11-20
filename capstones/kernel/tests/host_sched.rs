//! Host acceptance tests for `kcore::Scheduler` (SPEC 5.3).
//!
//! ```text
//! cargo test --test host_sched
//! ```

use std::collections::VecDeque;

use harness::within;
use kcore::{Scheduler, TaskId, TaskState};

/// SPEC 5.3: `new` is a `const fn`; the kernel keeps its scheduler in a
/// `static`. This function compiles only if `new` is a `const fn`.
const fn made_in_a_const_fn() -> Scheduler<4> {
    Scheduler::new()
}

#[test]
fn the_boot_task_exists_and_runs() {
    let scheduler: Scheduler<4> = Scheduler::new();
    assert_eq!(
        scheduler.current(),
        TaskId(0),
        "task 0 is the code that created the scheduler"
    );
    assert_eq!(scheduler.state(TaskId(0)), Some(TaskState::Running));
    assert_eq!(scheduler.state(TaskId(1)), None, "no other task exists yet");
    assert_eq!(
        scheduler.state(TaskId(99)),
        None,
        "an id beyond N is simply unknown, not a panic"
    );
}

#[test]
fn spawn_hands_out_ids_in_order_until_n_are_used() {
    let mut scheduler: Scheduler<4> = Scheduler::new();
    for expected in 1..4 {
        assert_eq!(
            scheduler.spawn(),
            Some(TaskId(expected)),
            "ids are 1, 2, 3 in order"
        );
        assert_eq!(
            scheduler.state(TaskId(expected)),
            Some(TaskState::Ready),
            "a new task is ready"
        );
    }
    assert_eq!(
        scheduler.spawn(),
        None,
        "N = 4 and four ids (0..=3) are used"
    );
    assert_eq!(
        scheduler.current(),
        TaskId(0),
        "spawning never switches tasks"
    );
}

#[test]
fn yield_with_nothing_ready_keeps_the_caller_running() {
    let mut scheduler: Scheduler<2> = Scheduler::new();
    assert_eq!(scheduler.yield_current(), None, "no other task: no switch");
    assert_eq!(scheduler.current(), TaskId(0));
    assert_eq!(scheduler.state(TaskId(0)), Some(TaskState::Running));
}

#[test]
fn yield_is_round_robin() {
    let mut scheduler: Scheduler<4> = Scheduler::new();
    let a = scheduler.spawn().expect("room for a");
    let b = scheduler.spawn().expect("room for b");

    assert_eq!(
        scheduler.yield_current(),
        Some(a),
        "the task that became ready first runs first"
    );
    assert_eq!(
        scheduler.current(),
        a,
        "the scheduler's current task follows the decision"
    );
    assert_eq!(scheduler.state(a), Some(TaskState::Running));
    assert_eq!(
        scheduler.state(TaskId(0)),
        Some(TaskState::Ready),
        "the yielding task stays runnable"
    );

    let order: Vec<TaskId> = (0..5)
        .map(|_| scheduler.yield_current().expect("others are ready"))
        .collect();
    assert_eq!(
        order,
        [b, TaskId(0), a, b, TaskId(0)],
        "a yielding task goes to the back of the queue, so the tasks take turns"
    );
}

#[test]
fn a_blocked_task_does_not_run_until_it_is_woken() {
    let mut scheduler: Scheduler<4> = Scheduler::new();
    let a = scheduler.spawn().expect("room for a");
    let b = scheduler.spawn().expect("room for b");

    assert_eq!(
        scheduler.block_current(),
        Some(a),
        "blocking hands the CPU to the next ready task"
    );
    assert_eq!(scheduler.state(TaskId(0)), Some(TaskState::Blocked));
    assert_eq!(scheduler.yield_current(), Some(b));
    assert_eq!(
        scheduler.yield_current(),
        Some(a),
        "the blocked task 0 is skipped"
    );
    assert_eq!(scheduler.yield_current(), Some(b), "... every time");

    assert!(scheduler.wake(TaskId(0)), "waking a blocked task succeeds");
    assert_eq!(scheduler.state(TaskId(0)), Some(TaskState::Ready));
    assert_eq!(
        scheduler.current(),
        b,
        "waking never switches tasks by itself"
    );
    assert_eq!(
        scheduler.yield_current(),
        Some(a),
        "a woken task queues behind the tasks already ready"
    );
    assert_eq!(
        scheduler.yield_current(),
        Some(TaskId(0)),
        "then it gets its turn"
    );
}

#[test]
fn wake_refuses_tasks_that_are_not_blocked() {
    let mut scheduler: Scheduler<4> = Scheduler::new();
    let a = scheduler.spawn().expect("room for a");
    assert!(
        !scheduler.wake(TaskId(0)),
        "the running task is not blocked"
    );
    assert!(!scheduler.wake(a), "a ready task is not blocked");
    assert!(
        !scheduler.wake(TaskId(3)),
        "a task that does not exist is not blocked"
    );
    assert!(
        !scheduler.wake(TaskId(1000)),
        "an id beyond N is not blocked"
    );
    assert_eq!(
        scheduler.yield_current(),
        Some(a),
        "a refused wake must not put a second copy of the task in the ready queue"
    );
    assert_eq!(scheduler.yield_current(), Some(TaskId(0)));
    assert_eq!(
        scheduler.yield_current(),
        Some(a),
        "still exactly two tasks take turns"
    );

    assert_eq!(scheduler.block_current(), Some(TaskId(0)));
    assert!(scheduler.wake(a), "first wake of a blocked task succeeds");
    assert!(
        !scheduler.wake(a),
        "the task is ready now; a second wake is refused"
    );
}

#[test]
fn blocking_with_nothing_ready_changes_nothing() {
    let mut scheduler: Scheduler<2> = Scheduler::new();
    assert_eq!(
        scheduler.block_current(),
        None,
        "no task could run instead: the scheduler reports the deadlock with None"
    );
    assert_eq!(
        scheduler.current(),
        TaskId(0),
        "after None the caller is still the current task"
    );
    assert_eq!(
        scheduler.state(TaskId(0)),
        Some(TaskState::Running),
        "... and still running"
    );
    assert_eq!(
        scheduler.exit_current(),
        None,
        "the same rule holds for exit"
    );
    assert_eq!(scheduler.state(TaskId(0)), Some(TaskState::Running));
}

#[test]
fn an_exited_task_never_runs_again_and_its_id_is_not_reused() {
    let mut scheduler: Scheduler<4> = Scheduler::new();
    let a = scheduler.spawn().expect("room for a");
    let b = scheduler.spawn().expect("room for b");

    assert_eq!(scheduler.yield_current(), Some(a));
    assert_eq!(
        scheduler.exit_current(),
        Some(b),
        "exit hands the CPU to the next ready task"
    );
    assert_eq!(scheduler.state(a), Some(TaskState::Exited));
    assert!(!scheduler.wake(a), "an exited task cannot be woken");
    assert_eq!(scheduler.yield_current(), Some(TaskId(0)));
    assert_eq!(
        scheduler.yield_current(),
        Some(b),
        "only the two live tasks take turns"
    );

    assert_eq!(
        scheduler.spawn(),
        Some(TaskId(3)),
        "the next id is 3: the id of an exited task is not reused"
    );
    assert_eq!(
        scheduler.spawn(),
        None,
        "N counts tasks over the scheduler's whole life"
    );
}

/// The cooperative demo of milestone 5, replayed on the bare state machine:
/// the boot task spawns A, B and C and joins A; each task yields once.
#[test]
fn milestone_5_cooperative_order() {
    let mut scheduler: Scheduler<8> = Scheduler::new();
    let boot = TaskId(0);
    let [a, b, c] = [(); 3].map(|()| scheduler.spawn().expect("room"));
    let name = |id: TaskId| ["boot", "A", "B", "C"][id.0];
    let mut log = Vec::new();

    // boot: join(A) blocks.
    let mut running = scheduler.block_current().expect("A is ready");
    // Each task: print, yield, print, exit. Six prints in total.
    let mut rounds = [0_usize; 4];
    while running != boot {
        log.push(format!("{}{}", name(running), rounds[running.0]));
        rounds[running.0] += 1;
        running = if rounds[running.0] < 2 {
            scheduler.yield_current().expect("others are ready")
        } else {
            if running == a {
                assert!(scheduler.wake(boot), "A's exit wakes the joiner");
            }
            scheduler
                .exit_current()
                .expect("boot or another task is ready")
        };
    }
    assert_eq!(
        log,
        ["A0", "B0", "C0", "A1", "B1", "C1"],
        "this is the order the transcript of SPEC 4.1 requires"
    );
    for task in [a, b, c] {
        assert_eq!(scheduler.state(task), Some(TaskState::Exited));
    }
}

#[test]
fn agrees_with_a_simple_model_over_a_long_random_run() {
    // Miri runs this about a hundred times slower than native code.
    let steps = if cfg!(miri) { 200 } else { 5_000 };
    within(60, "scheduler random run", move || {
        const N: usize = 12;
        let mut scheduler: Scheduler<N> = Scheduler::new();
        // The model: every state, plus the ready queue in order.
        let mut states: Vec<TaskState> = vec![TaskState::Running];
        let mut ready: VecDeque<usize> = VecDeque::new();
        let mut current = 0_usize;

        let mut seed: u32 = 0x1234_5678;
        let mut next = |bound: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % bound
        };

        for step in 0..steps {
            match next(6) {
                0 => {
                    let expected = (states.len() < N).then(|| {
                        states.push(TaskState::Ready);
                        ready.push_back(states.len() - 1);
                        TaskId(states.len() - 1)
                    });
                    assert_eq!(scheduler.spawn(), expected, "step {step}: spawn");
                }
                1 | 2 => {
                    let expected = ready.pop_front().map(|next_task| {
                        states[current] = TaskState::Ready;
                        ready.push_back(current);
                        states[next_task] = TaskState::Running;
                        current = next_task;
                        TaskId(next_task)
                    });
                    assert_eq!(
                        scheduler.yield_current(),
                        expected,
                        "step {step}: yield_current"
                    );
                }
                3 => {
                    let expected = ready.pop_front().map(|next_task| {
                        states[current] = TaskState::Blocked;
                        states[next_task] = TaskState::Running;
                        current = next_task;
                        TaskId(next_task)
                    });
                    assert_eq!(
                        scheduler.block_current(),
                        expected,
                        "step {step}: block_current"
                    );
                }
                4 => {
                    let id = next(N as u32 + 2) as usize;
                    let expected = states.get(id) == Some(&TaskState::Blocked);
                    if expected {
                        states[id] = TaskState::Ready;
                        ready.push_back(id);
                    }
                    assert_eq!(
                        scheduler.wake(TaskId(id)),
                        expected,
                        "step {step}: wake({id})"
                    );
                }
                _ => {
                    // Exit rarely, so the run does not die out early.
                    if next(4) == 0 {
                        let expected = ready.pop_front().map(|next_task| {
                            states[current] = TaskState::Exited;
                            states[next_task] = TaskState::Running;
                            current = next_task;
                            TaskId(next_task)
                        });
                        assert_eq!(
                            scheduler.exit_current(),
                            expected,
                            "step {step}: exit_current"
                        );
                    }
                }
            }
            assert_eq!(scheduler.current(), TaskId(current), "step {step}: current");
            for id in 0..N + 1 {
                assert_eq!(
                    scheduler.state(TaskId(id)),
                    states.get(id).copied(),
                    "step {step}: state of task {id}"
                );
            }
        }
    });
}

#[test]
fn new_is_a_const_fn() {
    let mut scheduler = made_in_a_const_fn();
    let task = scheduler.spawn().expect("room for one task");
    assert_eq!(
        scheduler.yield_current(),
        Some(task),
        "a scheduler made in a const fn works like any other"
    );
}
