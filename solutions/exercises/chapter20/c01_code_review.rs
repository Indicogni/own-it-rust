//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter20 c01_
//! Then: cargo clippy --test chapter20
//!
//! This is the kind of code you will be asked to review at work. It compiles
//! without a warning, and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every await that holds something it should not, and every
//!    place where cancellation could leave state half-updated.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep all three signatures; the tests depend on them.
//! 4. Run clippy and resolve its warnings for this file.

use std::{cell::RefCell, collections::BTreeMap, future::Future, sync::Mutex};

/// A. Return the cached value for `key`. On a miss, await `load`, cache its
/// result, and return it. Other tasks polled while `load` is pending must be
/// able to use the cache.
async fn get_or_load(
    cache: &Mutex<BTreeMap<String, String>>,
    key: &str,
    load: impl Future<Output = String>,
) -> String {
    // The guard is a temporary of this one statement. It is released before
    // the await, so other tasks can use the cache while `load` is pending,
    // and the future stays `Send`. (`drop(guard)` before the await would
    // release the lock too, but the compiler would still count the guard as
    // stored across the await, and the future would not be `Send`.)
    if let Some(value) = cache.lock().unwrap().get(key) {
        return value.clone();
    }
    let value = load.await;
    // Lock again only for the insert. If another task stored a value for
    // this key while we were loading, keep that one and return it, so every
    // caller sees the same value.
    cache
        .lock()
        .unwrap()
        .entry(key.to_owned())
        .or_insert(value)
        .clone()
}

/// B. Move `amount` from `from` to `to` once `approve` resolves to true, and
/// return whether money moved. A rejected transfer, or one whose future is
/// dropped before approval, changes nothing. Inputs guarantee `*from >= amount`
/// and no overflow.
async fn transfer(
    from: &mut u64,
    to: &mut u64,
    amount: u64,
    approve: impl Future<Output = bool>,
) -> bool {
    // Nothing changes before the await. A rejection, or a drop while the
    // approval is pending, therefore leaves both balances as they were.
    if !approve.await {
        return false;
    }
    // No await between these two lines, so nothing can cancel between them.
    *from -= amount;
    *to += amount;
    true
}

async fn save(store: &RefCell<Vec<String>>, item: &str) {
    store.borrow_mut().push(item.to_owned());
}

/// C. Save every item, in order, and return how many were saved.
async fn save_all(store: &RefCell<Vec<String>>, items: &[&str]) -> usize {
    for item in items {
        // Without `.await` this line only builds a future and drops it.
        save(store, item).await;
    }
    items.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::{
        marker::PhantomData,
        pin::{Pin, pin},
        task::{Context, Poll, Waker},
    };

    fn ready<F: Future>(future: F) -> F::Output {
        let mut f = pin!(future);
        match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(x) => x,
            Poll::Pending => panic!("expected immediately ready future"),
        }
    }

    /// Returns Pending once, then Ready.
    struct YieldOnce(bool);
    impl Future for YieldOnce {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                return Poll::Ready(());
            }
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    fn cache_of(entries: &[(&str, &str)]) -> Mutex<BTreeMap<String, String>> {
        Mutex::new(
            entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn a_hit_returns_the_cached_value() {
        let cache = cache_of(&[("k", "cached")]);
        let value = ready(get_or_load(&cache, "k", async { "loaded".to_string() }));
        assert_eq!(value, "cached");
    }

    // The miss tests run inside `within`: a fix that locks the cache a second
    // time while the first guard is still alive blocks forever, and `within`
    // turns that into a failure with a message.

    #[test]
    fn a_miss_loads_and_caches() {
        let (value, cached) = within(10, "get_or_load on a cache miss", || {
            let cache = cache_of(&[]);
            let value = ready(get_or_load(&cache, "k", async { "loaded".to_string() }));
            let cached = cache.lock().unwrap().get("k").cloned();
            (value, cached)
        });
        assert_eq!(value, "loaded");
        assert_eq!(
            cached.as_deref(),
            Some("loaded"),
            "a loaded value must be stored in the cache"
        );
    }

    #[test]
    fn a_cache_is_unlocked_while_loading() {
        within(10, "get_or_load with a slow load", || {
            let cache = cache_of(&[("other", "x")]);
            let mut f = pin!(get_or_load(&cache, "k", async {
                YieldOnce(false).await;
                "slow".to_string()
            }));
            let mut cx = Context::from_waker(Waker::noop());
            assert!(f.as_mut().poll(&mut cx).is_pending());
            assert!(
                cache.try_lock().is_ok(),
                "the lock was held across `.await`: every other task would block"
            );
            assert_eq!(f.as_mut().poll(&mut cx), Poll::Ready("slow".to_string()));
            assert_eq!(
                cache.lock().unwrap().len(),
                2,
                "the cache must hold the old entry and the loaded one"
            );
        });
    }

    /// Reports at run time whether `T` is `Send`. (A `T: Send` bound in a test
    /// would stop this whole file from compiling until the code is fixed.)
    /// `is_send` resolves to the inherent method when `T: Send`, and to the
    /// trait's default method otherwise.
    struct SendProbe<T>(PhantomData<T>);
    trait NotSend {
        fn is_send(&self) -> bool {
            false
        }
    }
    impl<T> NotSend for SendProbe<T> {}
    impl<T: Send> SendProbe<T> {
        fn is_send(&self) -> bool {
            true
        }
    }
    fn probe<T>(_: &T) -> SendProbe<T> {
        SendProbe(PhantomData)
    }

    #[test]
    fn a_future_can_move_between_threads() {
        let cache = cache_of(&[]);
        let f = get_or_load(&cache, "k", async { "loaded".to_string() });
        assert!(
            probe(&f).is_send(),
            "the get_or_load future is not Send, so a multi-threaded runtime could not \
             spawn it. It stores a MutexGuard across `.await`. Calling drop(guard) is \
             not enough for the compiler: end the guard's scope (a block, or a single \
             statement) before the await"
        );
    }

    #[test]
    fn b_approved_transfer_moves_money() {
        let (mut a, mut b) = (100, 5);
        assert!(ready(transfer(&mut a, &mut b, 30, async { true })));
        assert_eq!((a, b), (70, 35), "an approved transfer moves the amount");
    }

    #[test]
    fn b_rejected_transfer_changes_nothing() {
        let (mut a, mut b) = (100, 5);
        assert!(!ready(transfer(&mut a, &mut b, 30, async { false })));
        assert_eq!((a, b), (100, 5), "a rejected transfer lost money");
    }

    #[test]
    fn b_cancelled_transfer_changes_nothing() {
        let (mut a, mut b) = (100, 5);
        {
            let mut f = Box::pin(transfer(&mut a, &mut b, 30, std::future::pending()));
            assert!(
                f.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
        }
        assert_eq!((a, b), (100, 5), "dropping a pending transfer lost money");
    }

    #[test]
    fn c_every_item_is_saved_in_order() {
        let store = RefCell::new(Vec::new());
        assert_eq!(ready(save_all(&store, &["a", "b", "c"])), 3);
        assert_eq!(
            *store.borrow(),
            ["a", "b", "c"],
            "save_all reported success, but the store does not hold the items"
        );
        assert_eq!(ready(save_all(&store, &[])), 0);
    }
}
