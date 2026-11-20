//! Lesson 08: the builder in ordinary use.
//! This program must compile and run.

#![deny(missing_docs)]
//! (The line above makes the compiler reject public items without a doc
//! comment. The generated builder is public, so it needs doc comments too.)

use m5_proc_macros::{BuildError, Builder};

/// A type without `Clone`, `Default` and `Debug`.
pub struct Handle(pub u32);

/// A unit of work.
#[derive(Builder)]
pub struct Job {
    /// Required.
    pub id: u32,
    /// Required.
    pub name: String,
    /// Optional: an Option field.
    pub note: Option<String>,
    /// Optional, written with a longer path.
    pub limit: std::option::Option<u8>,
    /// Required, and not `Clone`.
    pub handle: Handle,
    /// A raw identifier.
    pub r#type: u8,
}

/// Fields with attributes.
#[derive(Builder)]
pub struct Options {
    /// `Default::default()` when not set.
    #[builder(default)]
    pub retries: u8,
    /// 30 when not set.
    #[builder(default = 30)]
    pub timeout: u64,
    /// The setter converts.
    #[builder(into)]
    pub owner: String,
    /// Both, in two attributes, with another attribute between them.
    #[builder(into)]
    #[allow(dead_code)]
    #[builder(default = String::from("main"))]
    pub branch: String,
    /// An Option field with a default of its own.
    #[builder(default = Some(8))]
    pub jobs: Option<u8>,
    /// An Option field with `into`: the setter takes `impl Into<String>`.
    #[builder(into)]
    pub tag: Option<String>,
}

/// No field at all.
#[derive(Builder)]
pub struct Nothing {}

fn main() {
    // Every required field set, in any order, optional ones left out.
    let job = Job::builder()
        .name("build".to_owned())
        .handle(Handle(9))
        .r#type(2)
        .id(7)
        .build()
        .ok()
        .expect("all required fields are set");
    assert_eq!((job.id, job.name.as_str(), job.r#type), (7, "build", 2));
    assert_eq!(job.note, None);
    assert_eq!(job.limit, None);
    assert_eq!(job.handle.0, 9);

    // The setter of an Option field takes the inner type. The last call wins.
    let job = Job::builder()
        .id(1)
        .id(2)
        .name(String::new())
        .note("urgent".to_owned())
        .limit(3)
        .handle(Handle(0))
        .r#type(0)
        .build()
        .ok()
        .expect("all required fields are set");
    assert_eq!(job.id, 2);
    assert_eq!(job.note.as_deref(), Some("urgent"));
    assert_eq!(job.limit, Some(3));

    // Missing fields: all of them, in declaration order, without `r#`.
    let error: BuildError = match Job::builder()
        .name("x".to_owned())
        .note("n".to_owned())
        .build()
    {
        Ok(_) => panic!("three required fields are missing"),
        Err(error) => error,
    };
    assert_eq!(error.type_name(), "Job");
    assert_eq!(error.missing(), ["id", "handle", "type"]);
    assert_eq!(
        error.to_string(),
        "cannot build `Job`: missing `id`, `handle`, `type`"
    );

    // Defaults and conversions.
    let options = match Options::builder().owner("ann").build() {
        Ok(options) => options,
        Err(error) => panic!("only `owner` is required: {error}"),
    };
    assert_eq!(options.retries, 0);
    assert_eq!(options.timeout, 30);
    assert_eq!(options.owner, "ann");
    assert_eq!(options.branch, "main");
    assert_eq!(options.jobs, Some(8));
    assert_eq!(options.tag, None);

    let options = match Options::builder()
        .retries(5)
        .timeout(1)
        .owner(String::from("bob"))
        .branch("dev")
        .jobs(2)
        .tag("v1")
        .build()
    {
        Ok(options) => options,
        Err(error) => panic!("only `owner` is required: {error}"),
    };
    assert_eq!((options.retries, options.timeout), (5, 1));
    assert_eq!(
        (options.owner.as_str(), options.branch.as_str()),
        ("bob", "dev")
    );
    assert_eq!(options.jobs, Some(2));
    assert_eq!(options.tag.as_deref(), Some("v1"));

    let error = match Options::builder().build() {
        Ok(_) => panic!("`owner` is missing"),
        Err(error) => error,
    };
    assert_eq!(error.missing(), ["owner"]);

    assert!(Nothing::builder().build().is_ok());
}
