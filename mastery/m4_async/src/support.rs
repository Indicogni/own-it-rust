//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`within`] turns a test that would hang into a test that fails with a message.
//! - [`alloc`] counts heap allocations, so a test can check *how* you solved a task.
//! - [`block_on`] runs one future on the current thread. It is the executor of
//!   Chapter 21, lesson 04, so that every lesson can be tested without the
//!   executor you build in lesson 02. It gives up with a message when the
//!   future is pending and nobody wakes it.
//! - [`yield_now`] gives other tasks one turn.
//! - [`poll_to_end`] polls one future by hand, a limited number of times.
//! - [`WakeCounter`] and [`WakeLog`] are wakers that record their calls.
//! - [`Gate`] is a future that stays pending until a test opens it.
//! - [`Anchored`] is a future that notices when it is moved after its first poll.
//! - [`DropLog`] records the order in which values are dropped.
//! - [`implements!`](crate::implements) answers "does this type implement
//!   this trait?" with a `bool`.

use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread, ThreadId};
use std::time::{Duration, Instant};

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test
/// with a clear message if it has not finished after `secs` seconds.
///
/// Tests use this around code that would otherwise block forever when a
/// solution has a deadlock or never wakes a task. `what` names the thing
/// being run and appears in the failure message.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::RecvTimeoutError;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a task that is never woken, a lock that is taken twice on one \
             thread, or a driver thread that never stops."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

/// `implements!(Type: Bound)` is `true` when the concrete type `Type`
/// satisfies `Bound`, and `false` otherwise.
///
/// ```
/// use m4_async::implements;
/// use std::marker::PhantomPinned;
///
/// assert!(implements!(String: Unpin));
/// assert!(!implements!(PhantomPinned: Unpin));
/// ```
///
/// A bound that is not met is normally a compile error, and a test cannot
/// describe a compile error. This macro turns the question into a value.
///
/// How it works: `Probe<T>` has an inherent associated constant `YES` that
/// exists only when `T` meets the bound, and a trait constant `YES` that
/// always exists and is `false`. The path `<Probe<T>>::YES` prefers the
/// inherent constant when there is one. The type must be written out in
/// full; the macro cannot be used on a generic parameter.
#[macro_export]
macro_rules! implements {
    ($ty:ty : $($bound:tt)+) => {{
        struct Probe<T: ?Sized>(::core::marker::PhantomData<T>);
        trait Fallback {
            const YES: bool = false;
        }
        impl<T: ?Sized> Fallback for Probe<T> {}
        #[allow(dead_code)]
        impl<T: ?Sized + $($bound)+> Probe<T> {
            const YES: bool = true;
        }
        <Probe<$ty>>::YES
    }};
}

/// Locks a mutex and ignores poisoning. Support code never leaves its data
/// half-updated, so the data is usable even after another thread panicked.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub mod alloc {
    //! A global allocator that counts allocations per thread.
    //!
    //! `lib.rs` installs it for test builds. Counts only cover the calling
    //! thread, so tests that run in parallel do not disturb each other; for the
    //! same reason, call [`count`] *inside* a `within` closure, not around it.
    //! The optimizer may remove allocations in release builds, so tests only
    //! assert upper bounds.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    /// Forwards to the system allocator and counts allocations and growth.
    pub struct Counting;

    fn record() {
        let _ = ACTIVE.try_with(|active| {
            if active.get() {
                let _ = COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    // SAFETY: every method forwards unchanged to `System`, which upholds the
    // GlobalAlloc contract. Recording touches only const-initialized
    // thread-locals without destructors, so it never allocates or reenters.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, COUNT.with(Cell::get) - before)
    }
}

struct Unpark {
    thread: Thread,
    woken: AtomicBool,
}

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
        self.thread.unpark();
    }
}

/// How long [`block_on`] waits for a wake before it reports a lost wakeup.
const STALL_LIMIT: Duration = Duration::from_secs(10);

