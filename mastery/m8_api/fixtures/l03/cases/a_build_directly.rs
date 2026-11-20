//@ expect: E0603 or E0423
//@ why: (A) if users can use `SensorId` as a constructor, as in `SensorId(0)`, the "never zero" rule is not a rule, and the field's type can never change

fn main() {
    // The name of a tuple struct is also a function that builds one, unless a
    // field is private (E0603). A struct with a named field has no such
    // function at all (E0423).
    let build = gauge::SensorId;
    let _ = build;
}
