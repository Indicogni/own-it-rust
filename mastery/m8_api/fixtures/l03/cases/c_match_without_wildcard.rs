//@ expect: E0004
//@ why: (C) a `match` that lists every variant stops compiling when a variant is added; `#[non_exhaustive]` on the enum makes users write a wildcard arm today

use gauge::Fault;

fn main() {
    let fault = Fault::Timeout;
    let text = match fault {
        Fault::Timeout => "timeout",
        Fault::OutOfRange { .. } => "out of range",
    };
    println!("{text}");
}
