//@ expect: E0599 | E0277
//@ why: `cc` must exist only in stages that implement HasRecipient; a new envelope has no recipient yet.
use m1_types::l07_sealed_typestate::Envelope;

fn main() {
    let draft = Envelope::new("ada").cc("alan");
    println!("{}", draft.stage());
}
