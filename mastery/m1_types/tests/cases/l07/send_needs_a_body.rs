//@ expect: E0599
//@ why: `send` must exist in stage Ready only; an envelope without a body must be a compile error, not a run-time check.
use m1_types::l07_sealed_typestate::Envelope;

fn main() {
    let addressed = Envelope::new("ada").to("grace");
    println!("{}", addressed.send());
}
