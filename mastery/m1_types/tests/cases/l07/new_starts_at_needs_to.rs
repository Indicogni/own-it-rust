//@ expect: E0599
//@ why: `new` must create an envelope in stage NeedsTo and in no other stage.
use m1_types::l07_sealed_typestate::{Envelope, Ready};

fn main() {
    let shortcut = Envelope::<Ready>::new("ada");
    println!("{}", shortcut.send());
}
