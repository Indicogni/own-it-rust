//@ expect: E0308
//@ why: the return type of `names` must not show how Pipeline stores its stages; a caller that spells out the iterator type must not compile.
use m1_types::c01_code_review::Pipeline;

type Entry = (String, Box<dyn Fn(&str) -> String>);

fn main() {
    let pipeline = Pipeline::new().stage("trim", |text| text.trim().to_string());
    // This caller depends on the exact iterator type. If it compiles, every
    // change to the fields of Pipeline breaks it.
    let names: std::iter::Map<std::slice::Iter<'_, Entry>, fn(&Entry) -> &String> =
        pipeline.names();
    println!("{}", names.count());
}
