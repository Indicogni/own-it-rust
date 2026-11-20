//! LESSON 01 — Variants and payloads
//! Run: cargo test --test chapter10 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Can an enum whose variants carry data be cast to an integer with `as`,
//!    like a fieldless enum?
//! 2. Can two variants of one enum carry different types?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain the discriminant, payload, exhaustive match, and
//! why default enum layout is not a stable serialization format.
//!
//! COMPILER / TRANSFER LAB (optional, not checked by tests): Inspect size_of
//! and align_of for a fieldless enum, an enum with String and integer
//! payloads, and Option<Box<i32>>. Separate what you observe from what the
//! language guarantees; the null-pointer optimization is documented only for
//! a short list of types. Then add a variant to Reply and let the compiler
//! list the matches that need an update.
//! Use examples/ch10_scratch.rs, and restore it to compiling code afterwards.

#[derive(Debug, PartialEq)]
enum Reply {
    Pending,
    Count(u32),
    Text(String),
}
/// A. Classify every variant as "pending", "count", or "text".
fn kind(reply: &Reply) -> &'static str {
    match reply {
        Reply::Pending => "pending",
        Reply::Count(_) => "count",
        Reply::Text(_) => "text",
    }
}
/// B. Consume a reply and render Pending => "waiting", Count(n) => "count:N",
/// Text(s) => s unchanged (move its owner, do not clone).
fn render(reply: Reply) -> String {
    match reply {
        Reply::Pending => String::from("waiting"),
        Reply::Count(n) => format!("count:{n}"),
        // `text` is the String that was inside the variant: moved, not copied.
        Reply::Text(text) => text,
    }
}

