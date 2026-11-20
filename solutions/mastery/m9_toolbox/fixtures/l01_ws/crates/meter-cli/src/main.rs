//! `meter-cli 1500ms` prints the duration in milliseconds.
//! `meter-cli --json 2s` prints it as JSON. `meter-cli --caps` prints what was
//! compiled in.

fn main() {
    #[cfg(windows)]
    winconsole::enable_ansi();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["--caps"] => {
            println!("json: {}", if json_support() { "on" } else { "off" });
            println!(
                "fast-path: {}",
                if meter::has_fast_path() { "on" } else { "off" }
            );
        }
        ["--json", text] => match meter::parse_millis(text) {
            Some(millis) => println!("{}", json(millis)),
            None => fail(text),
        },
        [text] => match meter::parse_millis(text) {
            Some(millis) => println!("{millis} ms"),
            None => fail(text),
        },
        _ => {
            eprintln!("usage: meter-cli [--json] <duration> | --caps");
            std::process::exit(2);
        }
    }
}

fn fail(text: &str) -> ! {
    eprintln!("not a duration: {text}");
    std::process::exit(1);
}

fn json_support() -> bool {
    cfg!(feature = "json")
}

#[cfg(feature = "json")]
fn json(millis: u64) -> String {
    meter::to_json(millis)
}

#[cfg(not(feature = "json"))]
fn json(_millis: u64) -> String {
    eprintln!("this build has no JSON support");
    std::process::exit(1);
}
