//@ expect: E0423 | E0603
//@ why: code outside the module must not be able to write an AdminToken value itself; a unit struct `pub struct AdminToken;` can be forged by anyone.
use m1_types::l06_zst_never::{AdminToken, wipe};

fn main() {
    let forged = AdminToken;
    let mut data = vec![1, 2, 3];
    wipe(&forged, &mut data);
}
