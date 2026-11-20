//! LESSON 04 — Data layout: size, alignment, niches, and arrays of structs
//! Read first: CONCEPTS.md, "Lesson 04". Start after: lesson 03; chapters 9
//! (structs), 10 (enums) and 3 (sizes of the basic types).
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l04_
//! Bench: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l04_layout
//! Lab: LABS.md, "Lab 4" compares the two layouts of task D in time.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a struct holds a `u8`, a `u64`
//! and another `u8`. What is the smallest size it can have, and why is it not
//! 10 bytes?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! In A and C you change a supplied type. In B and D you choose the fields of
//! a type yourself. The tests look at `size_of`, at slices, and at allocation
//! counts; they never look at time.
//! EXPLAIN afterward: Why does the order of fields change the size of a
//! `repr(C)` struct, and why does it not change the size of an ordinary Rust
//! struct? What is a niche, and which of your types have one? When is a struct
//! of arrays better than an array of structs, and when is it worse?
//! JUDGMENT: Layout work pays off for types that exist a million times or that
//! a hot loop walks over; it is wasted on a configuration struct. Measure with
//! `size_of` and a benchmark before and after. Boxing a large, rare variant is
//! almost always right. A struct of arrays helps loops that read a few fields
//! of many records, and hurts code that reads all fields of one record. Keep
//! `repr(C)` for types that cross a language or file-format boundary; inside
//! Rust, the default representation already orders the fields for you.

use std::num::NonZeroU32;

/// A. One record of a binary file format. The format is shared with a C
/// program, so the struct is `repr(C)`: the fields lie in memory in the order
/// in which they are declared.
///
/// As declared here, the struct takes 32 bytes. The next version of the
/// format may put the fields in any order. Reorder the fields so that the
/// struct takes 16 bytes. Keep `repr(C)`, the field names and the field types.
/// (Without `repr(C)` the compiler would reorder the fields for you, and the
/// C program could no longer read the record. The tests therefore pass a
/// `Sample` to a function with the C calling convention, which the compiler
/// accepts only for a type with a defined layout.)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub flag: u8,
    pub timestamp: u64,
    pub kind: u8,
    pub value: u32,
    pub channel: u16,
}

/// B. The number of a slot in a table with fewer than `u32::MAX` slots.
///
/// Many structures store an `Option<SlotId>` per element. Choose the field so
/// that `Option<SlotId>` is as small as a `u32`: the value `u32::MAX` is never
/// a valid slot number, so one bit pattern is free to mean `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(u32); // Replace the field with your representation.

impl SlotId {
    /// The id for slot `index`, or `None` for `u32::MAX`.
    pub fn new(index: u32) -> Option<SlotId> {
        todo!("B")
    }

    /// The slot number that this id was made from.
    pub fn index(self) -> u32 {
        todo!("B")
    }
}

/// C. A message in a queue. Almost all messages are `Ping` or `Data`; a
/// `Trace` is rare. As declared here, every message takes 136 bytes, because
/// an enum is as large as its largest variant.
///
/// Change the type so that a message takes at most 16 bytes. `Ping` and `Data`
/// must stay free of heap allocations. The tests build a `Trace` only through
/// [`Message::trace`] and read it only through [`Message::trace_words`], so
/// you may change what the variant holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Ping,
    Data { id: u32, len: u32 },
    Trace([u64; 16]),
}

impl Message {
    /// A trace message that holds `words`.
    pub fn trace(words: [u64; 16]) -> Message {
        Message::Trace(words)
    }

    /// The words of a trace message; `None` for the other variants.
    pub fn trace_words(&self) -> Option<&[u64; 16]> {
        match self {
            Message::Trace(words) => Some(words),
            _ => None,
        }
    }
}

/// One order as the rest of the program sees it. Supplied; do not edit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    pub price_cents: u32,
    pub quantity: u32,
    pub open: bool,
    pub note: [u8; 24],
}

/// The total value (`price_cents * quantity`) of the open orders, computed
/// over an array of structs. Supplied; `benches/l04_layout.rs` compares it
/// with your `OrderTable::open_value`.
pub fn open_value_of(orders: &[Order]) -> u64 {
    orders
        .iter()
        .filter(|order| order.open)
        .map(|order| u64::from(order.price_cents) * u64::from(order.quantity))
        .sum()
}

/// D. The same orders as a *struct of arrays*: one column per field instead
/// of one struct per order. A loop that needs the prices then walks over
/// memory that holds nothing but prices.
///
/// The fields are yours to choose. The accessors below return whole columns
/// as slices, which is what makes the layout visible to the tests.
#[derive(Debug, Clone, Default)]
pub struct OrderTable {
    // Your fields.
}

impl OrderTable {
    /// An empty table with room for `capacity` orders. After this call,
    /// pushing up to `capacity` orders must not allocate.
    pub fn with_capacity(capacity: usize) -> OrderTable {
        todo!("D")
    }

    /// Appends one order as the last row.
    pub fn push(&mut self, order: Order) {
        todo!("D")
    }

