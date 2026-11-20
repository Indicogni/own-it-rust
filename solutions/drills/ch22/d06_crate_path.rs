//! DRILL 22-06 · Exported macros and helper paths
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0425
//!
//! Run `cargo drill ch22`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where is `label` resolved after expansion in main?

// FIX: the expansion lands in `main`, where a bare `label` is not in scope.
// `$crate::util::label` names the helper from any call site, and the helper
// must be `pub` (or `pub(crate)`) to be reachable from there.
mod util {
    pub fn label(n: u32) -> String {
        format!("#{n}")
    }

    #[macro_export]
    macro_rules! tag {
        ($n:expr) => {
            $crate::util::label($n)
        };
    }
}

fn main() {
    assert_eq!(tag!(7), "#7"); // KEEP
}
