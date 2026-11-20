//@ expect: E0382
//@ why: a transition must take the envelope by value, so that the old stage cannot be used again.
use m1_types::l07_sealed_typestate::Envelope;

fn main() {
    let draft = Envelope::new("ada");
    let to_grace = draft.to("grace");
    let to_alan = draft.to("alan");
    println!("{} {}", to_grace.stage(), to_alan.stage());
}
