//! Chapter 23 exercises: implement the `todo!()` bodies and answer the
//! predictions. Start with README.md in this directory.
// These lints are allowed because unfinished `todo!()` bodies leave arguments
// and imports unused. `guess` is no longer used by a lesson once you have
// answered all of its predictions.
#![allow(
    unused_variables,
    unused_mut,
    unused_imports,
    dead_code,
    unreachable_code,
    clippy::diverging_sub_expression
)]
mod c01_code_review;
mod l01_ascii;
mod l02_pointers;
mod l03_raw_slices;
mod l04_tagged;
mod l05_representations;
mod l06_moves;
mod l07_gap_buffer;
mod l08_capstone;
mod l09_cells_layout;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;

/// A test helper for the technique tests. You do not need to edit it.
///
/// Behaviour tests cannot see whether a task was solved with the raw-pointer
/// technique that its contract asks for: a safe standard-library call often
/// gives the same result. So a few tests read the text of your function and
/// look for the operation the contract names. This is a plain text search. It
/// is a reminder, not a proof: only your SAFETY argument (and Miri, see
/// LABS.md) says anything about soundness.
mod source {
    /// Returns the part of a lesson file that you write: everything before
    /// the test module, with `//` comments removed.
    pub fn code_of(file: &str) -> String {
        let end = file.find("#[cfg(test)]").unwrap_or(file.len());
        file[..end]
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns the text of one item of a lesson file, from `item` (for example
    /// `"fn as_str"`) to its closing brace, taken from [`code_of`].
    pub fn body_of(file: &str, item: &str) -> String {
        let code = code_of(file);
        let start = code.find(item).unwrap_or_else(|| {
            panic!("`{item}` was not found in the lesson file: keep the supplied names")
        });
        let mut depth = 0_usize;
        for (offset, c) in code[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' if depth > 0 => {
                    depth -= 1;
                    if depth == 0 {
                        return code[start..=start + offset].to_string();
                    }
                }
                _ => {}
            }
        }
        code[start..].to_string()
    }
}
