//@ expect: E0597
//@ why: a RawIter holds raw pointers into the slice, so the borrow checker must keep the slice alive for as long as the iterator or its items are used.
use m1_types::l04_phantom::RawIter;

fn main() {
    let item;
    {
        let data = vec![1_u8, 2, 3];
        item = RawIter::new(&data).next();
    }
    println!("{item:?}");
}
