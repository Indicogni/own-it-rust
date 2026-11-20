//! LESSON 03 — Room to change: private fields, `non_exhaustive`, sealed traits, newtypes
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same package).
//!
//! This file is the whole source of a small library, `gauge`, shortly before its
//! release 1.0.0. The tests compile it a second time as a crate of its own and
//! compile the programs in `fixtures/l03/cases/` against it. Each case says in
//! its first lines what must happen and why. Read the cases: they are the
//! precise contract.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider: the declarations below work
//! today. For each one, which later change would be a major change, and which
//! one-line edit made today turns it into a minor one?
//!
//! IMPLEMENT tasks A to E: finish the bodies, and change the declarations so
//! that users cannot depend on what must stay free to change. The names and the
//! signatures of the functions stay as they are. Keep the supplied tests and add
//! one edge case.
//! EXPLAIN afterward: Every tool in this lesson takes something away from your
//! users today. Say, for each of the four tools, what the user loses and which
//! later change the library gains. When is the price too high?
//!
//! COMPILER / TRANSFER LAB (optional): when all tests pass, make three changes
//! that a version 1.1.0 might contain: a variant `Fault::Disconnected`, a field
//! `at_ms: u64` in `Reading` (set to 0 by `Reading::new`), and a third unit
//! `Kelvin`. Run the tests again. `fixtures/l03/cases/use_api.rs` plays the user
//! who wrote code against 1.0.0; it must still pass without an edit.

/// A. The number of a sensor. Zero is not a valid number.
///
/// Planned for later: the numbers may become 64 bits wide. So the
/// representation must not be visible: users get a `SensorId` from `new` and
/// read it with `get`, and cannot build or open one in another way.
///
/// One more requirement: an `Option<SensorId>` must be no larger than a `u32`.
/// The standard library has a type that makes "never zero" known to the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SensorId(pub u32);

impl SensorId {
    /// `None` for 0.
    pub fn new(raw: u32) -> Option<SensorId> {
        todo!("Implement this task")
    }

    pub fn get(self) -> u32 {
        todo!("Implement this task")
    }
}

/// B. One measurement.
///
/// Planned for later: a time stamp. A user who can write `Reading { .. }` with
/// all fields, or take a `Reading` apart with a pattern that names all fields,
/// stops compiling on the day a field is added. Users must build a reading with
/// `new` and read it with the two methods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub sensor: SensorId,
    pub celsius: f64,
}

impl Reading {
    pub fn new(sensor: SensorId, celsius: f64) -> Reading {
        todo!("Implement this task")
    }

    pub fn sensor(&self) -> SensorId {
        todo!("Implement this task")
    }

    pub fn celsius(&self) -> f64 {
        todo!("Implement this task")
    }
}

/// C. Why a sensor gave no reading.
///
/// Planned for later: more reasons, and more data in `OutOfRange`. So a user's
/// `match` must be forced to have a wildcard arm, and a user must not be able to
/// build an `OutOfRange` value or to name all of its fields in a pattern.
/// Use the `out_of_range` function below to build that variant inside the library.
#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    Timeout,
    OutOfRange { limit: f64 },
}

/// The fault for a value beyond `limit` degrees Celsius.
pub fn out_of_range(limit: f64) -> Fault {
    Fault::OutOfRange { limit }
}

/// A sentence for a fault: `no answer in time` for `Timeout`, and
/// `beyond 85 °C` for `OutOfRange` with the limit 85 (the limit is printed with
/// `{}`).
pub fn describe(fault: &Fault) -> String {
    todo!("Implement this task")
}

/// D. A temperature unit, used as a type parameter: `show::<Fahrenheit>(&reading)`.
///
/// Planned for later: more required items in this trait. That is a major change
/// for a trait that users can implement. So `Unit` must be sealed: users can
/// name it, use it as a bound and call it, but only this library implements it.
/// Seal it with a public supertrait that lives in a private module.
pub trait Unit {
    /// What is printed after the number.
    const SYMBOL: &'static str;

    /// Converts degrees Celsius into this unit.
    fn from_celsius(celsius: f64) -> f64;
}

/// Degrees Celsius.
#[derive(Debug, Clone, Copy)]
pub struct Celsius;

/// Degrees Fahrenheit: `celsius * 1.8 + 32`.
#[derive(Debug, Clone, Copy)]
pub struct Fahrenheit;

impl Unit for Celsius {
    const SYMBOL: &'static str = "°C";

    fn from_celsius(celsius: f64) -> f64 {
        todo!("Implement this task")
    }
}

impl Unit for Fahrenheit {
    const SYMBOL: &'static str = "°F";

