//@ expect: run
//@ why: Codec must be dyn compatible, with `name` and `encode_into` callable on a trait object, and `twin` and `extension` still usable on concrete types.
use m1_types::l05_trait_objects::{Caesar, Codec, Upper};
use std::io::Write;

fn main() -> std::io::Result<()> {
    let codecs: Vec<Box<dyn Codec>> = vec![Box::new(Upper), Box::new(Caesar { shift: 13 })];

    let mut out: Vec<u8> = Vec::new();
    let mut names = Vec::new();
    for codec in &codecs {
        names.push(codec.name());
        codec.encode_into("Hello", &mut out)?;
        out.write_all(b"|")?;
    }
    assert_eq!(names, ["upper", "caesar-13"]);
    assert_eq!(out, b"HELLO|Uryyb|");

    // The methods that need a concrete `Self` are still there.
    assert_eq!(Caesar { shift: 3 }.twin().name(), "caesar-3");
    assert_eq!(<Upper as Codec>::extension(), "up");

    // A trait object can write to a trait object.
    let sink: &mut dyn Write = &mut out;
    codecs[0].encode_into("x", sink)?;
    assert_eq!(out.last(), Some(&b'X'));
    Ok(())
}
