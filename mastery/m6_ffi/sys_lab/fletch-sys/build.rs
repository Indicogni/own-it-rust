//! Build script of `fletch-sys` (lesson 02 lab).
//!
//! TODO 2: write this script. It must
//!
//! 1. compile `vendor/fletch/src/fletch.c` into a static library named
//!    `fletch` with the `cc` crate. The source includes "fletch.h", which
//!    lives in `vendor/fletch/include`;
//! 2. define the C macro `FLETCH_WIDE` when, and only when, the Cargo
//!    feature `wide` of this package is enabled;
//! 3. tell Cargo to run the script again when the C source, the header or
//!    this file changes;
//! 4. publish the absolute path of `vendor/fletch/include` to dependent
//!    packages under the metadata key `include`.

fn main() {
    todo!("lesson 02, TODO 2: write the build script of fletch-sys")
}