    fn from_celsius(celsius: f64) -> f64 {
        todo!("Implement this task")
    }
}

/// The reading in unit `U` with one decimal place, a space, and the symbol:
/// `21.5 °C`.
pub fn show<U: Unit>(reading: &Reading) -> String {
    todo!("Implement this task")
}

/// E. The readings of one day, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Log {
    readings: Vec<Reading>,
}

/// The iterator that `Log::readings` returns. It yields `&Reading`.
///
/// Planned for later: the log may store its readings in another collection.
/// Today this name is an alias, so the signature of `readings` promises a
/// `std::slice::Iter`, and users can rely on everything that type can do.
/// Make `Readings` a type of this library that only promises `Iterator` (and
/// `Debug` and `Clone`, which cost nothing to keep).
pub type Readings<'a> = std::slice::Iter<'a, Reading>;

impl Log {
    pub fn new() -> Log {
        Log::default()
    }

    pub fn push(&mut self, reading: Reading) {
        self.readings.push(reading);
    }

    pub fn readings(&self) -> Readings<'_> {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support;

    /// Compiles one program of `fixtures/l03/cases/` against this file, built
    /// as the library crate `gauge`.
    #[track_caller]
    fn case(name: &str) {
        let gauge = support::library_from_file("gauge", "src/l03_future_proofing.rs");
        support::case(&format!("fixtures/l03/cases/{name}"), &[("gauge", &gauge)]);
    }

    fn id(raw: u32) -> SensorId {
        SensorId::new(raw).expect("the test uses a valid sensor number")
    }

    #[test]
    fn a_sensor_ids_are_never_zero() {
        assert_eq!(SensorId::new(0), None, "0 is not a valid sensor number");
        assert_eq!(id(7).get(), 7);
        assert_eq!(id(u32::MAX).get(), u32::MAX);
        assert!(id(2) < id(10), "sensor numbers are ordered as numbers");
    }

    #[test]
    fn a_an_optional_sensor_id_costs_nothing() {
        assert_eq!(
            std::mem::size_of::<Option<SensorId>>(),
            std::mem::size_of::<u32>(),
            "Option<SensorId> must be as small as a u32: store a type whose value 0 is \
             known to the compiler as impossible, so that `None` can use that bit pattern"
        );
    }

    #[test]
    fn a_users_cannot_build_or_open_a_sensor_id() {
        case("a_build_directly.rs");
        case("a_read_field.rs");
        case("a_rebuild.rs");
    }

    #[test]
    fn b_readings_are_built_and_read_through_methods() {
        let reading = Reading::new(id(3), 21.5);
        assert_eq!(reading.sensor(), id(3));
        assert_eq!(reading.celsius(), 21.5);
    }

    #[test]
    fn b_users_cannot_write_a_reading_as_a_struct_expression() {
        case("b_struct_expression.rs");
        case("b_full_pattern.rs");
    }

    #[test]
    fn b_users_cannot_read_the_fields_of_a_reading() {
        case("b_read_field.rs");
    }

    #[test]
    fn c_faults_are_described() {
        assert_eq!(describe(&Fault::Timeout), "no answer in time");
        assert_eq!(describe(&out_of_range(85.0)), "beyond 85 °C");
        assert_eq!(describe(&out_of_range(-40.5)), "beyond -40.5 °C");
    }

    #[test]
    fn c_users_must_expect_more_faults() {
        case("c_match_without_wildcard.rs");
        case("c_build_variant.rs");
        case("c_variant_pattern_without_rest.rs");
    }

    #[test]
    fn d_units_convert_and_print() {
        let reading = Reading::new(id(1), 21.46);
        assert_eq!(show::<Celsius>(&reading), "21.5 °C");
        assert_eq!(show::<Fahrenheit>(&reading), "70.6 °F");
        assert_eq!(show::<Fahrenheit>(&Reading::new(id(1), -40.0)), "-40.0 °F");
    }

    #[test]
    fn d_users_cannot_implement_unit() {
        case("d_implement_unit.rs");
    }

