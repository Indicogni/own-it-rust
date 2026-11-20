#![allow(warnings)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args[1] == "report" {
        if args[2] == "--json" {
            let text = std::fs::read_to_string(&args[3]).unwrap();
            println!("{}", tally_core::to_json(&tally_core::parse(&text)));
        } else {
            let text = std::fs::read_to_string(&args[2]).unwrap();
            print!("{}", tally_core::report(&tally_core::parse(&text)));
        }
    }
}
