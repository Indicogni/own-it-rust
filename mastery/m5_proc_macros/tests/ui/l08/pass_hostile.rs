//! Lesson 08: the builder in a module that reuses every familiar name.
//! This program must compile and run.

#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]

mod hostile {
    // Types and traits with the names of the standard ones.
    pub struct Option;
    pub struct Some;
    pub struct None;
    pub struct Result;
    pub struct Ok;
    pub struct Err;
    pub struct Vec;
    pub struct String;
    pub struct BuildError;
    pub trait Default {}
    pub trait Into {}

    // Unit structs and constants with the names of likely local variables.
    pub struct value;
    pub struct missing;
    pub struct builder;
    pub struct this;
    pub const required: u8 = 0;
    pub const field: u8 = 0;
    pub const error: u8 = 0;
    pub const result: u8 = 0;

    // The user's own field types are written with full paths, because in
    // this module the short names mean something else.
    #[derive(m5_proc_macros::Builder)]
    pub struct Job {
        pub id: ::core::primitive::u32,
        pub note: ::core::option::Option<::std::string::String>,
        #[builder(default)]
        pub retries: ::core::primitive::u8,
        #[builder(default = ::std::string::String::from("main"), into)]
        pub branch: ::std::string::String,
        // Fields with the names of the generated functions' helpers.
        pub value: ::core::primitive::u8,
        pub missing: ::core::primitive::u8,
    }
}

fn main() {
    let job = match hostile::Job::builder().id(3).value(1).missing(2).build() {
        Ok(job) => job,
        Err(error) => panic!("{error}"),
    };
    assert_eq!((job.id, job.retries, job.value, job.missing), (3, 0, 1, 2));
    assert_eq!(job.note, None);
    assert_eq!(job.branch, "main");

    let error = match hostile::Job::builder().note("n".to_owned()).build() {
        Ok(_) => panic!("three fields are missing"),
        Err(error) => error,
    };
    assert_eq!(error.missing(), ["id", "value", "missing"]);
}