    pub fn len(&self) -> usize {
        todo!("D")
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Row `index` put back together as an `Order`; `None` past the end.
    pub fn get(&self, index: usize) -> Option<Order> {
        todo!("D")
    }

    /// The `price_cents` of every row, in row order, as one slice.
    pub fn prices(&self) -> &[u32] {
        todo!("D")
    }

    /// The `quantity` of every row, in row order, as one slice.
    pub fn quantities(&self) -> &[u32] {
        todo!("D")
    }

    /// The `open` flag of every row, in row order, as one slice.
    pub fn open_flags(&self) -> &[bool] {
        todo!("D")
    }

    /// The total value (`price_cents * quantity`) of the open orders. Read
    /// only the three columns that the sum needs.
    pub fn open_value(&self) -> u64 {
        todo!("D")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::{count, measure};
    use std::hint::black_box;
    use std::mem::{align_of, offset_of, size_of, size_of_val};

    /// Compiles only when the field has exactly the type `T`.
    fn has_type<T>(_field: &T) {}

    /// Stands for the C program that reads a `Sample`. If the build stops
    /// here with "`extern` fn uses type `Sample`, which is not FFI-safe",
    /// then `Sample` has lost its `#[repr(C)]`: put it back and reorder the
    /// fields instead.
    #[deny(improper_ctypes_definitions)]
    extern "C" fn c_side_reads(sample: Sample) -> u64 {
        sample.timestamp + u64::from(sample.value)
    }

    fn order(i: u32) -> Order {
        let mut note = [0u8; 24];
        note[0] = i as u8;
        Order {
            id: u64::from(i) + 1_000,
            price_cents: i * 3 + 1,
            quantity: i % 7 + 1,
            open: !i.is_multiple_of(3),
            note,
        }
    }

    #[test]
    fn a_sample_takes_sixteen_bytes() {
        let sample = Sample {
            flag: 1,
            timestamp: 2,
            kind: 3,
            value: 4,
            channel: 5,
        };
        has_type::<u8>(&sample.flag);
        has_type::<u64>(&sample.timestamp);
        has_type::<u8>(&sample.kind);
        has_type::<u32>(&sample.value);
        has_type::<u16>(&sample.channel);
        assert_eq!(
            (
                sample.flag,
                sample.timestamp,
                sample.kind,
                sample.value,
                sample.channel
            ),
            (1, 2, 3, 4, 5)
        );
        assert_eq!(
            c_side_reads(sample),
            6,
            "the C side must see the same field values"
        );
        assert_eq!(align_of::<Sample>(), 8);
        assert_eq!(
            size_of::<Sample>(),
            16,
            "the five fields need 8 + 4 + 2 + 1 + 1 = 16 bytes; a larger size is padding that \
             the declaration order forces"
        );
        assert_eq!(
            offset_of!(Sample, timestamp) % 8,
            0,
            "a u64 always starts at a multiple of 8"
        );
    }

    #[test]
    fn b_slot_ids_round_trip() {
        for index in [0, 1, 2, 77, u32::MAX - 1] {
            let id = SlotId::new(index).expect("every index below u32::MAX is valid");
            assert_eq!(id.index(), index);
        }
        assert_eq!(SlotId::new(u32::MAX), None, "u32::MAX is not a slot number");
        assert_ne!(SlotId::new(0), SlotId::new(1));
    }

    #[test]
    fn b_an_optional_slot_id_is_as_small_as_a_u32() {
        assert_eq!(size_of::<SlotId>(), 4);
        assert_eq!(
            size_of::<Option<SlotId>>(),
            4,
            "Option<SlotId> must not need extra bytes for `None`; give SlotId a field type \
             with a bit pattern that it never uses"
        );
        assert_eq!(
            size_of::<[Option<SlotId>; 1000]>(),
            4_000,
            "a table of 1,000 optional ids takes 4,000 bytes"
        );
    }

    #[test]
    fn c_messages_keep_their_content() {
        let mut words = [0u64; 16];
        words[3] = 33;
        words[15] = 1_515;
        let trace = Message::trace(words);
        assert_eq!(trace.trace_words(), Some(&words));
        assert_eq!(trace.clone(), trace);
        assert_eq!(Message::Ping.trace_words(), None);
        assert_eq!(Message::Data { id: 1, len: 2 }.trace_words(), None);
        assert_ne!(
            Message::Data { id: 1, len: 2 },
            Message::Data { id: 1, len: 3 }
        );
    }

    #[test]
    fn c_a_message_takes_at_most_sixteen_bytes() {
        assert!(
            size_of::<Message>() <= 16,
            "a Message takes {} bytes; move the large, rare payload out of the enum",
            size_of::<Message>()
        );
        let (_, allocations) = count(|| {
            black_box(Message::Ping);
            black_box(Message::Data { id: 7, len: 512 });
        });
        assert_eq!(
            allocations, 0,
            "Ping and Data are the common messages; they must not allocate"
        );
        let (_, usage) = measure(|| {
            let mut queue: Vec<Message> = Vec::with_capacity(1_000);
            for id in 0..1_000 {
                queue.push(Message::Data { id, len: 64 });
            }
            black_box(queue)
        });
        assert!(
            usage.bytes <= 16_000,
            "a queue of 1,000 Data messages asked for {} bytes; the budget is 16,000",
            usage.bytes
        );
    }

    #[test]
    fn d_rows_round_trip_through_the_columns() {
        let mut table = OrderTable::with_capacity(4);
        assert!(table.is_empty());
        for i in 0..50 {
            table.push(order(i));
        }
        assert_eq!(table.len(), 50);
        for i in 0..50 {
            assert_eq!(table.get(i as usize), Some(order(i)));
        }
        assert_eq!(table.get(50), None);
        assert!(OrderTable::default().is_empty());
        assert_eq!(OrderTable::default().open_value(), 0);
    }

    #[test]
    fn d_columns_are_contiguous_slices() {
        let mut table = OrderTable::with_capacity(100);
        let orders: Vec<Order> = (0..100).map(order).collect();
        for &order in &orders {
            table.push(order);
        }
        let prices: Vec<u32> = orders.iter().map(|order| order.price_cents).collect();
        let quantities: Vec<u32> = orders.iter().map(|order| order.quantity).collect();
        let open: Vec<bool> = orders.iter().map(|order| order.open).collect();
        assert_eq!(table.prices(), prices);
        assert_eq!(table.quantities(), quantities);
        assert_eq!(table.open_flags(), open);
        assert_eq!(
            size_of_val(table.prices())
                + size_of_val(table.quantities())
                + size_of_val(table.open_flags()),
            100 * 9,
            "the three columns that `open_value` reads hold 9 bytes per order"
        );
        assert_eq!(table.open_value(), open_value_of(&orders));
    }

    #[test]
    fn d_columns_are_stored_not_rebuilt() {
        let mut table = OrderTable::with_capacity(1_000);
        let (_, allocations) = count(|| {
            for i in 0..1_000 {
                table.push(order(i));
            }
        });
        assert_eq!(
            allocations, 0,
            "pushing 1,000 orders into a table made with_capacity(1_000) allocated \
             {allocations} time(s); reserve every column in with_capacity"
        );
        let (first, allocations) = count(|| {
            let total = table.open_value();
            (table.prices().as_ptr(), table.prices().as_ptr(), total)
        });
        assert_eq!(
            allocations, 0,
            "prices() and open_value() must read the stored columns; they allocated \
             {allocations} time(s)"
        );
        assert_eq!(
            first.0, first.1,
            "two calls of prices() must return the same stored column"
        );
        let (_, usage) = measure(|| black_box(OrderTable::with_capacity(1_000)));
        assert!(
            usage.allocations <= 5,
            "with_capacity made {} allocations; one per column is enough",
            usage.allocations
        );
        assert!(
            usage.bytes <= 1_000 * 48,
            "a table for 1,000 orders asked for {} bytes; an array of 1,000 `Order` structs \
             takes 48,000, and the columns need fewer because they hold no padding",
            usage.bytes
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// All answers are for a 64-bit target.
    #[test]
    fn p_predictions() {
        // P1. A `repr(C)` struct keeps the declared field order.
        #[repr(C)]
        struct Header {
            tag: u8,
            length: u32,
            checksum: u8,
        }
        // (size, alignment, offset of `checksum`)
        let p1: (usize, usize, usize) = guess();
        assert_eq!(
            p1,
            (
                size_of::<Header>(),
                align_of::<Header>(),
                offset_of!(Header, checksum)
            ),
            "P1: (size, alignment, offset of checksum) of Header (left: your prediction, right: actual)"
        );

        // P2. Which `Option`s are free?
        let p2: (usize, usize, usize, usize) = guess();
        assert_eq!(
            p2,
            (
                size_of::<Option<Box<u64>>>(),
                size_of::<Option<u64>>(),
                size_of::<Option<&u8>>(),
                size_of::<Option<NonZeroU32>>()
            ),
            "P2: sizes of Option<Box<u64>>, Option<u64>, Option<&u8>, Option<NonZeroU32> (left: your prediction, right: actual)"
        );

        // P3. An enum is as large as its largest variant plus, if needed, a tag.
        enum Shape {
            Dot,
            Circle(u8),
            Rect(u64),
        }
        let _ = (Shape::Dot, Shape::Circle(1), Shape::Rect(1));
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            (size_of::<Shape>(), align_of::<Shape>()),
            "P3: (size, alignment) of Shape (left: your prediction, right: actual)"
        );

        // P4. A loop adds up `price_cents` of 1,024 orders. A cache line is
        // 64 bytes. How many cache lines hold the data that the loop reads,
        // for an array of `Order` structs and for a column of `u32` prices?
        // (Assume that both start at the beginning of a cache line.)
        let rows = 1_024usize;
        let p4: (usize, usize) = guess();
        assert_eq!(
            p4,
            (
                (rows * size_of::<Order>()).div_ceil(64),
                (rows * size_of::<u32>()).div_ceil(64)
            ),
            "P4: cache lines read for (array of Order, column of u32) (left: your prediction, right: actual)"
        );
    }
}
