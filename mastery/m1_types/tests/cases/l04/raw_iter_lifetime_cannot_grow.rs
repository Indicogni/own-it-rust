//@ expect: error
//@ why: the lifetime of a RawIter must never grow; a contravariant marker such as PhantomData<fn(&'a T)> would let it yield references that outlive the slice.
use m1_types::l04_phantom::RawIter;

fn lengthen<'short>(iter: RawIter<'short, u8>) -> RawIter<'static, u8> {
    iter
}

fn main() {
    let data = vec![1_u8, 2, 3];
    let forever: Option<&'static u8> = lengthen(RawIter::new(&data)).next();
    drop(data);
    println!("{forever:?}");
}
