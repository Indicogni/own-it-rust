//@ expect: run
//@ why: Id<T> must be covariant in T, as a plain value of type T would be: an Id<&'static str> must be usable where an Id<&'a str> is expected.
use m1_types::l04_phantom::Id;

fn same_kind<'a>(a: Id<&'a str>, b: Id<&'a str>) -> bool {
    a == b
}

fn shorten<'long: 'short, 'short>(id: Id<&'long str>) -> Id<&'short str> {
    id
}

fn main() {
    let of_static: Id<&'static str> = Id::new(1);
    let local = String::from("local");
    let of_local: Id<&str> = id_for(&local);
    assert!(!same_kind(of_static, of_local));
    assert_eq!(shorten(of_static).raw(), 1);
}

fn id_for<'a>(_text: &'a str) -> Id<&'a str> {
    Id::new(2)
}
