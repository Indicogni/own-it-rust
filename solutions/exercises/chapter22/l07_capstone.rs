//! LESSON 07 — Capstone: structured events
//! Run: cargo test --test chapter22 l07_
//!
//! PREDICT before running: Can a macro check that every field name is a Rust identifier? What should duplicate field names mean in this API?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN: Specify the event grammar, evaluation order, exported-helper paths, and diagnostics for unsupported syntax.
//! COMPILER LAB (examples/ch22_scratch.rs): Try an expression instead of an identifier field name, an omitted colon, and a value with a side effect. Improve error messages with a fallback compile_error! rule.

use std::collections::BTreeMap;
#[derive(Debug, PartialEq)]
struct Event {
    kind: String,
    fields: BTreeMap<String, String>,
}
/// A. Construct an Event from owned fields. Kind must be preserved verbatim.
/// Move both inputs in: an owned String kind and the map are not copied.
fn make_event(kind: impl Into<String>, fields: BTreeMap<String, String>) -> Event {
    Event {
        kind: kind.into(),
        fields,
    }
}
/// B. Accept kind; field_ident: expression pairs, zero or more with optional
/// trailing comma. Evaluate kind first, each field value once in source order.
/// Stringify field names, Display-format values, and keep the last duplicate.
macro_rules! event {
    ($kind:expr; $($field:ident : $value:expr),* $(,)?) => {{
        // Bind the kind first, so that it is evaluated before any field.
        let kind = $kind;
        // `mut` is needed only when there is at least one field.
        #[allow(unused_mut)]
        let mut fields = BTreeMap::new();
        $( fields.insert(String::from(stringify!($field)), $value.to_string()); )*
        make_event(kind, fields)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    #[test]
    fn a_make_event_keeps_kind_verbatim() {
        let e = make_event(" Job:🦀 ", BTreeMap::new());
        assert_eq!(e.kind, " Job:🦀 ");
        let fields = BTreeMap::from([("k".to_string(), "v".to_string())]);
        assert_eq!(make_event(String::new(), fields.clone()).fields, fields);
    }
    #[test]
    fn a_make_event_moves_its_inputs() {
        let kind = String::from("job");
        let p = kind.as_ptr();
        let fields = BTreeMap::from([("id".to_string(), "7".to_string())]);
        let (event, allocations) = count(|| make_event(kind, fields));
        assert_eq!(allocations, 0, "an owned kind and map should move in");
        assert_eq!(
            event.kind.as_ptr(),
            p,
            "the event must hold the caller's String buffer, not a copy of it"
        );
    }
    #[test]
    fn b_event_grammar_builds_a_typed_value() {
        let value: Event = event!("job"; id:7,ok:true,);
        assert_eq!(
            value,
            Event {
                kind: "job".into(),
                fields: BTreeMap::from([("id".into(), "7".into()), ("ok".into(), "true".into())])
            }
        );
        let empty: Event = event!("empty";);
        assert!(empty.fields.is_empty());
    }
    #[test]
    fn b_event_evaluation_order_and_duplicates() {
        let mut n = 0;
        let e: Event = event!({n+=1;"ordered"}; x:{n+=1;n},x:{n+=1;n});
        assert_eq!(n, 3, "kind and both values must each run exactly once");
        assert_eq!(e.fields["x"], "3", "the last duplicate field wins");
    }
    #[test]
    fn b_kind_is_evaluated_before_any_field() {
        let log = RefCell::new(Vec::new());
        let note = |s: &'static str| {
            log.borrow_mut().push(s);
            s
        };
        let e: Event = event!(note("kind"); b: note("b"), a: note("a"));
        assert_eq!(
            *log.borrow(),
            ["kind", "b", "a"],
            "evaluate kind first, then the values in source order (not in key order)"
        );
        assert_eq!(e.kind, "kind");
        assert_eq!(e.fields.keys().collect::<Vec<_>>(), ["a", "b"]);
    }
    #[test]
    fn b_values_use_display_not_debug() {
        let city = "São Paulo";
        let e: Event = event!(String::from("move"); city: city, ratio: 0.5, unit: 'm');
        assert_eq!(e.kind, "move");
        assert_eq!(e.fields["city"], "São Paulo");
        assert_eq!(e.fields["ratio"], "0.5");
        assert_eq!(e.fields["unit"], "m");
    }
}