/// C. Design task: this time YOU write the type. A parcel locker slot is in
/// exactly one of three states:
///
/// - free;
/// - reserved for a customer (a name) for a number of minutes (a u32);
/// - holding a parcel that has a tracking code (a String).
///
/// Replace the placeholder variant with your own variants. Give each variant
/// only the data its state needs, so that nonsense such as "free, but with a
/// tracking code" cannot even be written. `Slot` must stay an enum, but the
/// tests call only the five functions below, so variant names and payload
/// shapes are your decision.
enum Slot {
    Free,
    // Named fields, because two payloads of different meaning sit side by
    // side and `Reserved(String, u32)` would not say which is which.
    Reserved { customer: String, minutes: u32 },
    Holding(String),
}
/// C1. A free slot.
fn free_slot() -> Slot {
    Slot::Free
}
/// C2. A slot reserved for `customer` for `minutes` minutes.
fn reserved_slot(customer: &str, minutes: u32) -> Slot {
    Slot::Reserved {
        customer: customer.to_owned(),
        minutes,
    }
}
/// C3. A slot holding the parcel with this tracking code. Store the String
/// you are given; do not copy its text.
fn holding_slot(code: String) -> Slot {
    Slot::Holding(code)
}
/// C4. "free", "reserved for NAME (N min)", or "holding CODE".
fn describe(slot: &Slot) -> String {
    match slot {
        Slot::Free => String::from("free"),
        Slot::Reserved { customer, minutes } => {
            format!("reserved for {customer} ({minutes} min)")
        }
        Slot::Holding(code) => format!("holding {code}"),
    }
}
/// C5. Empty the slot. A holding slot gives Some(its tracking code), moved
/// out and not cloned. Every other state gives None.
fn take_parcel(slot: Slot) -> Option<String> {
    match slot {
        Slot::Holding(code) => Some(code),
        // The other states are listed, not hidden behind `_`, so a fourth
        // state added later has to be decided here too.
        Slot::Free | Slot::Reserved { .. } => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::mem::discriminant;
    #[test]
    fn a_each_state_has_one_variant() {
        assert_eq!(kind(&Reply::Pending), "pending");
        assert_eq!(kind(&Reply::Count(0)), "count");
        assert_eq!(kind(&Reply::Text(String::new())), "text");
    }
    #[test]
    fn b_payload_ownership_moves_out() {
        assert_eq!(render(Reply::Pending), "waiting");
        assert_eq!(render(Reply::Count(7)), "count:7");
        let s = "港".to_owned();
        let p = s.as_ptr();
        let out = render(Reply::Text(s));
        assert_eq!(out, "港");
        assert_eq!(
            out.as_ptr(),
            p,
            "render must return the same String buffer it was given, not a copy"
        );
        assert_eq!(render(Reply::Count(u32::MAX)), "count:4294967295");
        assert_eq!(render(Reply::Text(String::new())), "");
    }
    #[test]
    fn b_text_payload_moves_without_allocating() {
        let reply = Reply::Text("ready".into());
        let (out, allocations) = count(|| render(reply));
        assert_eq!(out, "ready");
        assert_eq!(
            allocations, 0,
            "the Text payload should move out; an allocation here means it was copied"
        );
    }
    #[test]
    fn c_each_state_describes_itself() {
        assert_eq!(describe(&free_slot()), "free");
        assert_eq!(
            describe(&reserved_slot("Ana", 30)),
            "reserved for Ana (30 min)"
        );
        assert_eq!(
            describe(&reserved_slot("李", u32::MAX)),
            "reserved for 李 (4294967295 min)"
        );
        assert_eq!(describe(&holding_slot("ZX-81".into())), "holding ZX-81");
    }
    #[test]
    fn c_only_a_holding_slot_gives_a_parcel() {
        assert_eq!(take_parcel(free_slot()), None);
        assert_eq!(
            take_parcel(reserved_slot("Ana", 30)),
            None,
            "a reservation holds no parcel yet"
        );
        let code = String::from("ZX-81");
        let p = code.as_ptr();
        let slot = holding_slot(code);
        let (out, allocations) = count(|| take_parcel(slot));
        let out = out.expect("a holding slot gives back its tracking code");
        assert_eq!(out, "ZX-81");
        assert_eq!(
            out.as_ptr(),
            p,
            "the tracking code must be the same buffer that went in, not a copy"
        );
        assert_eq!(
            allocations, 0,
            "taking the parcel moves the String out; it must not allocate"
        );
    }
    #[test]
    fn c_three_states_are_three_variants() {
        let free = free_slot();
        let reserved = reserved_slot("Ana", 30);
        let holding = holding_slot("ZX-81".into());
        // `discriminant` tells which variant a value is. If these lines do
        // not compile, `Slot` is no longer an enum.
        assert!(
            discriminant(&free) != discriminant(&reserved)
                && discriminant(&reserved) != discriminant(&holding)
                && discriminant(&free) != discriminant(&holding),
            "each state must be its own variant of Slot; a struct with a state \
             field and optional data lets impossible combinations exist"
        );
        assert!(
            discriminant(&reserved) == discriminant(&reserved_slot("Bo", 5)),
            "two reservations are the same variant; only their payloads differ"
        );
        assert!(
            size_of::<Slot>() <= 5 * size_of::<usize>(),
            "Slot takes {} bytes. An enum is about as big as its largest \
             variant, so check for payload fields that a state does not need",
            size_of::<Slot>()
        );
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Size {
        Small,
        Large(u8),
    }
    enum Level {
        Low = 1,
        Mid,
        High = 10,
        Top,
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Discriminants continue from the previous one.
        let p1: (i32, i32) = (2, 11);
        assert_eq!(
            p1,
            (Level::Mid as i32, Level::Top as i32),
            "P1: implicit discriminants"
        );

        // P2. How many machine words (usize) does Option<Box<i32>> take?
        let p2: usize = 1;
        assert_eq!(
            p2 * size_of::<usize>(),
            size_of::<Option<Box<i32>>>(),
            "P2: the null-pointer optimization"
        );

        // P3. Derived ordering on an enum: write the sorted vector.
        let mut sizes = vec![Size::Large(1), Size::Small, Size::Large(0)];
        sizes.sort();
        let p3: Vec<Size> = vec![Size::Small, Size::Large(0), Size::Large(1)];
        assert_eq!(p3, sizes, "P3: variants order by declaration");

        // P4. Do these two values have the same discriminant?
        let p4: bool = true;
        assert_eq!(
            p4,
            discriminant(&Reply::Count(1)) == discriminant(&Reply::Count(2)),
            "P4: discriminant ignores the payload"
        );
    }
}