/// Runs one future to completion on the current thread. While the future is
/// pending the thread is parked, and the future's waker unparks it.
///
/// This executor has no task queue, no timers and no reactor. You build
/// those in this module. Because it runs tests, it does one thing that a
/// production executor does not: when the future has been pending for ten
/// seconds without a wake, it panics instead of sleeping for ever.
#[track_caller]
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let unpark = Arc::new(Unpark {
        thread: thread::current(),
        woken: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&unpark));
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        // The flag records a wake that arrived at any moment since the last
        // check, also during the poll, so no wake is lost. `park_timeout`
        // may return early for no reason; the loop then looks at the flag
        // and the clock again.
        let parked_at = Instant::now();
        while !unpark.woken.swap(false, Ordering::SeqCst) {
            let waited = parked_at.elapsed();
            assert!(
                waited < STALL_LIMIT,
                "a future returned Pending and was not woken for {} s. A wakeup was lost: \
                 every path that returns Pending must leave the waker with someone who \
                 will call it",
                STALL_LIMIT.as_secs()
            );
            thread::park_timeout(STALL_LIMIT - waited);
        }
    }
}

/// A future that returns `Pending` once, after asking to be polled again.
/// Awaiting it lets every other task that is ready run first.
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

/// The future returned by [`yield_now`].
#[derive(Debug)]
pub struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Polls `future` with `cx` until it is ready. Returns the output and the
/// number of polls that it took.
///
/// Tests use this where [`block_on`] does not fit: it allocates nothing and
/// never leaves the calling thread, so a test can count allocations or polls
/// around it. A future that is still pending after `max_polls` polls fails
/// the test with a message; a plain `loop` would spin for ever.
#[track_caller]
pub fn poll_to_end<F: Future>(
    mut future: Pin<&mut F>,
    cx: &mut Context<'_>,
    max_polls: usize,
) -> (F::Output, usize) {
    for polls in 1..=max_polls {
        if let Poll::Ready(output) = future.as_mut().poll(cx) {
            return (output, polls);
        }
    }
    panic!(
        "the future was polled {max_polls} times and was pending every time. This test polls \
         in a loop without waiting for a wake, so every poll should make progress: look for \
         a path that returns Pending without polling the future inside"
    )
}

/// A waker target that counts how often it was woken.
#[derive(Debug, Default)]
pub struct WakeCounter {
    wakes: AtomicUsize,
}

impl WakeCounter {
    /// A new counter and a waker that increments it.
    pub fn new() -> (Arc<WakeCounter>, Waker) {
        let counter = Arc::new(WakeCounter::default());
        let waker = Waker::from(Arc::clone(&counter));
        (counter, waker)
    }

    /// How many times a waker of this counter was woken.
    pub fn count(&self) -> usize {
        self.wakes.load(Ordering::SeqCst)
    }
}

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

/// A waker target that reports every wake through a channel, together with
/// the thread that made the call. A test can then *wait* for a wake with a
/// time limit instead of sleeping and hoping.
pub struct WakeLog {
    wakes: Mutex<Sender<ThreadId>>,
}

impl WakeLog {
    /// A waker, and the receiving end on which each wake arrives.
    pub fn pair() -> (Waker, Receiver<ThreadId>) {
        let (tx, rx) = mpsc::channel();
        let waker = Waker::from(Arc::new(WakeLog {
            wakes: Mutex::new(tx),
        }));
        (waker, rx)
    }
}

impl Wake for WakeLog {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        // The receiver may be gone when the test has ended; that is fine.
        let _ = lock(&self.wakes).send(thread::current().id());
    }
}

#[derive(Default)]
struct GateState {
    open: bool,
    polls: usize,
    waiters: Vec<Waker>,
}

/// A door that tests open by hand. [`Gate::wait`] gives a future that is
/// pending until [`Gate::open`] is called, from any thread.
#[derive(Clone, Default)]
pub struct Gate {
    state: Arc<Mutex<GateState>>,
}

impl Gate {
    /// A closed gate.
    pub fn new() -> Gate {
        Gate::default()
    }

    /// Opens the gate and wakes every future that waits for it.
    pub fn open(&self) {
        let waiters = {
            let mut state = lock(&self.state);
            state.open = true;
            std::mem::take(&mut state.waiters)
        };
        for waiter in waiters {
            waiter.wake();
        }
    }

