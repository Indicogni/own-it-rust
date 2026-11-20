//! LESSON 08 — Patterns, items, and token munchers
//! Run: cargo test --test chapter22 l08_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: A repetition has no counter, so how can a macro give the third name in a list the value 2? Can a macro look inside an item that it captured with `$i:item`?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN: Explain what your muncher carries from one step to the next, why internal rules stand before the entry rule, and when you capture an item whole instead of matching its parts.
//! COMPILER LAB (examples/ch22_scratch.rs): In a copy of pick!, change `pat` to `pat_param` and pass an arm `1 | 2 => "small"`. Write a rule `($e:expr $name:ident) => {}` and read what may follow an `expr`. Move the internal rule of your attr_group! below its entry rule and read the error.

/// A. (`pat`) Evaluate `$value` once and match it against the arms in order.
/// The first arm whose pattern matches, and whose `if` guard (when there is
/// one) is true, gives `Some(result)`. If no arm matches, the answer is
/// `None`. Only the chosen arm's result expression runs. A name bound by a
/// pattern is usable in that arm's guard and result. Match `$value` itself:
/// no clone and no allocation.
macro_rules! pick {
    ($value:expr; $($pattern:pat $(if $guard:expr)? => $result:expr),+ $(,)?) => {
        // One `match` evaluates `$value` once and tries the arms in order.
        match $value {
            $( $pattern $(if $guard)? => ::core::option::Option::Some($result), )+
            // Not reachable when the caller's last pattern matches everything.
            #[allow(unreachable_patterns)]
            _ => ::core::option::Option::None,
        }
    };
}

