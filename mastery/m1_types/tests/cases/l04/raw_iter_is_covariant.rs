//@ expect: run
//@ why: RawIter<'a, T> must be covariant in 'a and in T, like &'a [T]; a marker such as PhantomData<&'a mut T> or PhantomData<Cell<T>> makes it invariant in T.
use m1_types::l04_phantom::RawIter;

fn shorten_lifetime<'long: 'short, 'short, T>(iter: RawIter<'long, T>) -> RawIter<'short, T> {
    iter
}

fn shorten_items<'long: 'short, 'short, 'i>(
    iter: RawIter<'i, &'long str>,
) -> RawIter<'i, &'short str> {
    iter
}

fn main() {
    static WORDS: [&str; 2] = ["static", "words"];
    let iter: RawIter<'static, &'static str> = RawIter::new(&WORDS);
    let mut iter = shorten_items(shorten_lifetime(iter));
    assert_eq!(iter.next(), Some(&"static"));
    assert_eq!(iter.as_slice(), ["words"]);
}