    /// A future that completes once the gate is open.
    pub fn wait(&self) -> GateWait {
        GateWait { gate: self.clone() }
    }

    /// How many times futures of this gate have been polled.
    pub fn polls(&self) -> usize {
        lock(&self.state).polls
    }
}

/// The future returned by [`Gate::wait`].
pub struct GateWait {
    gate: Gate,
}

impl Future for GateWait {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = lock(&self.gate.state);
        state.polls += 1;
        if state.open {
            return Poll::Ready(());
        }
        // A test uses few waiters, so the list stays short even though a
        // future that is polled twice is listed twice.
        state.waiters.push(cx.waker().clone());
        Poll::Pending
    }
}

/// What an [`Anchored`] future has seen. Keep a clone to ask it afterwards.
#[derive(Clone, Debug, Default)]
pub struct AnchorReport {
    polls: Arc<AtomicUsize>,
    moved: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
}

impl AnchorReport {
    /// How many times the future was polled.
    pub fn polls(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }

    /// `true` if the future was at a different address in a later poll, or
    /// when it was dropped, than in its first poll.
    pub fn moved(&self) -> bool {
        self.moved.load(Ordering::SeqCst)
    }

    /// `true` once the future has been dropped.
    pub fn dropped(&self) -> bool {
        self.dropped.load(Ordering::SeqCst)
    }
}

/// A future that is `!Unpin` and checks the pinning promise: after its first
/// poll it must stay at the same address until it is dropped.
///
/// It returns `Pending` for the first `pending` polls (waking itself each
/// time, so any executor polls it again) and then returns `value`.
pub struct Anchored<T> {
    value: Cell<Option<T>>,
    pending: Cell<usize>,
    home: Cell<Option<usize>>,
    report: AnchorReport,
    _pinned: PhantomPinned,
}

impl<T> Anchored<T> {
    /// A future that is pending `pending` times and then gives `value`.
    pub fn new(pending: usize, value: T) -> (Anchored<T>, AnchorReport) {
        let report = AnchorReport::default();
        let future = Anchored {
            value: Cell::new(Some(value)),
            pending: Cell::new(pending),
            home: Cell::new(None),
            report: report.clone(),
            _pinned: PhantomPinned,
        };
        (future, report)
    }

    fn check_address(&self) {
        let here = std::ptr::from_ref(self) as usize;
        match self.home.get() {
            None => self.home.set(Some(here)),
            Some(home) if home != here => self.report.moved.store(true, Ordering::SeqCst),
            Some(_) => {}
        }
    }
}

impl<T> Future for Anchored<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // Every field is a `Cell`, so a shared reference is enough and this
        // `!Unpin` type needs no unsafe code.
        let this: &Self = &self;
        this.check_address();
        this.report.polls.fetch_add(1, Ordering::SeqCst);
        if this.pending.get() > 0 {
            this.pending.set(this.pending.get() - 1);
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match this.value.take() {
            Some(value) => Poll::Ready(value),
            None => panic!("an Anchored future was polled again after it returned Ready"),
        }
    }
}

impl<T> Drop for Anchored<T> {
    fn drop(&mut self) {
        if self.home.get().is_some() {
            self.check_address();
        }
        self.report.dropped.store(true, Ordering::SeqCst);
    }
}

/// Records events, usually drops, in the order in which they happen.
#[derive(Clone, Debug, Default)]
pub struct DropLog {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl DropLog {
    /// An empty log.
    pub fn new() -> DropLog {
        DropLog::default()
    }

    /// Adds an event by hand.
    pub fn push(&self, event: &'static str) {
        lock(&self.events).push(event);
    }

    /// A value that writes `name` into the log when it is dropped.
    pub fn guard(&self, name: &'static str) -> Noisy {
        Noisy {
            name,
            log: self.clone(),
        }
    }

    /// The events so far.
    pub fn events(&self) -> Vec<&'static str> {
        lock(&self.events).clone()
    }
}

/// A value that announces its own drop in a [`DropLog`].
#[derive(Debug)]
pub struct Noisy {
    name: &'static str,
    log: DropLog,
}

impl Noisy {
    /// The name that will be logged.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        self.log.push(self.name);
    }
}