/// B. (`item`, internal rule) Put every attribute of the bracketed list on
/// EVERY item that follows it. The list may be empty and there may be no
/// items. Each item keeps its own attributes, doc comments and visibility.
///
///     attr_group! { [#[inline] #[track_caller]] fn first() {} pub fn second() {} }
///
/// The STARTER rule only lets the file compile: it copies the items and drops
/// the list. Replace it. You may add rules and change this matcher, but keep
/// the invocation syntax shown above.
macro_rules! attr_group {
    // Internal rule, placed first: the entry rule below would try to parse
    // `one` as an item, and a fragment that fails to parse is a hard error.
    (@one [$(#[$attr:meta])*] $item:item) => {
        $(#[$attr])*
        $item
    };
    // The whole list is captured as ONE token tree. A capture without a
    // repetition may be used inside `$( ... )*`, once per item; a repeated
    // `$attr` could not be combined with the repeated `$item`.
    ($attrs:tt $($item:item)*) => {
        $( attr_group!(@one $attrs $item); )*
    };
}

/// C. (`vis`, tt-muncher with an accumulator) Generate a module of numbered
/// constants of the integer type `$ty`. The first name is 0. Every later name
/// is the constant before it plus one. A name written as `NAME = value` gets
/// that value instead, and the names after it count on from there.
///
///     numbered! { pub mod column: usize { ID, NAME, EMAIL = 5, PHONE } }
///
/// gives `column::ID == 0`, `NAME == 1`, `EMAIL == 5` and `PHONE == 6`, and
/// `column::ALL`, a slice of (name, value) pairs in source order. The module
/// keeps the visibility and the attributes that the caller wrote. A trailing
/// comma and an empty list are accepted.
///
/// The STARTER rule gives every constant the value 0, because a repetition
/// cannot count. Keep what it does for the module and for `ALL`. Number the
/// constants with internal rules that take the list apart one name at a time
/// and carry the next value along.
macro_rules! numbered {
    // Step rules. State: the type, and the value for the next plain name.
    // No names are left.
    (@step $ty:ty, $next:expr;) => {};
    // A name with its own value. This rule stands before the plain one, which
    // would also accept the name and then stop at the `=`.
    (@step $ty:ty, $next:expr; $name:ident = $value:expr, $($rest:tt)*) => {
        pub const $name: $ty = $value;
        numbered!(@step $ty, $name + 1; $($rest)*);
    };
    // A plain name takes the carried value.
    (@step $ty:ty, $next:expr; $name:ident, $($rest:tt)*) => {
        pub const $name: $ty = $next;
        numbered!(@step $ty, $name + 1; $($rest)*);
    };
    // Entry rule. It writes a comma after every name, so the step rules need
    // no special case for the last one.
    (
        $(#[$attr:meta])*
        $vis:vis mod $module:ident : $ty:ty {
            $($name:ident $(= $value:expr)?),* $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis mod $module {
            numbered!(@step $ty, 0; $($name $(= $value)?,)*);
            pub const ALL: &[(&str, $ty)] = &[$((stringify!($name), $name)),*];
        }
    };
}

#[cfg(test)]
mod tests {
    use practice::alloc::count;
    use std::cell::RefCell;
    use std::mem::size_of;
    use std::panic::Location;

    #[derive(Debug, PartialEq)]
    enum Shape {
        Circle(u32),
        Square(u32),
        Rect { w: u32, h: u32 },
    }

    fn describe(shape: Shape) -> Option<u32> {
        pick!(shape;
            Shape::Circle(r) => r,
            Shape::Rect { w, h } if w == h => w,
            Shape::Rect { w, .. } => w + 100,
        )
    }

    #[test]
    fn a_first_matching_arm_wins() {
        assert_eq!(describe(Shape::Circle(3)), Some(3));
        assert_eq!(
            describe(Shape::Rect { w: 4, h: 4 }),
            Some(4),
            "the guard `w == h` is true, so the second arm is chosen"
        );
        assert_eq!(
            describe(Shape::Rect { w: 4, h: 9 }),
            Some(104),
            "the guard is false, so matching continues with the third arm"
        );
        assert_eq!(describe(Shape::Square(2)), None, "no arm matches a Square");
    }

    #[test]
    fn a_or_patterns_ranges_and_wildcards() {
        let doubled: Option<u32> =
            pick!(Shape::Square(7); Shape::Circle(n) | Shape::Square(n) => n * 2);
        assert_eq!(doubled, Some(14), "`A(n) | B(n)` is one pattern");
        let sign: Option<&str> =
            pick!(-3_i32; i32::MIN..0 => "negative", 0 => "zero", _ => "positive");
        assert_eq!(sign, Some("negative"));
        let pair: Option<(char, char)> = pick!("héllo".chars().nth(1); Some(c) => (c, 'x'));
        assert_eq!(pair, Some(('é', 'x')));
    }

    #[test]
    fn a_value_runs_once_and_only_the_chosen_result_runs() {
        let log = RefCell::new(Vec::new());
        let note = |s: &'static str| {
            log.borrow_mut().push(s);
            s
        };
        let got: Option<&str> = pick!(note("value").len();
            0 => note("zero"),
            n if { note("guard"); n > 100 } => note("huge"),
            5 => note("five"),
            _ => note("other"),
        );
        assert_eq!(got, Some("five"));
        assert_eq!(
            *log.borrow(),
            ["value", "guard", "five"],
            "evaluate `$value` once, then run only the guard that is tried and the result that is chosen"
        );
    }

    #[test]
    fn a_bound_values_move_out_without_a_copy() {
        let text = String::from("moved, not cloned");
        let buffer = text.as_ptr();
        let (got, allocations): (Option<String>, usize) =
            count(|| pick!(Some(text); Some(s) if !s.is_empty() => s));
        assert_eq!(
            allocations, 0,
            "pick! must not allocate: match the value itself, not a clone of it"
        );
        assert_eq!(
            got.map(|s| s.as_ptr()),
            Some(buffer),
            "the String bound by the pattern must be the caller's buffer, not a copy"
        );
    }

    attr_group! {
        [#[track_caller] #[inline]]
        /// With #[track_caller], `Location::caller()` is the place that called
        /// this function. Without it, it is the line inside this function.
        fn first_caller_line() -> u32 {
            Location::caller().line()
        }
        pub(crate) fn second_caller_line() -> u32 {
            Location::caller().line()
        }
    }
    attr_group! {
        [#[repr(u32)] #[allow(dead_code)]]
        enum Signal {
            Stop,
            Go,
        }
        /// Keeps its own derive next to the attributes from the list.
        #[derive(Debug, PartialEq)]
        pub enum Level {
            Low,
            High,
        }
    }
    attr_group!([]);
    attr_group! {
        []
        fn untouched() -> u8 {
            9
        }
    }

    #[test]
    fn b_every_function_gets_the_attributes() {
        let (first, second, here) = (first_caller_line(), second_caller_line(), line!());
        assert_eq!(
            first, here,
            "the first function must report its caller's line: it needs #[track_caller] from the list (the starter rule drops the list)"
        );
        assert_eq!(
            second, here,
            "the SECOND function needs the attributes too: the list goes on every item, not only on the first"
        );
    }

    #[test]
    fn b_every_type_gets_the_attributes_and_keeps_its_own() {
        assert_eq!(
            size_of::<Signal>(),
            4,
            "#[repr(u32)] stores the discriminant in a u32, so the enum is 4 bytes (the starter rule drops the list)"
        );
        assert_eq!(
            size_of::<Level>(),
            4,
            "the second enum needs #[repr(u32)] from the list as well"
        );
        assert_eq!(
            Level::Low,
            Level::Low,
            "an item's own #[derive] stays in place"
        );
        assert_ne!(Level::Low, Level::High);
    }

    #[test]
    fn b_an_empty_list_changes_nothing() {
        assert_eq!(untouched(), 9);
    }

    mod generated {
        numbered! {
            /// Column positions in one exported row.
            pub mod column: usize {
                ID, NAME, EMAIL = 5, PHONE
            }
        }
        numbered! {
            mod level: u8 { LOW = 1 << 2, MID, HIGH, }
        }
        numbered! {
            pub mod mixed: i64 { A = 10, B, C = -3, D, E = D * 2, F }
        }
        numbered! {
            pub(super) mod empty: u16 {}
        }
        numbered! {
            pub mod letters: u32 {
                A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
            }
        }
        /// `level` is private to this module, so the tests read it from here.
        pub fn levels() -> [u8; 3] {
            [level::LOW, level::MID, level::HIGH]
        }
    }

    #[test]
    fn c_names_count_up_from_zero() {
        use generated::column;
        assert_eq!(
            (column::ID, column::NAME),
            (0, 1),
            "the first name is 0 and the next is 1 (the starter rule gives every name 0)"
        );
        assert_eq!(
            generated::letters::Z,
            25,
            "the 26th name is 25: each step adds one"
        );
        assert_eq!(generated::letters::ALL.len(), 26);
    }

    #[test]
    fn c_an_explicit_value_restarts_the_count() {
        use generated::{column, mixed};
        assert_eq!(column::EMAIL, 5, "`EMAIL = 5` sets the value");
        assert_eq!(
            column::PHONE,
            6,
            "the name after `EMAIL = 5` is 6, not its position in the list"
        );
        assert_eq!(
            (mixed::A, mixed::B, mixed::C, mixed::D, mixed::E, mixed::F),
            (10, 11, -3, -2, -4, -3),
            "every name without a value is the constant before it plus one"
        );
        assert_eq!(
            generated::levels(),
            [4, 5, 6],
            "`LOW = 1 << 2` is 4, so MID is 4 + 1 = 5 (not 1 << 3)"
        );
    }

    #[test]
    fn c_all_lists_names_and_values_in_source_order() {
        assert_eq!(
            generated::column::ALL,
            [("ID", 0), ("NAME", 1), ("EMAIL", 5), ("PHONE", 6)],
            "ALL pairs every name with its numbered value"
        );
        assert!(
            generated::empty::ALL.is_empty(),
            "an empty list gives an empty module"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `$v:vis` is the one fragment that may match nothing at all.
        // What text does it hold for `fn`, `pub fn` and `pub(crate) fn`?
        macro_rules! vis_text {
            ($v:vis fn) => {
                stringify!($v)
            };
        }
        let p1: (&str, &str, &str) = ("", "pub", "pub(crate)");
        assert_eq!(
            p1,
            (vis_text!(fn), vis_text!(pub fn), vis_text!(pub(crate) fn)),
            "P1: what a vis fragment matches"
        );

        // P2. The `pat` rule comes first. Which rule matches `Some(_)`, and
        // which matches `1 + 2`? (`1` alone is a valid pattern.)
        macro_rules! shape {
            ($p:pat) => {
                "pat"
            };
            ($e:expr) => {
                "expr"
            };
        }
        let p2: (&str, &str) = ("pat", "expr");
        assert_eq!(
            p2,
            (shape!(Some(_)), shape!(1 + 2)),
            "P2: a rule must match ALL the tokens"
        );

        // P3. A muncher with an accumulator: it starts at 1; `+` adds one and
        // `*` doubles. The accumulator is matched as `$n:expr` at every step.
        macro_rules! tally {
            (@acc $n:expr;) => { $n };
            (@acc $n:expr; + $($rest:tt)*) => { tally!(@acc $n + 1; $($rest)*) };
            (@acc $n:expr; * $($rest:tt)*) => { tally!(@acc $n * 2; $($rest)*) };
            ($($tokens:tt)*) => { tally!(@acc 1; $($tokens)*) };
        }
        let p3: i32 = 5;
        assert_eq!(
            p3,
            tally!(+ * +),
            "P3: an expr accumulator keeps its grouping"
        );

        // P4. The macro writes a function AND a local variable. The caller has
        // a function and a local with the same names. Which does the caller see?
        macro_rules! define_both {
            () => {
                fn answer() -> i32 {
                    2
                }
                let local = 2;
                let _ = local;
            };
        }
        fn answer() -> i32 {
            1
        }
        let local = 1;
        let seen = {
            define_both!();
            (answer(), local)
        };
        let p4: (i32, i32) = (2, 1);
        assert_eq!(p4, seen, "P4: items are not hygienic, locals are");
    }
}
