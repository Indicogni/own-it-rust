//! DRILL 09-03 · Newtypes are distinct types
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does the same representation make two tuple structs interchangeable?

struct UserId(u64);
struct OrderId(u64);

fn cancel(order: OrderId) -> u64 {
    order.0
}

fn main() {
    let user = UserId(7);
    let order = OrderId(42);
    // FIX: `cancel` takes an OrderId. A UserId is a different type even
    // though both wrap a u64, so pass the order.
    let cancelled = cancel(order);
    assert_eq!(cancelled, 42); // KEEP
}
