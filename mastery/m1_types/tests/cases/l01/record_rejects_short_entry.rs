//@ expect: E0597
//@ why: a log of long-lived entries must still refuse an entry that dies before the log does.
use m1_types::l01_variance::record;

fn main() {
    let mut log: Vec<&str> = Vec::new();
    {
        let short_lived = String::from("gone at the end of this block");
        record(&mut log, &short_lived);
    }
    println!("{log:?}");
}