    /// A seal holds only as long as no public path leads to the supertrait.
    /// This test reads the supertraits of `Unit` from this file and tries to
    /// implement each of them for a type of another crate.
    #[test]
    fn d_users_cannot_name_the_supertrait_of_unit() {
        let source = support::read("src/l03_future_proofing.rs");
        // The text between `pub trait Unit` and the `{` of the trait: either
        // `: A + B` or `where Self: A + B`.
        let start = source
            .lines()
            .scan(0, |offset, line| {
                let at = *offset;
                *offset += line.len() + 1;
                Some((at, line))
            })
            .find(|(_, line)| line.trim_start().starts_with("pub trait Unit"))
            .map(|(at, line)| {
                at + line.find("pub trait Unit").unwrap_or(0) + "pub trait Unit".len()
            })
            .expect("this file still declares `pub trait Unit`");
        let header = &source[start.min(source.len())..];
        let header = &header[..header.find('{').unwrap_or(header.len())];
        let supertraits: Vec<&str> = header
            .trim()
            .trim_start_matches("where")
            .trim_start()
            .trim_start_matches("Self")
            .trim_start()
            .trim_start_matches(':')
            .split('+')
            .map(|path| {
                path.trim()
                    .trim_end_matches(',')
                    .trim_start_matches("self::")
            })
            .filter(|path| !path.is_empty())
            .collect();
        assert!(
            !supertraits.is_empty(),
            "`Unit` has no supertrait yet. Seal it: `pub trait Unit: sealed::Sealed {{`, with \
             the trait `Sealed` in a private module"
        );
        let gauge = support::library_from_file("gauge", "src/l03_future_proofing.rs");
        let build = support::Build {
            externs: &[("gauge", &gauge)],
            ..support::Build::default()
        };
        // Each supertrait is tried under the path that this file uses, and
        // under its bare name, in case it is re-exported at the top.
        let mut attempts = Vec::new();
        let mut private = false;
        for path in &supertraits {
            let name = path.rsplit("::").next().unwrap_or(path);
            for candidate in [*path, name] {
                let program = format!(
                    "struct Kelvin;\n\nimpl gauge::{candidate} for Kelvin {{}}\n\nfn main() {{}}\n"
                );
                let run = support::program_from_text("l03-open-the-seal", &program, &build);
                assert!(
                    run.outcome != "ok",
                    "`Unit` is not sealed: a user can write `impl gauge::{candidate} for Kelvin {{}}` \
                     and then implement `Unit`. No public path may lead to the supertrait: \
                     its module must be private (`mod sealed`, not `pub mod sealed`), and the \
                     trait must not be re-exported. `#[doc(hidden)]` hides a path from the \
                     documentation, not from the compiler."
                );
                // E0603 is the compiler's "is private".
                private |= candidate == *path && run.outcome.split(',').any(|code| code == "E0603");
                attempts.push(format!(
                    "`impl gauge::{candidate} for Kelvin {{}}` gives `{}`",
                    run.outcome
                ));
            }
        }
        assert!(
            private,
            "`Unit` is not sealed by privacy: the compiler must answer E0603 (\"is private\") \
             when a user names the supertrait, but {}. Put the supertrait into a private \
             module (`mod sealed`, not `pub mod sealed`).",
            attempts.join(", and ")
        );
    }

    #[test]
    fn e_the_log_yields_its_readings_oldest_first() {
        let mut log = Log::new();
        assert_eq!(log.readings().count(), 0);
        log.push(Reading::new(id(1), 20.0));
        log.push(Reading::new(id(2), 30.0));
        let sensors: Vec<u32> = log
            .readings()
            .map(|reading| reading.sensor().get())
            .collect();
        assert_eq!(sensors, [1, 2]);
        let mut twice = log.readings();
        twice.next();
        assert_eq!(
            log.readings().count(),
            2,
            "every call starts a new iteration"
        );
    }

    #[test]
    fn e_users_cannot_name_the_iterator_of_the_standard_library() {
        case("e_name_std_iterator.rs");
    }

    /// The other side of the contract: everything a user is meant to do still
    /// compiles and runs.
    #[test]
    fn u_the_supported_uses_work() {
        case("use_api.rs");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        use std::mem::size_of;
        use std::num::NonZeroU32;

        // P1. A newtype is a struct with one field. How many bytes does the
        // wrapper add?
        struct Meters(#[allow(dead_code)] f64);
        let p1: usize = crate::guess();
        assert_eq!(
            p1,
            size_of::<Meters>() - size_of::<f64>(),
            "P1: a newtype has the size of its field"
        );

        // P2. `Option` needs to remember whether a value is present.
        let p2_plain: usize = crate::guess();
        let p2_nonzero: usize = crate::guess();
        assert_eq!(
            p2_plain,
            size_of::<Option<u32>>(),
            "P2: every bit pattern of a u32 is a valid number, so `None` needs extra room"
        );
        assert_eq!(
            p2_nonzero,
            size_of::<Option<NonZeroU32>>(),
            "P2: 0 is never a valid NonZeroU32, so `None` is stored as 0"
        );

        // P3. Marker types such as `Celsius` have no fields.
        struct Marker;
        let p3: usize = crate::guess();
        assert_eq!(
            p3,
            size_of::<[Marker; 1000]>(),
            "P3: a type without fields has size 0, and so has an array of them"
        );
    }
}
