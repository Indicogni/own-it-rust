//@ expect: run
//@ why: a text without mistakes gives its quotas in order; comments, empty lines and extra white space are ignored

fn main() {
    let text = "# limits for the free plan\n\nuploads 20\n   downloads \t 500  \n";
    let quotas = quota::parse(text).expect("the text has no mistakes");
    let pairs: Vec<(&str, u32)> = quotas
        .iter()
        .map(|quota| (quota.name(), quota.limit()))
        .collect();
    assert_eq!(pairs, [("uploads", 20), ("downloads", 500)]);

    assert_eq!(quota::parse("").expect("an empty text is fine").len(), 0);
    assert_eq!(
        quota::parse("zero 0\nmax 4294967295").expect("both ends of u32 are limits")[1].limit(),
        u32::MAX
    );

    // `Quota` is an ordinary value: it can be printed, cloned and compared.
    let first = quotas[0].clone();
    assert_eq!(first, quotas[0]);
    assert!(format!("{first:?}").contains("uploads"));
}
