//@ expect: run
//@ why: a Legend must accept labels that borrow local data; Box<dyn Label> without a lifetime demands 'static.
use m1_types::l05_trait_objects::{Caption, Legend};

fn main() {
    let city = String::from("Lyon");
    let river = String::from("Rhone");
    let mut legend = Legend::new();
    legend.add(Caption(&city));
    legend.add(String::from("owned"));
    legend.add(Caption(&river));
    assert_eq!(legend.render(), "[Lyon] | owned | [Rhone]");
}
