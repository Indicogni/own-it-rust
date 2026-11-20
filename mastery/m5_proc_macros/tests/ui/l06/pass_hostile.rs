//! Lesson 06: `flags!` in a module that has its own ideas about names.
//! This program must compile and run.

#![allow(dead_code)]

mod hostile {
    // None of these may change what the generated code means.
    pub struct Vec;
    pub struct Clone;
    pub struct Copy;
    pub struct Debug;
    pub struct PartialEq;
    pub struct Eq;
    #[allow(non_camel_case_types)]
    pub struct bool;
    #[allow(non_camel_case_types)]
    pub struct u16;

    m5_proc_macros::flags! {
        pub struct Mode: u16 {
            FAST,
            SAFE,
        }
    }
}

fn main() {
    use hostile::Mode;
    let both = Mode::FAST.with(Mode::SAFE);
    assert!(both.contains(Mode::SAFE));
    assert_eq!(both.names(), ["FAST", "SAFE"]);
    assert_eq!(format!("{:?}", both.clone()), "Mode(3)");
}
