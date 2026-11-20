//! Reports which of its own features this copy of the crate was compiled with.

/// The enabled features, in a fixed order, joined with `+`. `none` if there are none.
pub fn features() -> String {
    let mut enabled = Vec::new();
    if cfg!(feature = "codegen") {
        enabled.push("codegen");
    }
    if cfg!(feature = "fast") {
        enabled.push("fast");
    }
    if cfg!(feature = "safe") {
        enabled.push("safe");
    }
    if enabled.is_empty() {
        "none".to_owned()
    } else {
        enabled.join("+")
    }
}
